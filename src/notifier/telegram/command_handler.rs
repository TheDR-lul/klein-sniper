use crate::notifier::telegram::TelegramNotifier;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tracing::{info, warn};

/// Bot command definitions
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "check connection")]
    Ping,
    #[command(description = "analyzer status")]
    Status,
    #[command(description = "command list")]
    Help,
    #[command(description = "last great deal")]
    Last,
    #[command(description = "top-5 offers")]
    Top5,
    #[command(description = "average prices")]
    Avg,
    #[command(description = "current configuration")]
    Config,
    #[command(description = "manual restart")]
    Refresh,
    #[command(description = "service uptime")]
    Uptime,
    #[command(description = "force send notification")]
    ForceNotify,
    #[command(description = "database statistics")]
    DbStats,
}

/// Start bot and handle commands
pub async fn run_bot(notifier: Arc<TelegramNotifier>) {
    let notifier_clone = notifier.clone();
    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
            let notifier = notifier_clone.clone();
            async move {
                handle_command(bot, msg, cmd, notifier).await;
                Result::<(), ()>::Ok(())
            }
        });

    Dispatcher::builder(notifier.bot.clone(), handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

/// Command handler
async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    notifier: Arc<TelegramNotifier>,
) {
    let chat_id = msg.chat.id;
    
    info!("Received command: {:?} from chat {}", cmd, chat_id);

    let response = match cmd {
        Command::Ping => {
            "✅ I'm online!".to_string()
        }
        Command::Status => {
            "📊 Analyzer is running. Waiting for next check.".to_string()
        }
        Command::Help => {
            Command::descriptions().to_string()
        }
        Command::Refresh => {
            info!("/refresh command received, triggering refresh...");
            notifier.refresh_notify.notify_one();
            "🔄 Manual refresh initiated.".to_string()
        }
        Command::Uptime => {
            let uptime = notifier.start_time.elapsed();
            format!(
                "⏱ Uptime: {:02}:{:02}:{:02}",
                uptime.as_secs() / 3600,
                (uptime.as_secs() % 3600) / 60,
                uptime.as_secs() % 60
            )
        }
        Command::Last => {
            match notifier.storage.lock().await.get_last_offer() {
                Ok(Some(offer)) => {
                    format!(
                        "🕵️ Last offer:\n📦 {}\n💰 {:.2} €\n📍 {}\n🔗 {}",
                        offer.title, offer.price, offer.location, offer.link
                    )
                }
                Ok(None) => "📭 No offers in database.".to_string(),
                Err(e) => {
                    warn!("/last error: {:?}", e);
                    format!("❌ Error: {:?}", e)
                }
            }
        }
        Command::Top5 => {
            match notifier.storage.lock().await.get_top5_offers() {
                Ok(offers) if !offers.is_empty() => {
                    let mut msg = String::from("🏆 Top-5 best offers:\n");
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
                    msg
                }
                Ok(_) => "📭 No offers in database.".to_string(),
                Err(e) => {
                    warn!("/top5 error: {:?}", e);
                    format!("❌ Error: {:?}", e)
                }
            }
        }
        Command::Avg => {
            match notifier.storage.lock().await.get_average_prices() {
                Ok(prices) if !prices.is_empty() => {
                    let mut msg = String::from("📊 Average prices by model:\n");
                    for (model, price) in prices {
                        msg.push_str(&format!("🔹 {} — {:.2} €\n", model, price));
                    }
                    msg
                }
                Ok(_) => "📭 No model statistics available.".to_string(),
                Err(e) => {
                    warn!("/avg error: {:?}", e);
                    format!("❌ Error: {:?}", e)
                }
            }
        }
        Command::Config => {
            if notifier.config.models.is_empty() {
                "⚠️ No models loaded in configuration.".to_string()
            } else {
                let mut msg = String::from("⚙️ Loaded models:\n");
                for model in &notifier.config.models {
                    msg.push_str(&format!("🔸 {} [{}]\n", model.query, model.category_id));
                }
                msg
            }
        }
        Command::ForceNotify => {
            match notifier.storage.lock().await.get_last_offer() {
                Ok(Some(offer)) => {
                    match notifier.notify(&offer).await {
                        Ok(_) => {
                            let _ = notifier.storage.lock().await.mark_notified(&offer.id);
                            "✅ Notification sent!".to_string()
                        }
                        Err(e) => {
                            warn!("/force_notify send error: {:?}", e);
                            format!("❌ Send error: {:?}", e)
                        }
                    }
                }
                _ => "❌ No last offer available for notification.".to_string(),
            }
        }
        Command::DbStats => {
            match notifier.storage.lock().await.get_database_stats() {
                Ok(stats) => {
                    let size_mb = stats.db_size_bytes as f64 / 1024.0 / 1024.0;
                    format!(
                        "💾 <b>Database Statistics</b>\n\n\
                         📦 Offers: {}\n\
                         ✅ Notified: {}\n\
                         📊 Stats records: {}\n\
                         💿 DB size: {:.2} MB",
                        stats.offer_count,
                        stats.notified_count,
                        stats.stats_count,
                        size_mb
                    )
                }
                Err(e) => {
                    warn!("/dbstats error: {:?}", e);
                    format!("❌ Error: {:?}", e)
                }
            }
        }
    };

    if let Err(e) = bot.send_message(chat_id, response).await {
        warn!("Failed to send response: {:?}", e);
    }
}
