use chrono::{DateTime, Utc};
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
}

/// Статистика по модели (для анализа отклонений)
#[derive(Debug, Clone)]
pub struct ModelStats {
    pub model: String,
    pub avg_price: f64,
    pub std_dev: f64,
    pub last_updated: DateTime<Utc>,
}

/// Запрос для парсера
#[derive(Debug, Clone)]
pub struct ScrapeRequest {
    pub query: String,
    pub category_id: String,
}

/// Ошибки, возникающие при загрузке страниц
#[derive(Debug)]
pub enum ScraperError {
    HttpError(String),
    Timeout,
    InvalidResponse,
}

/// Ошибки, возникающие при разборе HTML
#[derive(Debug)]
pub enum ParserError {
    HtmlParseError(String),
    MissingField(String),
}

/// Ошибки, связанные с хранилищем (БД)
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("❌ Ошибка базы данных: {0}")]
    DatabaseError(String),

    #[error("🔍 Не найдено")]
    NotFound,
}

// Автоматическое преобразование rusqlite::Error в StorageError
impl From<rusqlite::Error> for StorageError {
    fn from(err: rusqlite::Error) -> Self {
        StorageError::DatabaseError(err.to_string())
    }
}

// Автоматическое преобразование chrono::ParseError в StorageError
impl From<chrono::ParseError> for StorageError {
    fn from(err: chrono::ParseError) -> Self {
        StorageError::DatabaseError(err.to_string())
    }
}

/// Ошибки при уведомлениях (например, Telegram)
#[derive(Debug)]
pub enum NotifyError {
    ApiError(String),
    Unreachable,
}
