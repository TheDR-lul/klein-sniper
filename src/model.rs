use chrono::{DateTime, Utc,ParseError};
use thiserror::Error;
use rusqlite;

/// Основная информация об объявлении
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
    pub user_id: Option<String>,     
    pub user_name: Option<String>,   
    pub user_url: Option<String>,    
}
/// Статистика по модели (для анализа отклонений)
#[derive(Debug, Clone)]
pub struct ModelStats {
    pub model: String,
    pub avg_price: f64,
    pub std_dev: f64,
    pub last_updated: DateTime<Utc>,
}


#[derive(Debug)]
pub struct OfferLifecycle {
    pub price: f64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub price_changes: u32,
}
/// Запрос для парсера
#[derive(Debug, Clone)]
pub struct ScrapeRequest {
    pub query: String,
    pub category_id: String,
}

/// Errors that can occur during page fetching
#[derive(Debug, Error)]
pub enum ScraperError {
    #[error("🌐 HTTP error: {0}")]
    HttpError(String),
    
    #[error("❌ Invalid server response: {0}")]
    InvalidResponse(String),
    
    #[error("📄 HTML parsing error: {0}")]
    HtmlParseError(String),
}

/// Errors that can occur during HTML parsing
#[derive(Debug, Error)]
pub enum ParserError {
    #[error("📄 HTML parsing error: {0}")]
    HtmlParseError(String),
    
    #[error("🔍 Missing field: {0}")]
    MissingField(String),
}

/// Errors related to storage (DB)
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("❌ Database error: {0}")]
    DatabaseError(String),

    #[error("🔍 Not found")]
    NotFound,

    #[error("📅 Date parsing error: {0}")]
    ParseError(#[from] ParseError),
}

// Automatic conversion from rusqlite::Error to StorageError
impl From<rusqlite::Error> for StorageError {
    fn from(err: rusqlite::Error) -> Self {
        StorageError::DatabaseError(err.to_string())
    }
}

/// Errors during notifications (e.g., Telegram)
#[derive(Debug, Error)]
pub enum NotifyError {
    #[error("📡 API error: {0}")]
    ApiError(String),
    
    #[error("🔌 Service unreachable")]
    Unreachable,
}
