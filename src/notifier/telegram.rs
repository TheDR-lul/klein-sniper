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
use tokio::time::timeout;

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
        let params = [("chat_id", self.chat_id.to_string()), ("text", text.to_string())];
    
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
            "💸 Найдено выгодное предложение!\n\n📦 Модель: {}\n💰 Цена: {:.2} €\n🔗 Ссылка: {}",
            offer.model, offer.price, offer.link
        );
    
        let params = [
            ("chat_id", self.chat_id.to_string()),
            ("text", message.clone()),
        ];
    
        tracing::info!("📤 Sending Telegram message:\n{}", message);
    
        let response = match timeout(
            Duration::from_secs(10),
            self.client.post(&url).form(&params).send()
        ).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                tracing::error!("❌ Telegram send() failed: {:?}", e);
                return Err(NotifyError::ApiError(format!("Send failed: {}", e)));
            }
            Err(_) => {
                tracing::error!("⏳ Telegram send() timed out");
                return Err(NotifyError::Unreachable);
            }
        };
    
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "unknown".into());
    
        if !status.is_success() {
            tracing::warn!("❌ Telegram API responded [{}]: {}", status, body);
            return Err(NotifyError::Unreachable);
        }
    
        tracing::info!("✅ Telegram response [{}]: {}", status, body);
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
                                "/force_notify" => {
                                    match self.storage.lock().await.get_last_offer() {
                                        Ok(Some(offer)) => {
                                            let _ = self.notify(&offer).await;
                                        }
                                        _ => {
                                            let _ = self.notify_text("❌ Нет последнего оффера для уведомления.").await;
                                        }
                                    }
                                }
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
    tracing::info!("🔍 [cheapest] Старт проверки модели '{}'", model_name);

    let offers = match storage.lock().await.get_all_offers() {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("❌ [cheapest] Не удалось получить офферы для '{}': {:?}", model_name, e);
            return;
        }
    };

    let model_offers: Vec<_> = offers
        .into_iter()
        .filter(|o| o.model == model_name && o.price.is_finite())
        .collect();

    tracing::info!("📦 [cheapest] Найдено {} офферов для модели '{}'", model_offers.len(), model_name);

    if model_offers.is_empty() {
        tracing::info!("ℹ️ [cheapest] Нет офферов для '{}'", model_name);
        return;
    }

    let cheapest = model_offers
        .iter()
        .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

    if let Some(cheapest) = cheapest {
        tracing::info!(
            "💰 [cheapest] Самое дешёвое: {:.2} € | {} | id={}",
            cheapest.price,
            cheapest.link,
            cheapest.id
        );

        let mut map = best_deal_ids.lock().await;

        match map.get(model_name) {
            Some(prev_id) => {
                tracing::info!("📌 [cheapest] Предыдущий id для '{}': {}", model_name, prev_id);

                if prev_id == &cheapest.id {
                    tracing::info!(
                        "✅ [cheapest] Предложение уже уведомлено: {} € (id={})",
                        cheapest.price,
                        cheapest.id
                    );
                    return;
                } else {
                    tracing::info!(
                        "🔁 [cheapest] Обновление! Старое id: {}, новое id: {}",
                        prev_id,
                        cheapest.id
                    );
                }
            }
            None => {
                tracing::info!("🆕 [cheapest] Модель '{}' ещё не была уведомлена.", model_name);
            }
        }

        tracing::info!(
            "📤 [cheapest] Вызываем notify() для id={}, цена={:.2} €",
            cheapest.id,
            cheapest.price
        );

        match notifier.lock().await.notify(cheapest).await {
            Ok(_) => {
                tracing::info!("✅ [cheapest] Уведомление отправлено, сохраняем id.");
                map.insert(model_name.to_string(), cheapest.id.clone());
            }
            Err(e) => {
                tracing::warn!("❌ [cheapest] Ошибка при отправке уведомления: {:?}", e);
            }
        }
    } else {
        tracing::warn!("⚠️ [cheapest] Не удалось найти минимальное предложение для '{}'", model_name);
    }
}