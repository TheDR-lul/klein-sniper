use crate::model::{NotifyError, Offer};
use crate::storage::SqliteStorage;
use crate::config::AppConfig;
use reqwest::{Client, Error};
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify};
use tokio::time::sleep;
use tracing::{info, warn};
use std::collections::HashMap;

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

pub struct TelegramNotifier {
    bot_token: String,
    chat_id: i64,
    client: Client,
    offset: i64,
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
            .expect("❗ Не удалось создать HTTP клиент");

        Self {
            bot_token,
            chat_id,
            client,
            offset: 0,
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

        self.client.post(&url).form(&params).send().await?;
        Ok(())
    }

    pub async fn notify(&self, offer: &Offer) -> Result<(), NotifyError> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
    
        let message = format!(
            "💸 Найдено выгодное предложение!\n\n📦 Модель: {}\n💰 Цена: {:.2} €\n🔗 Ссылка: {}",
            offer.model, offer.price, offer.link
        );
    
        let params = [
            ("chat_id", self.chat_id.to_string()),
            ("text", message.clone()),
        ];
    
        tracing::info!("📤 Sending Telegram notification: {}", message);
    
        let response = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| NotifyError::ApiError(format!("Ошибка запроса: {}", e)))?;
    
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "unknown".into());
    
        if !status.is_success() {
            tracing::warn!("❌ Telegram error [{}]: {}", status, body);
            return Err(NotifyError::Unreachable);
        }
    
        tracing::info!("✅ Telegram success [{}]: {}", status, body); // 🔍 лог даже при успехе
    
        Ok(())
    }    

    pub async fn listen_for_commands(&mut self) {
        let url = format!("https://api.telegram.org/bot{}/getUpdates", self.bot_token);
        loop {
            let response = self.client.get(&url).query(&[("offset", self.offset + 1)]).send().await;
            if let Ok(resp) = response {
                if let Ok(api_response) = resp.json::<TelegramApiResponse>().await {
                    for update in api_response.result {
                        if let Some(text) = update.message.text.as_deref() {
                            match text {
                                "/ping" => { let _ = self.notify_text("✅ Я на связи!").await; },
                                "/status" => { let _ = self.notify_text("📊 Анализатор работает. Ждём следующую проверку.").await; },
                                "/help" => {
                                    let _ = self.notify_text("📋 Доступные команды:\n/ping — проверить подключение\n/status — статус анализатора\n/help — список команд\n/last — последнее выгодное предложение\n/top5 — топ 5 предложений\n/avg — средняя цена\n/config — текущая конфигурация\n/refresh — ручной перезапуск\n/uptime — аптайм сервиса").await;
                                },
                                "/refresh" => {
                                    self.refresh_notify.notify_one();
                                    let _ = self.notify_text("🔄 Принудительный перезапуск запущен.").await;

                                },
                                "/uptime" => {
                                    let uptime = self.start_time.elapsed();
                                    let hours = uptime.as_secs() / 3600;
                                    let minutes = (uptime.as_secs() % 3600) / 60;
                                    let seconds = uptime.as_secs() % 60;
                                    let msg = format!("⏱ Аптайм: {:02}:{:02}:{:02}", hours, minutes, seconds);
                                    let _ = self.notify_text(&msg).await;
                                },
                                "/last" => {
                                    match self.storage.lock().await.get_last_offer() {
                                        Ok(Some(offer)) => {
                                            let msg = format!(
                                                "🕵️ Последнее предложение:\n📦 {}\n💰 {:.2} €\n📍 {}\n🔗 {}",
                                                offer.title, offer.price, offer.location, offer.link
                                            );
                                            let _ = self.notify_text(&msg).await;
                                        }
                                        Ok(None) => {
                                            let _ = self.notify_text("📭 Нет предложений в базе.").await;
                                        }
                                        Err(e) => {
                                            let _ = self.notify_text(&format!("❌ Ошибка: {:?}", e)).await;
                                        }
                                    }
                                },
                                "/top5" => {
                                    match self.storage.lock().await.get_top5_offers() {
                                        Ok(offers) if !offers.is_empty() => {
                                            let mut msg = String::from("🏆 Топ-5 выгодных предложений:\n");
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
                                            let _ = self.notify_text(&msg).await;
                                        }
                                        Ok(_) => {
                                            let _ = self.notify_text("📭 Нет предложений в базе.").await;
                                        }
                                        Err(e) => {
                                            let _ = self.notify_text(&format!("❌ Ошибка: {:?}", e)).await;
                                        }
                                    }
                                },
                                "/avg" => {
                                    match self.storage.lock().await.get_average_prices() {
                                        Ok(prices) if !prices.is_empty() => {
                                            let mut msg = String::from("📊 Средние цены по моделям:\n");
                                            for (model, price) in prices {
                                                msg.push_str(&format!("🔹 {} — {:.2} €\n", model, price));
                                            }
                                            let _ = self.notify_text(&msg).await;
                                        }
                                        Ok(_) => {
                                            let _ = self.notify_text("📭 Нет статистики по моделям.").await;
                                        }
                                        Err(e) => {
                                            let _ = self.notify_text(&format!("❌ Ошибка: {:?}", e)).await;
                                        }
                                    }
                                },
                                "/config" => {
                                    if self.config.models.is_empty() {
                                        let _ = self.notify_text("⚠️ Нет загруженных моделей в конфигурации.").await;
                                    } else {
                                        let mut msg = String::from("⚙️ Загруженные модели:\n");
                                        for model in &self.config.models {
                                            msg.push_str(&format!("🔸 {} [{}]\n", model.query, model.category_id));
                                        }
                                        let _ = self.notify_text(&msg).await;
                                    }
                                },
                                _ => {
                                    let _ = self.notify_text("🤖 Неизвестная команда. Введите /help для списка.").await;
                                }
                            }
                        }
                        self.offset = update.update_id;
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
                { "command": "ping", "description": "Проверить подключение" },
                { "command": "status", "description": "Показать статус анализатора" },
                { "command": "help", "description": "Список доступных команд" },
                { "command": "last", "description": "Показать последнее выгодное предложение" },
                { "command": "top5", "description": "Топ 5 предложений по выгоде" },
                { "command": "avg", "description": "Средняя цена по модели" },
                { "command": "config", "description": "Текущая конфигурация" },
                { "command": "refresh", "description": "Ручной запуск анализа" },
                { "command": "uptime", "description": "Аптайм сканера" }
            ]
        });
        self.client.post(&url).json(&commands).send().await?;
        Ok(())
    }
}

/// 💡 Проверка самой дешёвой по конкретной модели
pub async fn check_and_notify_cheapest_for_model(
    model_name: &str,
    storage: Arc<Mutex<SqliteStorage>>,
    notifier: Arc<Mutex<TelegramNotifier>>,
    best_deal_ids: Arc<Mutex<HashMap<String, String>>>,
) {
    let offers = match storage.lock().await.get_all_offers() {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("❌ Failed to get all offers for '{}': {:?}", model_name, e);
            return;
        }
    };

    let model_offers: Vec<_> = offers
        .into_iter()
        .filter(|o| o.model == model_name && o.price.is_finite())
        .collect();

    if model_offers.is_empty() {
        tracing::info!("ℹ️ No offers found for model '{}'", model_name);
        return;
    }

    let cheapest = model_offers
        .iter()
        .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

    if let Some(cheapest) = cheapest {
        let mut map = best_deal_ids.lock().await;

        match map.get(model_name) {
            Some(prev_id) if prev_id == &cheapest.id => {
                tracing::info!("✅ Cheapest unchanged for '{}': {} €", model_name, cheapest.price);
                return;
            }
            _ => {
                tracing::info!("💸 New cheapest for '{}': {} € | {}", model_name, cheapest.price, cheapest.link);

                if let Err(e) = notifier.lock().await.notify(cheapest).await {
                    tracing::warn!("Telegram send error (cheapest): {e:?}");
                } else {
                    map.insert(model_name.to_string(), cheapest.id.clone());
                }
            }
        }
    } else {
        tracing::warn!("⚠️ Failed to find cheapest offer for '{}'", model_name);
    }
}
