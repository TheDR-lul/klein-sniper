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
use std::path::Path;
use std::collections::HashSet;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = match load_config("config.json") {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Config load error: {e}");
            return;
        }
    };
    let config = Arc::new(config);

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
                Err(model::ScraperError::InvalidResponse(html)) => {
                    warn!("Invalid server response");
                    let folder = Path::new("logs/html");
                    if let Err(e) = fs::create_dir_all(folder) {
                        warn!("Failed to create debug folder: {e}");
                    } else {
                        let filename = folder.join(format!("debug-{}.html", model_cfg.query.replace(' ', "_")));
                        if let Err(e) = fs::write(&filename, html) {
                            warn!("Failed to save debug HTML: {e}");
                        } else {
                            info!("Saved debug HTML: {}", filename.display());
                        }
                    }
                    continue;
                }
                Err(e) => {
                    match e {
                        model::ScraperError::Timeout => warn!("Timeout while fetching page"),
                        model::ScraperError::HttpError(msg) => warn!("HTTP error: {msg}"),
                        _ => warn!("Unexpected error"),
                    }
                    continue;
                }
            };

            let mut offers = match parser.parse_filtered(&html, model_cfg) {
                Ok(o) => o,
                Err(e) => {
                    let path = format!("debug-{}.html", model_cfg.query.replace(" ", "_"));
                    if let Err(write_err) = fs::write(&path, &html) {
                        warn!("Failed to write debug HTML to {path}: {write_err:?}");
                    } else {
                        warn!("🧩 HTML сохранён в файл: {}", path);
                    }
                    match e {
                        model::ParserError::HtmlParseError(msg) => warn!("HTML parse error: {msg}"),
                        model::ParserError::MissingField(field) => warn!("Missing field: {field}"),
                    }
                    continue;
                }
            };

            normalize_all(&mut offers, &config.models);

            let mut seen_ids = HashSet::new();

            for offer in &offers {
                seen_ids.insert(offer.id.clone());
                if let Err(e) = storage.lock().await.save_offer(offer) {
                    warn!("DB save error: {e:?}");
                }
            }

            let seen_ids_vec: Vec<String> = seen_ids.iter().cloned().collect();
            if let Err(e) = storage.lock().await.delete_missing_offers(&seen_ids_vec) {
                warn!("Failed to delete missing offers: {e:?}");
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