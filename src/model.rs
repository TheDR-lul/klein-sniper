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

/// Ошибки, возникающие при загрузке страниц
#[derive(Debug, Error)]
pub enum ScraperError {
    #[error("🌐 HTTP ошибка: {0}")]
    HttpError(String),
    
    #[error("❌ Некорректный ответ сервера: {0}")]
    InvalidResponse(String),
    
    #[error("📄 Ошибка парсинга HTML: {0}")]
    HtmlParseError(String),
}

/// Ошибки, возникающие при разборе HTML
#[derive(Debug, Error)]
pub enum ParserError {
    #[error("📄 Ошибка парсинга HTML: {0}")]
    HtmlParseError(String),
    
    #[error("🔍 Отсутствующее поле: {0}")]
    MissingField(String),
}

/// Ошибки, связанные с хранилищем (БД)
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("❌ Ошибка базы данных: {0}")]
    DatabaseError(String),

    #[error("🔍 Не найдено")]
    NotFound,

    #[error("📅 Ошибка парсинга даты: {0}")]
    ParseError(#[from] ParseError),
}

// Автоматическое преобразование rusqlite::Error в StorageError
impl From<rusqlite::Error> for StorageError {
    fn from(err: rusqlite::Error) -> Self {
        StorageError::DatabaseError(err.to_string())
    }
}

/// Ошибки при уведомлениях (например, Telegram)
#[derive(Debug, Error)]
pub enum NotifyError {
    #[error("📡 Ошибка API: {0}")]
    ApiError(String),
    
    #[error("🔌 Сервис недоступен")]
    Unreachable,
}
