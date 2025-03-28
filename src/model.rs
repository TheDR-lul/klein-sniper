// Core structs: Offer, ModelStats
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Offer {
    pub id: String,
    pub title: String,
    pub description: String,
    pub price: f64,
    pub location: String,
    pub model: String,
    pub link: String,
    pub posted_at: DateTime<Utc>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ModelStats {
    pub model: String,
    pub avg_price: f64,
    pub std_dev: f64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ScrapeRequest {
    pub query: String,
    pub category_id: String,
}

#[derive(Debug)]
pub enum ScraperError {
    HttpError(String),
    Timeout,
    InvalidResponse,
}

#[derive(Debug)]
pub enum ParserError {
    HtmlParseError(String),
    MissingField(String),
}

#[derive(Debug)]
pub enum StorageError {
    DatabaseError(String),
    NotFound,
}

#[derive(Debug)]
pub enum NotifyError {
    ApiError(String),
    Unreachable,
}
