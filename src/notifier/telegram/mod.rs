pub mod sender;
pub mod command_handler;
pub mod statistics;

use crate::model::{NotifyError, Offer};
use crate::storage::SqliteStorage;
use crate::config::AppConfig;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use std::time::Instant;
use teloxide::prelude::*;

#[derive(Clone)]
pub struct TelegramNotifier {
    pub bot: Bot,
    pub chat_id: ChatId,
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
        let bot = Bot::new(bot_token);
        Self {
            bot,
            chat_id: ChatId(chat_id),
            storage,
            config,
            start_time: Instant::now(),
            refresh_notify,
        }
    }

    pub async fn notify_text(&self, text: &str) -> Result<(), NotifyError> {
        sender::send_text(self, text).await
    }

    pub async fn notify(&self, offer: &Offer) -> Result<(), NotifyError> {
        sender::send_offer(self, offer).await
    }

    /// Start bot to listen for commands in separate task
    pub fn spawn_listener(notifier: Arc<TelegramNotifier>) {
        tokio::spawn(async move {
            tracing::info!("▶️ Starting Telegram listener...");
            
            // Set bot menu commands
            if let Err(e) = notifier.set_my_commands().await {
                tracing::warn!("Failed to set bot commands: {:?}", e);
            }
            
            // Start command handler
            command_handler::run_bot(notifier).await;
            
            tracing::info!("🛑 Telegram listener ended.");
        });
    }

    async fn set_my_commands(&self) -> Result<(), teloxide::RequestError> {
        use teloxide::types::BotCommand;
        
        let commands = vec![
            BotCommand::new("ping", "Check connection"),
            BotCommand::new("status", "Show analyzer status"),
            BotCommand::new("help", "Command list"),
            BotCommand::new("settings", "Show settings"),
            BotCommand::new("last", "Show last great offer"),
            BotCommand::new("top5", "Top 5 offers"),
            BotCommand::new("avg", "Average price"),
            BotCommand::new("config", "Current configuration"),
            BotCommand::new("refresh", "Manual restart"),
            BotCommand::new("uptime", "Service uptime"),
            BotCommand::new("dbstats", "Database stats"),
                BotCommand::new("cleardb", "Clear all data"),
                BotCommand::new("minscore", "Set min score"),
                BotCommand::new("maxdist", "Set max distance"),
                BotCommand::new("besttimes", "Best times to check"),
                BotCommand::new("profit", "Profit calculator"),
                BotCommand::new("market", "Market analysis"),
            ];
        
        self.bot.set_my_commands(commands).await?;
        Ok(())
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
