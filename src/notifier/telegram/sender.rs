use crate::model::{Offer, NotifyError};
use crate::notifier::telegram::TelegramNotifier;
use tracing::{info, warn};
use teloxide::prelude::*;
use teloxide::types::ParseMode;

/// Отправляет простое текстовое сообщение через Telegram
pub async fn send_text(notifier: &TelegramNotifier, text: &str) -> Result<(), NotifyError> {
    match notifier.bot.send_message(notifier.chat_id, text).await {
        Ok(_) => {
            info!("✅ Telegram text sent successfully");
            Ok(())
        }
        Err(e) => {
            warn!("❌ Telegram text error: {:?}", e);
            Err(NotifyError::ApiError(format!("Send failed: {}", e)))
        }
    }
}

/// Отправляет уведомление об оффере с форматированием HTML
pub async fn send_offer(notifier: &TelegramNotifier, offer: &Offer) -> Result<(), NotifyError> {
    let message = format!(
        "💸 <b>Найдена отличная сделка!</b>\n\n\
         📦 <b>Модель:</b> {}\n\
         💰 <b>Цена:</b> {:.2} €\n\
         📍 <b>Локация:</b> {}\n\
         🔗 <a href=\"{}\">Ссылка на объявление</a>",
        html_escape(&offer.model),
        offer.price,
        html_escape(&offer.location),
        html_escape(&offer.link)
    );
    
    info!("📤 Sending Telegram message for offer: {} — {:.2} €", offer.id, offer.price);
    
    match notifier.bot
        .send_message(notifier.chat_id, message)
        .parse_mode(ParseMode::Html)
        .await
    {
        Ok(_) => {
            info!("✅ Telegram notification sent successfully");
            Ok(())
        }
        Err(e) => {
            warn!("❌ Telegram send error: {:?}", e);
            Err(NotifyError::ApiError(format!("Send failed: {}", e)))
        }
    }
}

/// Экранирует HTML специальные символы
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
