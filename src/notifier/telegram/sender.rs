// notifier/telegram/sender.rs

use crate::model::{Offer, NotifyError};
use crate::notifier::telegram::TelegramNotifier;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

/// Sends a simple text message via Telegram.
pub async fn send_text(notifier: &TelegramNotifier, text: &str) -> Result<(), reqwest::Error> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", notifier.bot_token);
    let params = [
        ("chat_id", notifier.chat_id.to_string()),
        ("text", text.to_string()),
    ];
    let response = notifier.client.post(&url).form(&params).send().await?;
    let status = response.status();
    let body = response.text().await.unwrap_or_else(|_| "unknown".into());
    if !status.is_success() {
        warn!("❌ Telegram text error [{}]: {}", status, body);
    } else {
        info!("✅ Telegram text sent [{}]: {}", status, body);
    }
    Ok(())
}

/// Sends a notification message for an offer.
pub async fn send_offer(notifier: &TelegramNotifier, offer: &Offer) -> Result<(), NotifyError> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", notifier.bot_token);
    let message = format!(
        "💸 Found a great deal!\n\n📦 Model: {}\n💰 Price: {:.2} €\n🔗 Link: {}",
        offer.model, offer.price, offer.link
    );
    info!("📤 Sending Telegram message:\n{}", message);
    let response = match timeout(
        Duration::from_secs(10),
        notifier.client
            .post(&url)
            .form(&[("chat_id", notifier.chat_id.to_string()), ("text", message.clone())])
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
