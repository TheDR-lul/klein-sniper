mod config;
mod model;
mod scraper;
mod parser;
mod analyzer;
mod normalizer;
mod notifier;
mod storage;

use analyzer::AnalyzerImpl;
use notifier::telegram::{check_and_notify_cheapest_for_model, spawn_listener, TelegramNotifier};
use crate::analyzer::price_analysis::Analyzer;
use config::load_config;
use model::ScrapeRequest;
use scraper::{Scraper, ScraperImpl};
use parser::KleinanzeigenParser;
use normalizer::normalize_all;
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

    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("😱 Panic occurred: {:?}", panic_info);
    }));

    let config = match load_config("config.json") {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            error!("Config load error: {}", e);
            return;
        }
    };

    // Base scraper instance; its client will be cloned for each model's scraper.
    let base_scraper = ScraperImpl::new();
    let parser = KleinanzeigenParser::new();
    let analyzer = AnalyzerImpl::new();

    let storage = match SqliteStorage::new("data.db") {
        Ok(s) => Arc::new(Mutex::new(s)),
        Err(e) => {
            error!("Failed to initialize storage: {:?}", e);
            return;
        }
    };

    let refresh_notify = Arc::new(Notify::new());
    let notifier = Arc::new(TelegramNotifier::new(
        config.telegram_bot_token.clone(),
        config.telegram_chat_id,
        storage.clone(),
        config.clone(),
        refresh_notify.clone(),
    ));
    let best_deal_ids = Arc::new(Mutex::new(HashMap::<String, String>::new()));

    spawn_listener(notifier.clone());

    info!("Sending startup message...");
    if let Err(e) = notifier.notify_text("🚀 KleinSniper started!").await {
        warn!("Startup notification failed: {:?}", e);
    }

    loop {
        info!("Entering main loop...");
        info!("Models to process: {}", config.models.len());

        for model_cfg in &config.models {
            info!("Processing: {}", model_cfg.query);
            let request = ScrapeRequest {
                query: model_cfg.query.clone(),
                category_id: model_cfg.category_id.clone(),
            };

            // Create a scraper instance for the current model using settings from model_cfg.
            let scraper = ScraperImpl {
                client: base_scraper.client.clone(),
                category_id: model_cfg.category_id.clone(),
                min_price: model_cfg.min_price,
                max_price: model_cfg.max_price,
            };

            if let Ok(Some(prev_stats)) = storage.lock().await.get_stats(&model_cfg.query) {
                info!(
                    "Previous stats: {:.2} € | Updated: {}",
                    prev_stats.avg_price, prev_stats.last_updated
                );
            }

            info!("Fetching offers...");
            let html = match scraper.fetch(&request).await {
                Ok(html) => html,
                Err(model::ScraperError::InvalidResponse(html)) => {
                    log_and_save_html(&html, &model_cfg.query);
                    continue;
                }
                Err(e) => {
                    warn!("Scraper error: {:?}", e);
                    continue;
                }
            };

            info!("Parsing HTML...");
            let mut offers = match parser.parse_filtered(&html, model_cfg) {
                Ok(o) => o,
                Err(e) => {
                    log_and_save_html(&html, &model_cfg.query);
                    warn!("Parse error: {:?}", e);
                    continue;
                }
            };

            normalize_all(&mut offers, &config.models);

            let mut seen_ids = HashSet::new();
            for offer in &offers {
                seen_ids.insert(offer.id.clone());
                if let Err(e) = storage.lock().await.save_offer(offer) {
                    warn!("DB save error: {:?}", e);
                }
            }

            let seen_vec: Vec<String> = seen_ids.into_iter().collect();
            info!("Cleaning up old offers for model {}...", model_cfg.query);
            if let Err(e) = storage
                .lock()
                .await
                .delete_missing_offers_for_model(&model_cfg.query, &seen_vec)
            {
                warn!("Delete missing error: {:?}", e);
            }

            let stats = analyzer.calculate_stats(&offers);
            info!("Stats: avg = {:.2}, std_dev = {:.2}", stats.avg_price, stats.std_dev);

            info!("Updating stats...");
            if let Err(e) = storage.lock().await.update_stats(&stats) {
                warn!("Stats update failed: {:?}", e);
            }

            info!("Notifying cheapest...");
            check_and_notify_cheapest_for_model(
                &model_cfg.query,
                storage.clone(),
                notifier.clone(),
            )
            .await;

            let good_offers = analyzer.find_deals(&offers, &stats, model_cfg);
            info!("Good offers: {}", good_offers.len());

            for offer in good_offers {
                info!("Checking offer: {} — {:.2} €", offer.id, offer.price);

                match storage.lock().await.is_notified(&offer.id) {
                    Ok(true) => {
                        info!("Already notified: {}", offer.id);
                        continue;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        warn!("Notify check failed: {:?}", e);
                        continue;
                    }
                }

                info!("Sending Telegram notification...");
                if let Err(e) = notifier.notify(&offer).await {
                    warn!("Telegram send error: {:?}", e);
                } else if let Err(e) = storage.lock().await.mark_notified(&offer.id) {
                    warn!("Mark notified failed: {:?}", e);
                } else {
                    info!("Offer notified and marked.");
                }
            }

            info!("Finished model: {}", model_cfg.query);
        }

        info!(
            "Waiting for timer ({}s) or /refresh...",
            config.check_interval_seconds
        );

        tokio::select! {
            _ = sleep(Duration::from_secs(config.check_interval_seconds)) => {
                info!("Timer triggered.");
            }
            _ = refresh_notify.notified() => {
                info!("Manual refresh triggered.");
            }
        }

        info!("Restarting main loop...");
    }
}

fn log_and_save_html(html: &str, query: &str) {
    let folder = Path::new("logs/html");
    if let Err(e) = fs::create_dir_all(folder) {
        warn!("Failed to create debug folder: {}", e);
        return;
    }

    let filename = folder.join(format!("debug-{}.html", query.replace(' ', "_")));
    if let Err(e) = fs::write(&filename, html) {
        warn!("Failed to write debug HTML: {}", e);
    } else {
        info!("Saved debug HTML: {}", filename.display());
    }
}