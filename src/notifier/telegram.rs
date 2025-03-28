use crate::model::{Offer, NotifyError};
use reqwest::Client;
use std::time::Duration;

pub struct TelegramNotifier {
    bot_token: String,
    chat_id: i64,
    client: Client,
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
        }
    }

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
            ("chat_id", &self.chat_id.to_string()),
            ("text", &message),
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
}