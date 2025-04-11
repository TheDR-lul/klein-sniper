mod config;
mod model;
mod scraper;
mod parser;
mod analyzer;
mod normalizer;
mod notifier;
mod storage;

use analyzer::AnalyzerImpl;
use notifier::TelegramNotifier;
use crate::analyzer::price_analysis::Analyzer;
use config::{load_config, AppConfig, ModelConfig};
use model::ScrapeRequest;
use scraper::{Scraper, ScraperImpl};
use parser::KleinanzeigenParser;
use normalizer::normalize_all;
use storage::SqliteStorage;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use tracing_subscriber;
use futures::future::join_all;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Set panic hook to log details about any panic
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("😱 Panic occurred: {:?}", panic_info);
    }));

    // Load configuration from file
    let config: Arc<AppConfig> = match load_config("config.json") {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            error!("Config load error: {}", e);
            return;
        }
    };

    // Create the base scraper instance
    let base_scraper = ScraperImpl::new();
    let parser = KleinanzeigenParser::new();
    let analyzer = AnalyzerImpl::new();

    // Initialize storage (SQLite) with async access (wrapped in a Mutex)
    let storage = match SqliteStorage::new("data.db") {
        Ok(s) => Arc::new(Mutex::new(s)),
        Err(e) => {
            error!("Failed to initialize storage: {:?}", e);
            return;
        }
    };

    // Initialize notifier (Telegram) and refresh notifier
    let refresh_notify = Arc::new(Notify::new());
    let notifier = Arc::new(TelegramNotifier::new(
        config.telegram_bot_token.clone(),
        config.telegram_chat_id,
        storage.clone(),
        config.clone(),
        refresh_notify.clone(),
    ));

    // Spawn listener for manual refresh (e.g. via /refresh command)
    TelegramNotifier::spawn_listener(notifier.clone());

    info!("Sending startup message...");
    if let Err(e) = notifier.notify_text("🚀 KleinSniper started!").await {
        warn!("Startup notification failed: {:?}", e);
    }

    // Main processing loop
    loop {
        info!("Entering main loop...");
        info!("Models to process: {}", config.models.len());

        // Process all models concurrently
        let tasks: Vec<_> = config.models.iter().map(|model_cfg| {
            process_model(
                model_cfg,
                &base_scraper,
                &parser,
                &analyzer,
                storage.clone(),
                config.clone(),
                refresh_notify.clone(),
                notifier.clone(),
            )
        }).collect();
        join_all(tasks).await;

        info!(
            "Waiting for timer ({}s) or manual refresh...",
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

/// Processes a single model, performing scraping, parsing, normalization, analysis and notifications.
/// The functionality remains the same as in the original main loop.
async fn process_model(
    model_cfg: &ModelConfig,
    base_scraper: &ScraperImpl,
    parser: &KleinanzeigenParser,
    analyzer: &AnalyzerImpl,
    storage: Arc<Mutex<SqliteStorage>>,
    config: Arc<AppConfig>,
    _refresh_notify: Arc<Notify>,
    notifier: Arc<TelegramNotifier>,
) {
    info!("Processing model: {}", model_cfg.query);
    let request = ScrapeRequest {
        query: model_cfg.query.clone(),
        category_id: model_cfg.category_id.clone(),
    };

    // Create a scraper instance for the current model (cloning the client)
    let scraper = ScraperImpl {
        client: base_scraper.client.clone(),
        category_id: model_cfg.category_id.clone(),
        min_price: model_cfg.min_price,
        max_price: model_cfg.max_price,
    };

    // Optionally, retrieve previous stats from storage for logging
    {
        let storage_guard = storage.lock().await;
        if let Ok(Some(prev_stats)) = storage_guard.get_stats(&model_cfg.query) {
            info!(
                "Previous stats: {:.2} € | Updated: {}",
                prev_stats.avg_price, prev_stats.last_updated
            );
        }
    }

    info!("Fetching offers...");
    // Fetch HTML page for the current request
    let html = match scraper.fetch(&request).await {
        Ok(html) => html,
        Err(model::ScraperError::InvalidResponse(html)) => {
            log_and_save_html(&html, &model_cfg.query);
            return;
        }
        Err(e) => {
            warn!("Scraper error: {:?}", e);
            return;
        }
    };

    info!("Parsing HTML...");
    // Parse offers from the HTML
    let mut offers = match parser.parse_filtered(&html, model_cfg) {
        Ok(o) => o,
        Err(e) => {
            log_and_save_html(&html, &model_cfg.query);
            warn!("Parse error: {:?}", e);
            return;
        }
    };

    // Normalize offers based on configuration settings
    normalize_all(&mut offers, &config.models);

    // Save offers into storage and record seen IDs
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

    // Perform asynchronous extended analysis of the offers
    info!("Performing extended asynchronous analysis...");
    let analysis_result = analyzer.analyze_offers(&offers).await;
    info!("Advanced Analysis Results:");
    for (range, duration) in analysis_result.disappearance_map.iter() {
        info!(
            "Price Range {}-{}: Average Lifespan (s): {}",
            range.0,
            range.1,
            duration.num_seconds()
        );
    }
    info!("Price Change Frequency: {}", analysis_result.price_change_frequency);
    info!("RSI: {}", analysis_result.rsi);

    // Calculate basic statistics for the offers
    let stats = analyzer.calculate_stats(&offers);
    info!(
        "Base Stats: avg = {:.2}, std_dev = {:.2}",
        stats.avg_price, stats.std_dev
    );

    info!("Updating stats in storage...");
    if let Err(e) = storage.lock().await.update_stats(&stats) {
        warn!("Stats update failed: {:?}", e);
    }

    info!("Notifying cheapest offers...");
    TelegramNotifier::check_and_notify_cheapest_for_model(
        &model_cfg.query,
        storage.clone(),
        notifier.clone(),
    )
    .await;

    // Find "good" offers using the analyzer's deal finding method
    let good_offers = analyzer.find_deals(&offers, &stats, model_cfg);
    info!("Found {} good offers", good_offers.len());

    // Process each good offer and send notifications if necessary
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

    info!("Finished processing model: {}", model_cfg.query);
}

/// Logs and saves the provided HTML for debugging purposes.
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