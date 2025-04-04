use crate::model::{ModelStats, Offer, StorageError};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};

pub struct SqliteStorage {
    conn: Connection,
}

impl SqliteStorage {
    pub fn new(db_path: &str) -> Result<Self, StorageError> {
        let conn = Connection::open(db_path)?;

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
            ",
        )?;

        // Automigrations
        Self::migrate_add_column_if_missing(&conn, "offers", "location", "TEXT NOT NULL DEFAULT ''")?;
        Self::migrate_add_column_if_missing(&conn, "offers", "description", "TEXT NOT NULL DEFAULT ''")?;

        Ok(Self { conn })
    }

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

    pub fn save_offer(&self, offer: &Offer) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO offers (id, title, price, model, link, posted_at, fetched_at, location, description)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
            ],
        )?;
        Ok(())
    }

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

    pub fn is_notified(&self, offer_id: &str) -> Result<bool, StorageError> {
        let mut stmt = self.conn.prepare("SELECT 1 FROM notified WHERE offer_id = ?1")?;
        let mut rows = stmt.query(params![offer_id])?;
        Ok(rows.next()?.is_some())
    }

    /// Returns true if no record exists or if more than 24 hours passed since last notification.
    pub fn should_notify(&self, offer_id: &str) -> Result<bool, StorageError> {
        let mut stmt = self.conn.prepare("SELECT notified_at FROM notified WHERE offer_id = ?1")?;
        let mut rows = stmt.query(params![offer_id])?;
        if let Some(row) = rows.next()? {
            let notified_at_str: String = row.get(0)?;
            if notified_at_str.trim().is_empty() {
                return Ok(true);
            }
            let notified_at: DateTime<Utc> = notified_at_str
                .parse()
                .map_err(|e| StorageError::DatabaseError(format!("Invalid datetime: {}", e)))?;
            Ok(Utc::now() - notified_at > Duration::hours(24))
        } else {
            Ok(true)
        }
    }

    pub fn mark_notified(&self, offer_id: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO notified (offer_id, notified_at) VALUES (?1, datetime('now'))",
            params![offer_id],
        )?;
        Ok(())
    }

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

    pub fn get_last_offer(&self) -> Result<Option<Offer>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, price, model, link, posted_at, fetched_at, location, description
             FROM offers ORDER BY fetched_at DESC LIMIT 1",
        )?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let posted_at_str: String = row.get(5)?;
            let fetched_at_str: String = row.get(6)?;
            Ok(Some(Offer {
                id: row.get(0)?,
                title: row.get(1)?,
                price: row.get(2)?,
                model: row.get(3)?,
                link: row.get(4)?,
                posted_at: posted_at_str.parse()?,
                fetched_at: fetched_at_str.parse()?,
                location: row.get(7)?,
                description: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_top5_offers(&self) -> Result<Vec<Offer>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, price, model, link, posted_at, fetched_at, location, description
             FROM offers WHERE price > 0 ORDER BY price ASC LIMIT 5",
        )?;

        let rows = stmt.query_map([], |row| {
            let posted_at_str: String = row.get(5)?;
            let fetched_at_str: String = row.get(6)?;
            Ok(Offer {
                id: row.get(0)?,
                title: row.get(1)?,
                price: row.get(2)?,
                model: row.get(3)?,
                link: row.get(4)?,
                posted_at: posted_at_str.parse().map_err(|e| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e)))?,
                fetched_at: fetched_at_str.parse().map_err(|e| rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e)))?,
                location: row.get(7)?,
                description: row.get(8)?,
            })
        })?;

        let mut offers = Vec::new();
        for offer in rows {
            offers.push(offer?);
        }

        Ok(offers)
    }

    pub fn get_all_offers(&self) -> Result<Vec<Offer>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, price, model, link, posted_at, fetched_at, location, description FROM offers",
        )?;

        let rows = stmt.query_map([], |row| {
            let posted_at_str: String = row.get(5)?;
            let fetched_at_str: String = row.get(6)?;
            Ok(Offer {
                id: row.get(0)?,
                title: row.get(1)?,
                price: row.get(2)?,
                model: row.get(3)?,
                link: row.get(4)?,
                posted_at: posted_at_str.parse().map_err(|e| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e)))?,
                fetched_at: fetched_at_str.parse().map_err(|e| rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e)))?,
                location: row.get(7)?,
                description: row.get(8)?,
            })
        })?;

        let mut offers = Vec::new();
        for offer in rows {
            offers.push(offer?);
        }

        Ok(offers)
    }

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
}