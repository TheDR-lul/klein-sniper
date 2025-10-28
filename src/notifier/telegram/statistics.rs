use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

/// Глобальная статистика работы приложения
#[derive(Debug, Clone)]
pub struct AppStatistics {
    /// Время запуска приложения
    start_time: Instant,
    
    /// Количество обработанных циклов проверки
    pub check_cycles: Arc<AtomicU64>,
    
    /// Количество спарсенных офферов
    pub total_offers_parsed: Arc<AtomicU64>,
    
    /// Количество найденных выгодных сделок
    pub total_deals_found: Arc<AtomicU64>,
    
    /// Количество отправленных уведомлений
    pub total_notifications_sent: Arc<AtomicU64>,
    
    /// Количество HTTP ошибок при скрапинге
    pub total_scraper_errors: Arc<AtomicU64>,
    
    /// Количество ошибок парсинга
    pub total_parser_errors: Arc<AtomicU64>,
    
    /// Количество ошибок БД
    pub total_db_errors: Arc<AtomicU64>,
}

impl Default for AppStatistics {
    fn default() -> Self {
        Self::new()
    }
}

impl AppStatistics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            check_cycles: Arc::new(AtomicU64::new(0)),
            total_offers_parsed: Arc::new(AtomicU64::new(0)),
            total_deals_found: Arc::new(AtomicU64::new(0)),
            total_notifications_sent: Arc::new(AtomicU64::new(0)),
            total_scraper_errors: Arc::new(AtomicU64::new(0)),
            total_parser_errors: Arc::new(AtomicU64::new(0)),
            total_db_errors: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// Получить время работы приложения
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Увеличить счетчик циклов проверки
    pub fn increment_check_cycles(&self) {
        self.check_cycles.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Добавить количество спарсенных офферов
    pub fn add_parsed_offers(&self, count: u64) {
        self.total_offers_parsed.fetch_add(count, Ordering::Relaxed);
    }
    
    /// Добавить количество найденных сделок
    pub fn add_deals_found(&self, count: u64) {
        self.total_deals_found.fetch_add(count, Ordering::Relaxed);
    }
    
    /// Увеличить счетчик отправленных уведомлений
    pub fn increment_notifications(&self) {
        self.total_notifications_sent.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Увеличить счетчик ошибок скрапера
    pub fn increment_scraper_errors(&self) {
        self.total_scraper_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Увеличить счетчик ошибок парсера
    pub fn increment_parser_errors(&self) {
        self.total_parser_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Увеличить счетчик ошибок БД
    pub fn increment_db_errors(&self) {
        self.total_db_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Получить снимок статистики для экспорта
    pub fn snapshot(&self) -> StatisticsSnapshot {
        StatisticsSnapshot {
            uptime_seconds: self.uptime().as_secs(),
            check_cycles: self.check_cycles.load(Ordering::Relaxed),
            total_offers_parsed: self.total_offers_parsed.load(Ordering::Relaxed),
            total_deals_found: self.total_deals_found.load(Ordering::Relaxed),
            total_notifications_sent: self.total_notifications_sent.load(Ordering::Relaxed),
            total_scraper_errors: self.total_scraper_errors.load(Ordering::Relaxed),
            total_parser_errors: self.total_parser_errors.load(Ordering::Relaxed),
            total_db_errors: self.total_db_errors.load(Ordering::Relaxed),
        }
    }
    
    /// Форматированный отчет для Telegram
    pub fn formatted_report(&self) -> String {
        let uptime = self.uptime();
        let hours = uptime.as_secs() / 3600;
        let minutes = (uptime.as_secs() % 3600) / 60;
        let seconds = uptime.as_secs() % 60;
        
        format!(
            "📊 <b>Статистика работы приложения</b>\n\n\
             ⏱ <b>Время работы:</b> {:02}:{:02}:{:02}\n\
             🔄 <b>Циклов проверки:</b> {}\n\
             📦 <b>Всего офферов:</b> {}\n\
             💎 <b>Найдено сделок:</b> {}\n\
             📬 <b>Отправлено уведомлений:</b> {}\n\n\
             <b>Ошибки:</b>\n\
             ⚠️ Скрапер: {}\n\
             ⚠️ Парсер: {}\n\
             ⚠️ БД: {}",
            hours, minutes, seconds,
            self.check_cycles.load(Ordering::Relaxed),
            self.total_offers_parsed.load(Ordering::Relaxed),
            self.total_deals_found.load(Ordering::Relaxed),
            self.total_notifications_sent.load(Ordering::Relaxed),
            self.total_scraper_errors.load(Ordering::Relaxed),
            self.total_parser_errors.load(Ordering::Relaxed),
            self.total_db_errors.load(Ordering::Relaxed),
        )
    }
}

/// Снимок статистики для сериализации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsSnapshot {
    pub uptime_seconds: u64,
    pub check_cycles: u64,
    pub total_offers_parsed: u64,
    pub total_deals_found: u64,
    pub total_notifications_sent: u64,
    pub total_scraper_errors: u64,
    pub total_parser_errors: u64,
    pub total_db_errors: u64,
}

impl StatisticsSnapshot {
    /// Экспорт в JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    
    /// Экспорт в CSV (простой формат)
    pub fn to_csv(&self) -> String {
        format!(
            "metric,value\n\
             uptime_seconds,{}\n\
             check_cycles,{}\n\
             total_offers_parsed,{}\n\
             total_deals_found,{}\n\
             total_notifications_sent,{}\n\
             total_scraper_errors,{}\n\
             total_parser_errors,{}\n\
             total_db_errors,{}",
            self.uptime_seconds,
            self.check_cycles,
            self.total_offers_parsed,
            self.total_deals_found,
            self.total_notifications_sent,
            self.total_scraper_errors,
            self.total_parser_errors,
            self.total_db_errors,
        )
    }
}

