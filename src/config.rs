use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct ModelConfig {
    pub query: String,
    pub category_id: String,
    pub deviation_threshold: f64,
    pub min_price_delta: f64,
    pub min_price: f64,
    pub max_price: f64,
    pub match_keywords: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub telegram_bot_token: String,
    pub telegram_chat_id: i64,
    pub models: Vec<ModelConfig>,
    pub check_interval_seconds: u64,
}

pub fn load_config(path: &str) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: AppConfig = serde_json::from_str(&content)?;
    Ok(config)
}