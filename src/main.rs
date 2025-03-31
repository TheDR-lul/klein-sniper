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
use notifier::telegram::check_and_notify_cheapest_for_model;
use crate::analyzer::price_analysis::Analyzer;
use config::load_config;
use model::ScrapeRequest;
use scraper::{Scraper, ScraperImpl};
use parser::KleinanzeigenParser;
use normalizer::normalize_all;
use notifier::TelegramNotifier;
use storage::SqliteStorage;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = match load_config("config.json") {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            error!("Config load error: {e}");
            return;
        }
    };

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
    let best_deal_ids = Arc::new(Mutex::new(HashMap::<String, String>::new()));

    let notifier_clone = notifier.clone();
    tokio::spawn(async move {
        info!("▶️ Starting Telegram listener...");
        notifier_clone.lock().await.listen_for_commands().await;
        info!("🛑 Telegram listener ended.");
    });

    info!("📨 Sending startup message...");
    if let Err(e) = notifier.lock().await.notify_text("🚀 KleinSniper запущен!").await {
        warn!("Startup notification failed: {e:?}");
    }

    loop {
        info!("🔁 Entering main loop...");
        info!("📦 Models to process: {}", config.models.len());

        for model_cfg in &config.models {
            info!("🔄 Processing: {}", model_cfg.query);
            let request = ScrapeRequest {
                query: model_cfg.query.clone(),
                category_id: model_cfg.category_id.clone(),
            };

            if let Ok(Some(prev_stats)) = storage.lock().await.get_stats(&model_cfg.query) {
                info!("ℹ️ Previous stats: {:.2} € | Updated: {}", prev_stats.avg_price, prev_stats.last_updated);
            }

            info!("🌐 Fetching offers...");
            let html = match scraper.fetch(&request).await {
                Ok(html) => html,
                Err(model::ScraperError::InvalidResponse(html)) => {
                    log_and_save_html(&html, &model_cfg.query);
                    continue;
                }
                Err(e) => {
                    warn!("❌ Scraper error: {e:?}");
                    continue;
                }
            };

            info!("🧩 Parsing HTML...");
            let mut offers = match parser.parse_filtered(&html, model_cfg) {
                Ok(o) => o,
                Err(e) => {
                    log_and_save_html(&html, &model_cfg.query);
                    warn!("❌ Parse error: {e:?}");
                    continue;
                }
            };

            normalize_all(&mut offers, &config.models);

            let mut seen_ids = HashSet::new();
            for offer in &offers {
                seen_ids.insert(offer.id.clone());

                //info!("💾 Saving offer: {} | {:.2} € | {}", offer.id, offer.price, offer.link);
                if let Err(e) = storage.lock().await.save_offer(offer) {
                    warn!("DB save error: {e:?}");
                }
            }

            let seen_vec: Vec<String> = seen_ids.iter().cloned().collect();
            info!("🧹 Cleaning up old offers...");
            if let Err(e) = storage.lock().await.delete_missing_offers(&seen_vec) {
                warn!("Delete missing error: {e:?}");
            }

            let stats = analyzer.calculate_stats(&offers);
            info!("📈 Stats: avg = {:.2}, std_dev = {:.2}", stats.avg_price, stats.std_dev);

            info!("📥 Updating stats...");
            if let Err(e) = storage.lock().await.update_stats(&stats) {
                warn!("Stats update failed: {e:?}");
            }

            info!("🧪 Notifying cheapest...");
            check_and_notify_cheapest_for_model(
                &model_cfg.query,
                storage.clone(),
                notifier.clone(),
                best_deal_ids.clone(),
            ).await;

            let good_offers = analyzer.find_deals(&offers, &stats, model_cfg);
            info!("✅ Good offers: {}", good_offers.len());

            for offer in good_offers {
                info!("[deal] {} — {:.2} € | {}", offer.title, offer.price, offer.link);

                info!("🔎 Checking if notified: {}", offer.id);
                match storage.lock().await.is_notified(&offer.id) {
                    Ok(true) => {
                        info!("🔕 Already notified.");
                        continue;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        warn!("❌ Notify check failed: {e:?}");
                        continue;
                    }
                }

                info!("📤 Sending notification to Telegram for offer: {}", offer.id);
                if let Err(e) = notifier.lock().await.notify(&offer).await {
                    warn!("Telegram send error: {e:?}");
                } else {
                    info!("✅ Sent. Marking as notified...");
                    if let Err(e) = storage.lock().await.mark_notified(&offer.id) {
                        warn!("Mark notified failed: {e:?}");
                    }
                }
            }

            info!("[✅] Finished model: {}", model_cfg.query);
        }

        info!("😴 Sleeping or waiting /refresh...");
        tokio::select! {
            _ = sleep(Duration::from_secs(config.check_interval_seconds)) => {
                info!("⏰ Timer wakeup.");
            },
            _ = refresh_notify.notified() => {
                info!("🖐 Manual refresh wakeup.");
            }
        }

        info!("🔁 Re-entering loop...");
    }
}

fn log_and_save_html(html: &str, query: &str) {
    let folder = Path::new("logs/html");
    if let Err(e) = fs::create_dir_all(folder) {
        warn!("Failed to create debug folder: {e}");
        return;
    }

    let filename = folder.join(format!("debug-{}.html", query.replace(' ', "_")));
    if let Err(e) = fs::write(&filename, html) {
        warn!("Failed to write debug HTML: {e}");
    } else {
        info!("📄 Saved debug HTML to: {}", filename.display());
    }
}