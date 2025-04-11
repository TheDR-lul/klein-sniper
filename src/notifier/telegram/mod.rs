pub mod sender;
pub mod listener;
pub mod command_handler;
pub mod statistics;

use crate::model::{NotifyError, Offer};
use crate::storage::SqliteStorage;
use crate::config::AppConfig;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use std::sync::atomic::AtomicI64;
use std::time::Instant;

pub struct TelegramNotifier {
    pub bot_token: String,
    pub chat_id: i64,
    pub client: Client,
    pub offset: Arc<AtomicI64>,
    pub storage: Arc<Mutex<SqliteStorage>>,
    pub config: Arc<AppConfig>,
    pub start_time: Instant,
    pub refresh_notify: Arc<Notify>,
}

impl TelegramNotifier {
    pub fn new(
        bot_token: String,
        chat_id: i64,
        storage: Arc<Mutex<SqliteStorage>>,
        config: Arc<AppConfig>,
        refresh_notify: Arc<Notify>,
    ) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("❗ Failed to create HTTP client");
        Self {
            bot_token: bot_token.clone(),
            chat_id,
            client,
            offset: Arc::new(AtomicI64::new(0)),
            storage,
            config,
            start_time: Instant::now(),
            refresh_notify,
        }
    }

    pub async fn notify_text(&self, text: &str) -> Result<(), reqwest::Error> {
        sender::send_text(self, text).await
    }

    pub async fn notify(&self, offer: &Offer) -> Result<(), NotifyError> {
        sender::send_offer(self, offer).await
    }

    pub async fn listen_for_commands(&self) {
        listener::listen_for_commands(self).await;
    }

    pub async fn set_my_commands(&self) -> Result<(), reqwest::Error> {
        let url = format!("https://api.telegram.org/bot{}/setMyCommands", self.bot_token);
        let commands = serde_json::json!({
            "commands": [
                { "command": "ping", "description": "Check connection" },
                { "command": "status", "description": "Show analyzer status" },
                { "command": "help", "description": "Command list" },
                { "command": "last", "description": "Show last great offer" },
                { "command": "top5", "description": "Top 5 offers" },
                { "command": "avg", "description": "Average price" },
                { "command": "config", "description": "Current configuration" },
                { "command": "refresh", "description": "Manual restart" },
                { "command": "uptime", "description": "Service uptime" }
            ]
        });
        self.client.post(&url).json(&commands).send().await?;
        Ok(())
    }

    pub fn spawn_listener(notifier: Arc<TelegramNotifier>) {
        tokio::spawn(async move {
            tracing::info!("▶️ Starting Telegram listener...");
            notifier.listen_for_commands().await;
            tracing::info!("🛑 Telegram listener ended.");
        });
    }

    pub async fn check_and_notify_cheapest_for_model(
        model_name: &str,
        storage: Arc<Mutex<SqliteStorage>>,
        notifier: Arc<TelegramNotifier>,
    ) {
        use tracing::{info, warn};

        info!("🔍 [cheapest] Starting check for model '{}'", model_name);
        let offers = match storage.lock().await.get_all_offers() {
            Ok(o) => o,
            Err(e) => {
                warn!("❌ [cheapest] Failed to get offers for '{}': {:?}", model_name, e);
                return;
            }
        };

        let model_offers: Vec<Offer> = offers
            .into_iter()
            .filter(|o| o.model == model_name && o.price.is_finite())
            .collect();

        info!("📦 [cheapest] Found {} offers for model '{}'", model_offers.len(), model_name);

        if model_offers.is_empty() {
            info!("ℹ️ [cheapest] No offers for '{}'", model_name);
            return;
        }

        let cheapest = model_offers
            .iter()
            .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

        if let Some(cheapest) = cheapest {
            info!(
                "💰 [cheapest] Cheapest offer: {:.2} € | {} | id={}",
                cheapest.price, cheapest.link, cheapest.id
            );

            let should_notify = match storage.lock().await.should_notify(&cheapest.id) {
                Ok(flag) => flag,
                Err(e) => {
                    warn!("❌ [cheapest] Error checking notification status: {:?}", e);
                    false
                }
            };

            if !should_notify {
                info!(
                    "✅ [cheapest] Offer already notified recently: {} € (id={})",
                    cheapest.price, cheapest.id
                );
                return;
            }

            info!(
                "📤 [cheapest] Calling notify() for id={}, price={:.2} €",
                cheapest.id, cheapest.price
            );

            match notifier.notify(cheapest).await {
                Ok(_) => {
                    info!("✅ [cheapest] Notification sent, saving id.");
                    if let Err(e) = storage.lock().await.mark_notified(&cheapest.id) {
                        warn!("❌ [cheapest] Mark notified failed: {:?}", e);
                    }
                }
                Err(e) => {
                    warn!("❌ [cheapest] Error sending notification: {:?}", e);
                }
            }
        } else {
            warn!("⚠️ [cheapest] Failed to find the minimum offer for '{}'", model_name);
        }
    }
}