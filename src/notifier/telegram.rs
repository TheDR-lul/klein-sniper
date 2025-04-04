// src/notifier/telegram.rs

use crate::model::{NotifyError, Offer};
use crate::storage::SqliteStorage;
use crate::config::AppConfig;
use reqwest::{Client, Error};
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use tracing::{info, warn};
use tokio::sync::Notify;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::Mutex;
use std::collections::HashMap; // added import for HashMap

#[derive(Debug, Deserialize)]
struct TelegramApiResponse {
    result: Vec<TelegramUpdate>,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: TelegramMessage,
}

#[derive(Debug, Deserialize)]
struct TelegramMessage {
    chat: TelegramChat,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
}

#[derive(Clone)]
pub struct TelegramNotifier {
    bot_token: String,
    chat_id: i64,
    client: Client,
    offset: Arc<AtomicI64>, // changed type to Arc<AtomicI64>
    storage: Arc<Mutex<SqliteStorage>>,
    config: Arc<AppConfig>,
    start_time: Instant,
    refresh_notify: Arc<Notify>,
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
            .timeout(Duration::from_secs(10))
            .build()
            .expect("❗ Failed to create HTTP client");

        Self {
            bot_token,
            chat_id,
            client,
            offset: Arc::new(AtomicI64::new(0)), // create atomic value via Arc
            storage,
            config,
            start_time: Instant::now(),
            refresh_notify,
        }
    }

    pub async fn notify_text(&self, text: &str) -> Result<(), Error> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let params = [
            ("chat_id", self.chat_id.to_string()),
            ("text", text.to_string()),
        ];

        let response = self.client.post(&url).form(&params).send().await?;
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "unknown".into());

        if !status.is_success() {
            warn!("❌ Telegram text error [{}]: {}", status, body);
        } else {
            info!("✅ Telegram text sent [{}]: {}", status, body);
        }

        Ok(())
    }

    pub async fn notify(&self, offer: &Offer) -> Result<(), NotifyError> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);

        let message = format!(
            "💸 Found a great deal!\n\n📦 Model: {}\n💰 Price: {:.2} €\n🔗 Link: {}",
            offer.model, offer.price, offer.link
        );

        info!("📤 Sending Telegram message:\n{}", message);

        let response = match timeout(
            Duration::from_secs(10),
            self.client
                .post(&url)
                .form(&[
                    ("chat_id", self.chat_id.to_string()),
                    ("text", message.clone()),
                ])
                .send(),
        )
        .await
        {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                warn!("❌ Telegram send() failed: {:?}", e);
                return Err(NotifyError::ApiError(format!("Send failed: {}", e)));
            }
            Err(_) => {
                warn!("⏳ Telegram send() timed out");
                return Err(NotifyError::Unreachable);
            }
        };

        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "unknown".into());

        if !status.is_success() {
            warn!("❌ Telegram API responded [{}]: {}", status, body);
            return Err(NotifyError::Unreachable);
        }

        info!("✅ Telegram response [{}]: {}", status, body);
        Ok(())
    }

    pub async fn listen_for_commands(&self) {
        let url = format!("https://api.telegram.org/bot{}/getUpdates", self.bot_token);
        loop {
            let current_offset = self.offset.load(Ordering::SeqCst);
            let response = self
                .client
                .get(&url)
                .query(&[("offset", (current_offset + 1).to_string())])
                .send()
                .await;
            if let Ok(resp) = response {
                if let Ok(api_response) = resp.json::<TelegramApiResponse>().await {
                    for update in api_response.result {
                        if let Some(text) = update.message.text.as_deref() {
                            match text {
                                "/ping" => {
                                    if let Err(e) = self.notify_text("✅ I am online!").await {
                                        warn!("❌ /ping error: {e:?}");
                                    }
                                }
                                "/status" => {
                                    if let Err(e) = self
                                        .notify_text("📊 Analyzer is running. Waiting for the next check.")
                                        .await
                                    {
                                        warn!("❌ /status error: {e:?}");
                                    }
                                }
                                "/help" => {
                                    let help_msg = "📋 Available commands:\n\
                                        /ping — check connection\n\
                                        /status — analyzer status\n\
                                        /help — command list\n\
                                        /last — last great deal\n\
                                        /top5 — top 5 offers\n\
                                        /avg — average price\n\
                                        /config — current configuration\n\
                                        /refresh — manual restart\n\
                                        /uptime — service uptime";
                                    if let Err(e) = self.notify_text(help_msg).await {
                                        warn!("❌ /help error: {e:?}");
                                    }
                                }
                                "/refresh" => {
                                    info!("📣 /refresh command received, triggering refresh...");
                                    self.refresh_notify.notify_one();
                                    if let Err(e) = self
                                        .notify_text("🔄 Forced restart initiated.")
                                        .await
                                    {
                                        warn!("❌ /refresh error: {e:?}");
                                    }
                                }
                                "/uptime" => {
                                    let uptime = self.start_time.elapsed();
                                    let msg = format!(
                                        "⏱ Uptime: {:02}:{:02}:{:02}",
                                        uptime.as_secs() / 3600,
                                        (uptime.as_secs() % 3600) / 60,
                                        uptime.as_secs() % 60
                                    );
                                    if let Err(e) = self.notify_text(&msg).await {
                                        warn!("❌ /uptime error: {e:?}");
                                    }
                                }
                                "/last" => {
                                    match self.storage.lock().await.get_last_offer() {
                                        Ok(Some(offer)) => {
                                            let msg = format!(
                                                "🕵️ Last offer:\n📦 {}\n💰 {:.2} €\n📍 {}\n🔗 {}",
                                                offer.title, offer.price, offer.location, offer.link
                                            );
                                            if let Err(e) = self.notify_text(&msg).await {
                                                warn!("❌ /last notify error: {e:?}");
                                            }
                                        }
                                        Ok(None) => {
                                            if let Err(e) =
                                                self.notify_text("📭 No offers in the database.").await
                                            {
                                                warn!("❌ /last empty notify error: {e:?}");
                                            }
                                        }
                                        Err(e) => {
                                            if let Err(send_err) = self
                                                .notify_text(&format!("❌ Error: {:?}", e))
                                                .await
                                            {
                                                warn!("❌ /last send error: {send_err:?}");
                                            }
                                        }
                                    }
                                }
                                "/top5" => {
                                    match self.storage.lock().await.get_top5_offers() {
                                        Ok(offers) if !offers.is_empty() => {
                                            let mut msg =
                                                String::from("🏆 Top-5 best offers:\n");
                                            for (i, offer) in offers.iter().enumerate() {
                                                msg.push_str(&format!(
                                                    "{}. {} — {:.2} €\n📍 {}\n🔗 {}\n\n",
                                                    i + 1,
                                                    offer.title,
                                                    offer.price,
                                                    offer.location,
                                                    offer.link
                                                ));
                                            }
                                            if let Err(e) = self.notify_text(&msg).await {
                                                warn!("❌ /top5 notify error: {e:?}");
                                            }
                                        }
                                        Ok(_) => {
                                            if let Err(e) =
                                                self.notify_text("📭 No offers in the database.").await
                                            {
                                                warn!("❌ /top5 empty notify error: {e:?}");
                                            }
                                        }
                                        Err(e) => {
                                            if let Err(send_err) = self
                                                .notify_text(&format!("❌ Error: {:?}", e))
                                                .await
                                            {
                                                warn!("❌ /top5 send error: {send_err:?}");
                                            }
                                        }
                                    }
                                }
                                "/avg" => {
                                    match self.storage.lock().await.get_average_prices() {
                                        Ok(prices) if !prices.is_empty() => {
                                            let mut msg =
                                                String::from("📊 Average prices by model:\n");
                                            for (model, price) in prices {
                                                msg.push_str(&format!("🔹 {} — {:.2} €\n", model, price));
                                            }
                                            if let Err(e) = self.notify_text(&msg).await {
                                                warn!("❌ /avg notify error: {e:?}");
                                            }
                                        }
                                        Ok(_) => {
                                            if let Err(e) =
                                                self.notify_text("📭 No model statistics available.").await
                                            {
                                                warn!("❌ /avg empty notify error: {e:?}");
                                            }
                                        }
                                        Err(e) => {
                                            if let Err(send_err) = self
                                                .notify_text(&format!("❌ Error: {:?}", e))
                                                .await
                                            {
                                                warn!("❌ /avg send error: {send_err:?}");
                                            }
                                        }
                                    }
                                }
                                "/config" => {
                                    if self.config.models.is_empty() {
                                        if let Err(e) = self
                                            .notify_text("⚠️ No models loaded in the configuration.")
                                            .await
                                        {
                                            warn!("❌ /config empty error: {e:?}");
                                        }
                                    } else {
                                        let mut msg = String::from("⚙️ Loaded models:\n");
                                        for model in &self.config.models {
                                            msg.push_str(&format!("🔸 {} [{}]\n", model.query, model.category_id));
                                        }
                                        if let Err(e) = self.notify_text(&msg).await {
                                            warn!("❌ /config notify error: {e:?}");
                                        }
                                    }
                                }
                                "/force_notify" => {
                                    match self.storage.lock().await.get_last_offer() {
                                        Ok(Some(offer)) => {
                                            match self.notify(&offer).await {
                                                Ok(_) => {
                                                    let _ = self.storage.lock().await.mark_notified(&offer.id);
                                                }
                                                Err(e) => {
                                                    if let Err(se) = self
                                                        .notify_text(&format!("❌ Error sending: {:?}", e))
                                                        .await
                                                    {
                                                        warn!("❌ /force_notify send error: {se:?}");
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            if let Err(e) = self
                                                .notify_text("❌ No last offer available for notification.")
                                                .await
                                            {
                                                warn!("❌ /force_notify notify error: {e:?}");
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    if let Err(e) = self
                                        .notify_text("🤖 Unknown command. Type /help for a list of commands.")
                                        .await
                                    {
                                        warn!("❌ unknown command notify error: {e:?}");
                                    }
                                }
                            }
                        }
                        self.offset.store(update.update_id + 1, Ordering::SeqCst);
                    }
                }
            }
            sleep(Duration::from_secs(5)).await;
        }
    }

    pub async fn set_my_commands(&self) -> Result<(), Error> {
        let url = format!("https://api.telegram.org/bot{}/setMyCommands", self.bot_token);
        let commands = serde_json::json!({
            "commands": [
                { "command": "ping", "description": "Check connection" },
                { "command": "status", "description": "Show analyzer status" },
                { "command": "help", "description": "List available commands" },
                { "command": "last", "description": "Show last great offer" },
                { "command": "top5", "description": "Top 5 best offers" },
                { "command": "avg", "description": "Average price by model" },
                { "command": "config", "description": "Current configuration" },
                { "command": "refresh", "description": "Manual analysis restart" },
                { "command": "uptime", "description": "Scanner uptime" }
            ]
        });
        self.client.post(&url).json(&commands).send().await?;
        Ok(())
    }
}

/// Check for the cheapest offer for a specific model and notify if needed
pub async fn check_and_notify_cheapest_for_model(
    model_name: &str,
    storage: Arc<Mutex<SqliteStorage>>,
    notifier: Arc<TelegramNotifier>,
    best_deal_ids: Arc<Mutex<HashMap<String, String>>>,
) {
    info!("🔍 [cheapest] Starting check for model '{}'", model_name);

    let offers = match storage.lock().await.get_all_offers() {
        Ok(o) => o,
        Err(e) => {
            warn!("❌ [cheapest] Failed to get offers for '{}': {:?}", model_name, e);
            return;
        }
    };

    let model_offers: Vec<_> = offers
        .into_iter()
        .filter(|o| o.model == model_name && o.price.is_finite())
        .collect();

    info!(
        "📦 [cheapest] Found {} offers for model '{}'",
        model_offers.len(),
        model_name
    );

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

        // Сначала проверяем, можно ли уведомлять (если прошло 24 часа или записи нет)
        match storage.lock().await.should_notify(&cheapest.id) {
            Ok(false) => {
                info!("✅ [cheapest] Already notified within the period: {} (id={})", cheapest.price, cheapest.id);
                return;
            }
            Ok(true) => {} // можно уведомлять
            Err(e) => {
                warn!("❌ [cheapest] Notify check failed: {:?}", e);
                return;
            }
        }

        let mut map = best_deal_ids.lock().await;
        match map.get(model_name) {
            Some(prev_id) => {
                info!("📌 [cheapest] Previous id for '{}': {}", model_name, prev_id);
                if prev_id == &cheapest.id {
                    info!(
                        "✅ [cheapest] Offer already notified: {} € (id={})",
                        cheapest.price, cheapest.id
                    );
                    return;
                } else {
                    info!(
                        "🔁 [cheapest] Updating! Old id: {}, new id: {}",
                        prev_id, cheapest.id
                    );
                }
            }
            None => {
                info!("🆕 [cheapest] Model '{}' has not been notified yet.", model_name);
            }
        }

        info!(
            "📤 [cheapest] Calling notify() for id={}, price={:.2} €",
            cheapest.id, cheapest.price
        );

        match notifier.notify(cheapest).await {
            Ok(_) => {
                info!("✅ [cheapest] Notification sent, saving id.");
                map.insert(model_name.to_string(), cheapest.id.clone());
                if let Err(e) = storage.lock().await.mark_notified(&cheapest.id) {
                    warn!("❌ [cheapest] Mark notified failed: {:?}", e);
                }
            }
            Err(e) => {
                warn!("❌ [cheapest] Error sending notification: {:?}", e);
            }
        }
    } else {
        warn!(
            "⚠️ [cheapest] Failed to find the minimum offer for '{}'",
            model_name
        );
    }
}

pub fn spawn_listener(notifier: Arc<TelegramNotifier>) {
    tokio::spawn(async move {
        info!("▶️ Starting Telegram listener...");
        notifier.listen_for_commands().await;
        info!("🛑 Telegram listener ended.");
    });
}