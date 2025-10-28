use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct ModelConfig {
    pub query: String,
    pub category_id: String,
    pub deviation_threshold: f64,
    pub min_price_delta: f64,
    pub min_price: f64,
    pub max_price: f64,
    pub match_keywords: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScraperConfig {
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    
    #[serde(default = "default_delay_seconds")]
    pub delay_seconds: u64,
    
    #[serde(default = "default_user_agents")]
    pub user_agents: Vec<String>,
    
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,
}

impl Default for ScraperConfig {
    fn default() -> Self {
        Self {
            max_pages: default_max_pages(),
            delay_seconds: default_delay_seconds(),
            user_agents: default_user_agents(),
            max_retries: default_max_retries(),
            requests_per_minute: default_requests_per_minute(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnalyzerConfig {
    #[serde(default = "default_volatility_threshold")]
    pub volatility_threshold: f64,
    
    #[serde(default = "default_price_step")]
    pub price_step: u32,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            volatility_threshold: default_volatility_threshold(),
            price_step: default_price_step(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    /// Retention period for offers in days (0 = keep forever)
    #[serde(default = "default_offer_retention_days")]
    pub offer_retention_days: u32,
    
    /// Retention period for stats in days (0 = keep forever)
    #[serde(default = "default_stats_retention_days")]
    pub stats_retention_days: u32,
    
    /// Enable automatic cleanup on startup
    #[serde(default = "default_auto_cleanup")]
    pub auto_cleanup_on_startup: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            offer_retention_days: default_offer_retention_days(),
            stats_retention_days: default_stats_retention_days(),
            auto_cleanup_on_startup: default_auto_cleanup(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub telegram_bot_token: String,
    pub telegram_chat_id: i64,
    pub models: Vec<ModelConfig>,
    pub check_interval_seconds: u64,
    
    #[serde(default)]
    pub scraper: ScraperConfig,
    
    #[serde(default)]
    pub analyzer: AnalyzerConfig,
    
    #[serde(default)]
    pub database: DatabaseConfig,
}

// Default value functions
fn default_max_pages() -> usize { 20 }
fn default_delay_seconds() -> u64 { 1 }
fn default_volatility_threshold() -> f64 { 20.0 }
fn default_price_step() -> u32 { 50 }
fn default_max_retries() -> u32 { 3 }
fn default_requests_per_minute() -> u32 { 30 }
fn default_offer_retention_days() -> u32 { 30 }
fn default_stats_retention_days() -> u32 { 90 }
fn default_auto_cleanup() -> bool { true }

fn default_user_agents() -> Vec<String> {
    vec![
        // Chrome (latest 2024-2025)
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string(),
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36".to_string(),
        
        // Firefox (latest 2024-2025)
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0".to_string(),
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.7; rv:133.0) Gecko/20100101 Firefox/133.0".to_string(),
        
        // Edge (latest 2024-2025)
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0".to_string(),
        
        // Safari (latest 2024-2025)
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1.1 Safari/605.1.15".to_string(),
    ]
}

pub fn load_config(path: &str) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: AppConfig = serde_json::from_str(&content)?;
    validate_config(&config)?;
    Ok(config)
}

/// Configuration validation
fn validate_config(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Validate bot token
    if config.telegram_bot_token.trim().is_empty() {
        return Err("❌ Telegram bot token cannot be empty".into());
    }
    
    if !config.telegram_bot_token.contains(':') {
        return Err("❌ Invalid Telegram bot token format (must contain ':')".into());
    }
    
    // Validate chat_id
    if config.telegram_chat_id == 0 {
        return Err("❌ Telegram chat_id cannot be zero".into());
    }
    
    // Validate check interval
    if config.check_interval_seconds == 0 {
        return Err("❌ check_interval_seconds must be greater than zero".into());
    }
    
    if config.check_interval_seconds < 10 {
        eprintln!("⚠️ WARNING: check_interval_seconds is very low ({}s), recommended minimum is 60s", config.check_interval_seconds);
    }
    
    // Validate models
    if config.models.is_empty() {
        return Err("❌ At least one model must be configured".into());
    }
    
    for (idx, model) in config.models.iter().enumerate() {
        if model.query.trim().is_empty() {
            return Err(format!("❌ Model #{}: query cannot be empty", idx + 1).into());
        }
        
        if model.min_price < 0.0 {
            return Err(format!("❌ Model '{}': min_price cannot be negative", model.query).into());
        }
        
        if model.max_price < model.min_price {
            return Err(format!("❌ Model '{}': max_price ({}) must be >= min_price ({})", 
                model.query, model.max_price, model.min_price).into());
        }
        
        if model.deviation_threshold < 0.0 || model.deviation_threshold > 1.0 {
            return Err(format!("❌ Model '{}': deviation_threshold must be in range [0.0, 1.0]", model.query).into());
        }
        
        if model.match_keywords.is_empty() {
            eprintln!("⚠️ WARNING: Model '{}' has no keywords for filtering", model.query);
        }
    }
    
    // Validate scraper config
    if config.scraper.max_retries == 0 {
        eprintln!("⚠️ WARNING: max_retries is 0, no retry logic will be used");
    }
    
    if config.scraper.requests_per_minute == 0 {
        return Err("❌ requests_per_minute must be greater than zero".into());
    }
    
    if config.scraper.requests_per_minute > 120 {
        eprintln!("⚠️ WARNING: requests_per_minute is very high ({}), may cause rate limiting", config.scraper.requests_per_minute);
    }
    
    Ok(())
}