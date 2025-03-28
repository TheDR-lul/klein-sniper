// Telegram bot implementation

use crate::model::Offer;
use reqwest::{Client, Error};

pub struct TelegramNotifier {
    bot_token: String,
    chat_id: i64,
    client: Client,
}

impl TelegramNotifier {
    pub fn new(bot_token: String, chat_id: i64) -> Self {
        let client = Client::new();
        Self {
            bot_token,
            chat_id,
            client,
        }
    }

    pub async fn notify(&self, offer: &Offer) -> Result<(), Error> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let params = [
            ("chat_id", &self.chat_id.to_string()),
            ("text", &format!(
                "💸 Найдено выгодное предложение!\n\n📦 Модель: {}\n💰 Цена: {:.2} €\n🔗 Ссылка: {}",
                offer.model, offer.price, offer.link
            )),
        ];

        let _response = self.client
            .post(&url)
            .form(&params)
            .send()
            .await?;

        Ok(())
    }
}