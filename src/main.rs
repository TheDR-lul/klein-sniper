//! main.rs

mod config;
mod model;
mod scraper;
mod parser;
mod analyzer;
mod normalizer;
mod notifier;
mod storage;

use analyzer::AnalyzerImpl;
use crate::analyzer::price_analysis::Analyzer;
use config::load_config;
use model::ScrapeRequest;
use scraper::{Scraper, ScraperImpl};
use parser::KleinanzeigenParser;
use normalizer::normalize_all;
use notifier::TelegramNotifier;
use storage::SqliteStorage;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::time::{sleep, Duration};
use tracing::{info, error, warn};
use tracing_subscriber;
use std::fs;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // 1. Загрузка конфигурации
    let config = match load_config("config.json") {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Config load error: {e}");
            return;
        }
    };
    let config = Arc::new(config);

    // 2. Инициализация модулей
    let scraper = ScraperImpl::new();
    let parser = KleinanzeigenParser::new();
    let analyzer = AnalyzerImpl::new();
    let storage = Arc::new(Mutex::new(SqliteStorage::new("data.db").unwrap()));
    let refresh_notify = Arc::new(Notify::new());
    let notifier = Arc::new(Mutex::new(TelegramNotifier::new(
        config.telegram_bot_token.clone(),
        config.telegram_chat_id,
        storage.clone(),
        config.clone(),
        refresh_notify.clone(),
    )));

    // 3. Запуск слушателя команд Telegram
    let command_notifier = notifier.clone();
    tokio::spawn(async move {
        command_notifier.lock().await.listen_for_commands().await;
    });

    if let Err(e) = notifier.lock().await.notify_text("🚀 KleinSniper запущен!").await {
        warn!("Failed to send startup notification: {e:?}");
    }

    loop {
        info!("Starting new analysis cycle for {} model(s)...", config.models.len());

        for model_cfg in &config.models {
            info!("Processing model: {}", model_cfg.query);

            let request = ScrapeRequest {
                query: model_cfg.query.clone(),
                category_id: model_cfg.category_id.clone(),
            };

            if let Ok(Some(prev_stats)) = storage.lock().await.get_stats(&model_cfg.query) {
                info!("Previous avg price: {:.2} € (updated: {})", prev_stats.avg_price, prev_stats.last_updated);
            }

            let html = match scraper.fetch(&request).await {
                Ok(html) => html,
                Err(e) => {
                    match e {
                        model::ScraperError::Timeout => warn!("Timeout while fetching page"),
                        model::ScraperError::HttpError(msg) => warn!("HTTP error: {msg}"),
                        model::ScraperError::InvalidResponse => warn!("Invalid server response"),
                    }
                    continue;
                }
            };

            let mut offers = match parser.parse(&html) {
                Ok(o) => o,
                Err(e) => {
                    match e {
                        model::ParserError::HtmlParseError(msg) => warn!("HTML parse error: {msg}"),
                        model::ParserError::MissingField(field) => warn!("Missing field: {field}"),
                    }
                    continue;
                }
            };

            normalize_all(&mut offers, &config.models);

            for offer in &offers {
                if let Err(e) = storage.lock().await.save_offer(offer) {
                    warn!("DB save error: {e:?}");
                }
            }

            let stats = analyzer.calculate_stats(&offers);

            if let Err(e) = storage.lock().await.update_stats(&stats) {
                warn!("Failed to update stats: {e:?}");
            }

            let good_offers = analyzer.find_deals(&offers, &stats, model_cfg);

            info!("📊 {} | Avg: {:.2} € | StdDev: {:.2} | Found: {}", stats.model, stats.avg_price, stats.std_dev, good_offers.len());

            for offer in good_offers {
                info!("[deal] {} — {:.2} € | {}", offer.title, offer.price, offer.link);

                match storage.lock().await.is_notified(&offer.id) {
                    Ok(true) => {
                        info!("[skip] Already notified");
                        continue;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        warn!("Notify check error: {e:?}");
                        continue;
                    }
                }

                if let Err(e) = notifier.lock().await.notify(&offer).await {
                    warn!("Telegram send error: {e:?}");
                } else if let Err(e) = storage.lock().await.mark_notified(&offer.id) {
                    warn!("Notify mark error: {e:?}");
                }
            }

            info!("[done] Finished model: {}", model_cfg.query);
        }

        info!("[wait] Sleeping for {} seconds", config.check_interval_seconds);
        tokio::select! {
            _ = sleep(Duration::from_secs(config.check_interval_seconds)) => {},
            _ = refresh_notify.notified() => {
                info!("[refresh] Manual refresh triggered");
            }
        }
    }
}