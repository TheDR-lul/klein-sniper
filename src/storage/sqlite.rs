use crate::model::{ModelStats, Offer, StorageError};
use chrono::{DateTime, Duration, Utc, NaiveDateTime, TimeZone};
use rusqlite::{params, Connection, Row};
use std::collections::HashMap;
use tracing::info;

pub struct SqliteStorage {
    conn: Connection,
}

impl SqliteStorage {
    /// Create new storage, open DB connection and run migrations
    pub fn new(db_path: &str) -> Result<Self, StorageError> {
        let conn = Connection::open(db_path)?;
        
        // Enable WAL mode for better concurrent access
        conn.execute("PRAGMA journal_mode=WAL", [])?;
        conn.execute("PRAGMA synchronous=NORMAL", [])?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS offers (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                price REAL NOT NULL,
                model TEXT NOT NULL,
                link TEXT NOT NULL,
                posted_at TEXT NOT NULL,
                fetched_at TEXT NOT NULL,
                location TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS notified (
                offer_id TEXT PRIMARY KEY,
                notified_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS model_stats (
                model TEXT PRIMARY KEY,
                avg_price REAL NOT NULL,
                std_dev REAL NOT NULL,
                last_updated TEXT NOT NULL
            );
            
            -- Create indexes for query optimization
            CREATE INDEX IF NOT EXISTS idx_offers_model ON offers(model);
            CREATE INDEX IF NOT EXISTS idx_offers_price ON offers(price);
            CREATE INDEX IF NOT EXISTS idx_offers_fetched_at ON offers(fetched_at);
            CREATE INDEX IF NOT EXISTS idx_offers_model_price ON offers(model, price);
            CREATE INDEX IF NOT EXISTS idx_notified_at ON notified(notified_at);
            "
        )?;

        // Auto-migrations for offers table: ensure all required columns exist
        Self::migrate_add_column_if_missing(&conn, "offers", "location", "TEXT NOT NULL DEFAULT ''")?;
        Self::migrate_add_column_if_missing(&conn, "offers", "description", "TEXT NOT NULL DEFAULT ''")?;
        // Add user fields used in save_offer and queries
        Self::migrate_add_column_if_missing(&conn, "offers", "user_id", "TEXT")?;
        Self::migrate_add_column_if_missing(&conn, "offers", "user_name", "TEXT")?;
        Self::migrate_add_column_if_missing(&conn, "offers", "user_url", "TEXT")?;

        Ok(Self { conn })
    }

    /// Check if column exists and add it to table if missing
    fn migrate_add_column_if_missing(
        conn: &Connection,
        table: &str,
        column: &str,
        column_def: &str,
    ) -> Result<(), StorageError> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
        let existing_columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<_, _>>()?;

        if !existing_columns.iter().any(|c| c == column) {
            let alter_sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, column_def);
            conn.execute(&alter_sql, [])?;
        }

        Ok(())
    }

    /// Save (insert or update) offer in offers table
    pub fn save_offer(&self, offer: &Offer) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO offers (
                id, title, price, model, link, 
                posted_at, fetched_at, location, description,
                user_id, user_name, user_url
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                &offer.id,
                &offer.title,
                &offer.price,
                &offer.model,
                &offer.link,
                &offer.posted_at.to_rfc3339(),
                &offer.fetched_at.to_rfc3339(),
                &offer.location,
                &offer.description,
                &offer.user_id,
                &offer.user_name,
                &offer.user_url,
            ],
        )?;
        Ok(())
    }

    /// Group offers by seller ID for specified model
    pub fn group_offers_by_seller(&self, model: &str) -> Result<HashMap<String, usize>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT user_id, COUNT(*) FROM offers WHERE model = ?1 AND user_id IS NOT NULL GROUP BY user_id",
        )?;

        let rows = stmt.query_map(params![model], |row| {
            let user_id: String = row.get(0)?;
            let count: usize = row.get(1)?;
            Ok((user_id, count))
        })?;

        let mut result = HashMap::new();
        for row in rows {
            let (user_id, count) = row?;
            result.insert(user_id, count);
        }

        Ok(result)
    }

    /// Find probable reposts for specified model based on price proximity (< 10.0)
    pub fn find_probable_reposts_for_model(&self, model: &str) -> Result<Vec<Offer>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, price, model, link, posted_at, fetched_at, location, description, user_id, user_name, user_url 
             FROM offers WHERE model = ?1 AND user_id IS NOT NULL ORDER BY fetched_at DESC",
        )?;

        let rows = stmt.query_map(params![model], |row| Self::map_offer(row, true))?;

        let mut seen = HashMap::<(String, String), f64>::new();
        let mut reposts = Vec::new();

        for offer in rows {
            let offer = offer?;
            let key = (offer.title.clone(), offer.user_id.clone().unwrap_or_default());
            if let Some(prev_price) = seen.get(&key) {
                if (offer.price - prev_price).abs() < 10.0 {
                    reposts.push(offer);
                }
            } else {
                seen.insert(key, offer.price);
            }
        }

        Ok(reposts)
    }

    /// Delete offers for specified model whose IDs are not in current list
    pub fn delete_missing_offers_for_model(&self, model: &str, current_ids: &[String]) -> Result<(), StorageError> {
        if current_ids.is_empty() {
            self.conn.execute("DELETE FROM offers WHERE model = ?1", params![model])?;
            return Ok(());
        }

        let placeholders = current_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "DELETE FROM offers WHERE model = ?1 AND id NOT IN ({})",
            placeholders
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut params_vec = vec![model.to_string()];
        params_vec.extend(current_ids.iter().cloned());
        stmt.execute(rusqlite::params_from_iter(params_vec))?;
        Ok(())
    }

    /// Check if offer was already notified
    pub fn is_notified(&self, offer_id: &str) -> Result<bool, StorageError> {
        let mut stmt = self.conn.prepare("SELECT 1 FROM notified WHERE offer_id = ?1")?;
        let mut rows = stmt.query(params![offer_id])?;
        Ok(rows.next()?.is_some())
    }

    /// Return true if notification is missing or more than 24 hours passed since last notification
    pub fn should_notify(&self, offer_id: &str) -> Result<bool, StorageError> {
        let mut stmt = self.conn.prepare("SELECT notified_at FROM notified WHERE offer_id = ?1")?;
        let mut rows = stmt.query(params![offer_id])?;

        if let Some(row) = rows.next()? {
            let notified_at_str: String = row.get(0)?;
            if notified_at_str.trim().is_empty() {
                return Ok(true);
            }

            // Expected format returned by datetime('now') - "%Y-%m-%d %H:%M:%S"
            let notified_at_naive = NaiveDateTime::parse_from_str(&notified_at_str, "%Y-%m-%d %H:%M:%S")
                .map_err(|e| StorageError::DatabaseError(format!("Invalid datetime: {}", e)))?;
            let notified_at: DateTime<Utc> = Utc.from_utc_datetime(&notified_at_naive);

            Ok(Utc::now().signed_duration_since(notified_at) > Duration::hours(24))
        } else {
            Ok(true)
        }
    }

    /// Mark that notification for specified offer was sent (with current timestamp)
    pub fn mark_notified(&self, offer_id: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO notified (offer_id, notified_at) VALUES (?1, datetime('now'))",
            params![offer_id],
        )?;
        Ok(())
    }

    /// Get statistics for specified model if it exists
    pub fn get_stats(&self, model: &str) -> Result<Option<ModelStats>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT avg_price, std_dev, last_updated FROM model_stats WHERE model = ?1",
        )?;

        let mut rows = stmt.query(params![model])?;
        if let Some(row) = rows.next()? {
            let avg_price: f64 = row.get(0)?;
            let std_dev: f64 = row.get(1)?;
            let last_updated_str: String = row.get(2)?;
            let last_updated: DateTime<Utc> = last_updated_str.parse()?;

            Ok(Some(ModelStats {
                model: model.to_string(),
                avg_price,
                std_dev,
                last_updated,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update statistics for model
    pub fn update_stats(&self, stats: &ModelStats) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO model_stats (model, avg_price, std_dev, last_updated)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &stats.model,
                &stats.avg_price,
                &stats.std_dev,
                &stats.last_updated.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Return last offer by time
    pub fn get_last_offer(&self) -> Result<Option<Offer>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, price, model, link, posted_at, fetched_at, location, description,
                    user_id, user_name, user_url
             FROM offers ORDER BY fetched_at DESC LIMIT 1",
        )?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let offer = Self::map_offer(row, true)?;
            Ok(Some(offer))
        } else {
            Ok(None)
        }
    }

    /// Get 5 offers with minimum positive price
    pub fn get_top5_offers(&self) -> Result<Vec<Offer>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, price, model, link, posted_at, fetched_at, location, description,
                    user_id, user_name, user_url
             FROM offers WHERE price > 0 ORDER BY price ASC LIMIT 5",
        )?;

        let rows = stmt.query_map([], |row| Self::map_offer(row, true))?;
        let mut offers = Vec::new();
        for offer in rows {
            offers.push(offer?);
        }

        Ok(offers)
    }

    /// Get all offers
    pub fn get_all_offers(&self) -> Result<Vec<Offer>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, price, model, link, posted_at, fetched_at, location, description,
                    user_id, user_name, user_url
             FROM offers",
        )?;

        let rows = stmt.query_map([], |row| Self::map_offer(row, true))?;
        let mut offers = Vec::new();
        for offer in rows {
            offers.push(offer?);
        }

        Ok(offers)
    }

    /// Return list of (model, average price) for statistics
    pub fn get_average_prices(&self) -> Result<Vec<(String, f64)>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT model, avg_price FROM model_stats ORDER BY model ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let model: String = row.get(0)?;
            let avg_price: f64 = row.get(1)?;
            Ok((model, avg_price))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Private function to map result row to Offer struct
    /// If `full` parameter is true, expects user_id, user_name and user_url fields to be present in row
    fn map_offer(row: &Row, full: bool) -> Result<Offer, rusqlite::Error> {
        let posted_at_str: String = row.get(5)?;
        let fetched_at_str: String = row.get(6)?;
        let posted_at = posted_at_str.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
        })?;
        let fetched_at = fetched_at_str.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e))
        })?;

        let (user_id, user_name, user_url) = if full {
            (row.get(9)?, row.get(10)?, row.get(11)?)
        } else {
            (None, None, None)
        };

        Ok(Offer {
            id: row.get(0)?,
            title: row.get(1)?,
            price: row.get(2)?,
            model: row.get(3)?,
            link: row.get(4)?,
            posted_at,
            fetched_at,
            location: row.get(7)?,
            description: row.get(8)?,
            user_id,
            user_name,
            user_url,
        })
    }
    
    /// Clean up old offers based on retention period
    pub fn cleanup_old_offers(&self, retention_days: u32) -> Result<usize, StorageError> {
        if retention_days == 0 {
            info!("Offer retention is disabled (0 days), skipping cleanup");
            return Ok(0);
        }
        
        let cutoff_date = Utc::now() - Duration::days(retention_days as i64);
        let cutoff_str = cutoff_date.to_rfc3339();
        
        info!("Cleaning up offers older than {} days (before {})", retention_days, cutoff_str);
        
        let deleted = self.conn.execute(
            "DELETE FROM offers WHERE fetched_at < ?1",
            params![cutoff_str],
        )?;
        
        info!("Deleted {} old offers", deleted);
        Ok(deleted)
    }
    
    /// Clean up old stats based on retention period
    pub fn cleanup_old_stats(&self, retention_days: u32) -> Result<usize, StorageError> {
        if retention_days == 0 {
            info!("Stats retention is disabled (0 days), skipping cleanup");
            return Ok(0);
        }
        
        let cutoff_date = Utc::now() - Duration::days(retention_days as i64);
        let cutoff_str = cutoff_date.to_rfc3339();
        
        info!("Cleaning up stats older than {} days (before {})", retention_days, cutoff_str);
        
        let deleted = self.conn.execute(
            "DELETE FROM model_stats WHERE last_updated < ?1",
            params![cutoff_str],
        )?;
        
        info!("Deleted {} old stats records", deleted);
        Ok(deleted)
    }
    
    /// Clean up orphaned notification records (offers no longer exist)
    pub fn cleanup_orphaned_notifications(&self) -> Result<usize, StorageError> {
        info!("Cleaning up orphaned notification records");
        
        let deleted = self.conn.execute(
            "DELETE FROM notified WHERE offer_id NOT IN (SELECT id FROM offers)",
            [],
        )?;
        
        info!("Deleted {} orphaned notification records", deleted);
        Ok(deleted)
    }
    
    /// Run full cleanup (offers, stats, notifications)
    pub fn run_full_cleanup(&self, offer_retention_days: u32, stats_retention_days: u32) -> Result<(), StorageError> {
        info!("Running full database cleanup...");
        
        let offers_deleted = self.cleanup_old_offers(offer_retention_days)?;
        let stats_deleted = self.cleanup_old_stats(stats_retention_days)?;
        let notifications_deleted = self.cleanup_orphaned_notifications()?;
        
        // VACUUM to reclaim space
        info!("Running VACUUM to reclaim disk space...");
        self.conn.execute("VACUUM", [])?;
        
        info!(
            "Cleanup complete: {} offers, {} stats, {} notifications removed",
            offers_deleted, stats_deleted, notifications_deleted
        );
        
        Ok(())
    }
    
    /// Get database statistics
    pub fn get_database_stats(&self) -> Result<DatabaseStats, StorageError> {
        let offer_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM offers",
            [],
            |row| row.get(0),
        )?;
        
        let notified_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM notified",
            [],
            |row| row.get(0),
        )?;
        
        let stats_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM model_stats",
            [],
            |row| row.get(0),
        )?;
        
        let db_size: i64 = self.conn.query_row(
            "SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )?;
        
        Ok(DatabaseStats {
            offer_count: offer_count as usize,
            notified_count: notified_count as usize,
            stats_count: stats_count as usize,
            db_size_bytes: db_size as usize,
        })
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub offer_count: usize,
    pub notified_count: usize,
    pub stats_count: usize,
    pub db_size_bytes: usize,
}
