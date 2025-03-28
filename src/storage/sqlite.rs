// SQLite backend

use crate::model::{Offer, ModelStats};
use rusqlite::{params, Connection, Result};

pub struct SqliteStorage {
    conn: Connection,
}

impl SqliteStorage {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        Ok(Self { conn })
    }

    pub fn save_offer(&self, offer: &Offer) -> Result<()> {
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

    pub fn is_notified(&self, offer_id: &str) -> Result<bool, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT 1 FROM notified WHERE offer_id = ?1")?;
        let mut rows = stmt.query(params![offer_id])?;
    
        Ok(rows.next()?.is_some())
    }    

    pub fn mark_notified(&self, offer_id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO notified (offer_id, notified_at) VALUES (?1, datetime('now'))",
            params![offer_id],
        )?;
        Ok(())
    }

    pub fn get_stats(&self, model: &str) -> Result<Option<ModelStats>> {
        let mut stmt = self.conn.prepare(
            "SELECT avg_price, std_dev, last_updated FROM model_stats WHERE model = ?1",
        )?;
        let mut rows = stmt.query(params![model])?;

        if let Some(row) = rows.next()? {
            Ok(Some(ModelStats {
                model: model.to_string(),
                avg_price: row.get(0)?,
                std_dev: row.get(1)?,
                last_updated: row.get(2)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_stats(&self, stats: &ModelStats) -> Result<()> {
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