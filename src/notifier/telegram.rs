use crate::model::{NotifyError, Offer};
use reqwest::{Client, Error};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

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
}

impl TelegramNotifier {
    pub fn new(bot_token: String, chat_id: i64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("❌ Не удалось создать HTTP клиент");

        Self {
            bot_token,
            chat_id,
            client,
            offset: 0,
        }
    }

    /// Уведомление произвольным текстом (например, при запуске)
    pub async fn notify_text(&self, text: &str) -> Result<(), Error> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);

        let text = text.to_string();
        let params = [
            ("chat_id", self.chat_id.to_string()),
            ("text", text),
        ];

        self.client.post(&url).form(&params).send().await?;

        Ok(())
    }

    /// Уведомление по конкретному предложению
    pub async fn notify(&self, offer: &Offer) -> Result<(), NotifyError> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let message = format!(
            "💸 Найдено выгодное предложение!\n\n📦 Модель: {}\n💰 Цена: {:.2} €\n🔗 Ссылка: {}",
            offer.model, offer.price, offer.link
        );

        let params = [
            ("chat_id", self.chat_id.to_string()),
            ("text", message),
        ];

        let response = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| NotifyError::ApiError(format!("Ошибка запроса: {}", e)))?;

        if !response.status().is_success() {
            return Err(NotifyError::Unreachable);
        }

        Ok(())
    }

    /// Слушает команды от пользователя
    pub async fn listen_for_commands(&mut self) {
        let url = format!("https://api.telegram.org/bot{}/getUpdates", self.bot_token);

        loop {
            let response = self
                .client
                .get(&url)
                .query(&[("offset", self.offset + 1)])
                .send()
                .await;

            if let Ok(resp) = response {
                if let Ok(api_response) = resp.json::<TelegramApiResponse>().await {
                    for update in api_response.result {
                        if let Some(text) = update.message.text.as_deref() {
                            match text {
                                "/ping" => {
                                    let _ = self.notify_text("✅ Я на связи!").await;
                                }
                                "/status" => {
                                    let _ = self
                                        .notify_text("📊 Анализатор работает. Ждём следующую проверку.")
                                        .await;
                                }
                                _ => {
                                    let _ = self.notify_text("🤖 Неизвестная команда.").await;
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
}