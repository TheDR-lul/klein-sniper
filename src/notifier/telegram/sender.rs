use crate::model::{Offer, NotifyError};
use crate::notifier::telegram::TelegramNotifier;
use tracing::{info, warn};
use teloxide::prelude::*;
use teloxide::types::ParseMode;

/// Send simple text message via Telegram
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

/// Send offer notification with HTML formatting
pub async fn send_offer(notifier: &TelegramNotifier, offer: &Offer) -> Result<(), NotifyError> {
    let message = format!(
        "💸 <b>Great deal found!</b>\n\n\
         📦 <b>Model:</b> {}\n\
         💰 <b>Price:</b> {:.2} €\n\
         📍 <b>Location:</b> {}\n\
         🔗 <a href=\"{}\">Link to offer</a>",
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

/// Escape HTML special characters
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
