// notifier/telegram/listener.rs

use crate::notifier::telegram::command_handler::handle_command;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use crate::notifier::telegram::TelegramNotifier;

#[derive(Debug, Deserialize)]
struct TelegramApiResponse {
    result: Vec<TelegramUpdate>,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
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

/// Polls for Telegram updates and processes incoming commands.
pub async fn listen_for_commands(notifier: &TelegramNotifier) {
    let url = format!("https://api.telegram.org/bot{}/getUpdates", notifier.bot_token);
    loop {
        let current_offset = notifier.offset.load(std::sync::atomic::Ordering::SeqCst);
        let response = notifier.client.get(&url)
            .query(&[("offset", (current_offset + 1).to_string())])
            .send()
            .await;
        if let Ok(resp) = response {
            if let Ok(api_response) = resp.json::<TelegramApiResponse>().await {
                for update in api_response.result {
                    if let Some(text) = update.message.as_ref().and_then(|m| m.text.as_deref()) {
                        // Process the command using the command handler.
                        handle_command(text, notifier).await;
                    }
                    notifier.offset.store(update.update_id + 1, std::sync::atomic::Ordering::SeqCst);
                }
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
}