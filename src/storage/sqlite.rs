use crate::model::{Offer, ModelStats, StorageError};
use rusqlite::{params, Connection};
use chrono::{DateTime, Utc};

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
                fetched_at TEXT NOT NULL
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

        Ok(Self { conn })
    }

    pub fn save_offer(&self, offer: &Offer) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO offers (id, title, price, model, link, posted_at, fetched_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &offer.id,
                &offer.title,
                &offer.price,
                &offer.model,
                &offer.link,
                &offer.posted_at.to_rfc3339(),
                &offer.fetched_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn is_notified(&self, offer_id: &str) -> Result<bool, StorageError> {
        let mut stmt = self.conn.prepare("SELECT 1 FROM notified WHERE offer_id = ?1")?;
        let mut rows = stmt.query(params![offer_id])?;
        Ok(rows.next()?.is_some())
    }

    pub fn mark_notified(&self, offer_id: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO notified (offer_id, notified_at) VALUES (?1, datetime('now'))",
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
}