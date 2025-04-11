// notifier/telegram/command_handler.rs

use crate::notifier::telegram::TelegramNotifier;
use tracing::{info, warn};

/// Handles an incoming command and triggers the corresponding action.
pub async fn handle_command(command_text: &str, notifier: &TelegramNotifier) {
    info!("Handling command: {}", command_text);
    match command_text {
        "/ping" => {
            if let Err(e) = notifier.notify_text("✅ I am online!").await {
                warn!("/ping error: {:?}", e);
            }
        },
        "/status" => {
            if let Err(e) = notifier.notify_text("📊 Analyzer is running. Waiting for the next check.").await {
                warn!("/status error: {:?}", e);
            }
        },
        "/help" => {
            let help_msg = "📋 Available commands:\n\
                /ping — check connection\n\
                /status — analyzer status\n\
                /help — command list\n\
                /last — last great deal\n\
                /top5 — top 5 offers\n\
                /avg — average price\n\
                /config — current configuration\n\
                /refresh — manual restart\n\
                /uptime — service uptime";
            if let Err(e) = notifier.notify_text(help_msg).await {
                warn!("/help error: {:?}", e);
            }
        },
        "/refresh" => {
            info!("/refresh command received, triggering refresh...");
            notifier.refresh_notify.notify_one();
            if let Err(e) = notifier.notify_text("🔄 Forced restart initiated.").await {
                warn!("/refresh error: {:?}", e);
            }
        },
        "/uptime" => {
            let uptime = notifier.start_time.elapsed();
            let msg = format!(
                "⏱ Uptime: {:02}:{:02}:{:02}",
                uptime.as_secs() / 3600,
                (uptime.as_secs() % 3600) / 60,
                uptime.as_secs() % 60
            );
            if let Err(e) = notifier.notify_text(&msg).await {
                warn!("/uptime error: {:?}", e);
            }
        },
        "/last" => {
            match notifier.storage.lock().await.get_last_offer() {
                Ok(Some(offer)) => {
                    let msg = format!(
                        "🕵️ Last offer:\n📦 {}\n💰 {:.2} €\n📍 {}\n🔗 {}",
                        offer.title, offer.price, offer.location, offer.link
                    );
                    if let Err(e) = notifier.notify_text(&msg).await {
                        warn!("/last notify error: {:?}", e);
                    }
                },
                Ok(None) => {
                    if let Err(e) = notifier.notify_text("📭 No offers in the database.").await {
                        warn!("/last empty notify error: {:?}", e);
                    }
                },
                Err(e) => {
                    if let Err(send_err) = notifier.notify_text(&format!("❌ Error: {:?}", e)).await {
                        warn!("/last send error: {:?}", send_err);
                    }
                }
            }
        },
        "/top5" => {
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
                    if let Err(e) = notifier.notify_text(&msg).await {
                        warn!("/top5 notify error: {:?}", e);
                    }
                },
                Ok(_) => {
                    if let Err(e) = notifier.notify_text("📭 No offers in the database.").await {
                        warn!("/top5 empty notify error: {:?}", e);
                    }
                },
                Err(e) => {
                    if let Err(send_err) = notifier.notify_text(&format!("❌ Error: {:?}", e)).await {
                        warn!("/top5 send error: {:?}", send_err);
                    }
                }
            }
        },
        "/avg" => {
            match notifier.storage.lock().await.get_average_prices() {
                Ok(prices) if !prices.is_empty() => {
                    let mut msg = String::from("📊 Average prices by model:\n");
                    for (model, price) in prices {
                        msg.push_str(&format!("🔹 {} — {:.2} €\n", model, price));
                    }
                    if let Err(e) = notifier.notify_text(&msg).await {
                        warn!("/avg notify error: {:?}", e);
                    }
                },
                Ok(_) => {
                    if let Err(e) = notifier.notify_text("📭 No model statistics available.").await {
                        warn!("/avg empty notify error: {:?}", e);
                    }
                },
                Err(e) => {
                    if let Err(send_err) = notifier.notify_text(&format!("❌ Error: {:?}", e)).await {
                        warn!("/avg send error: {:?}", send_err);
                    }
                }
            }
        },
        "/config" => {
            if notifier.config.models.is_empty() {
                if let Err(e) = notifier.notify_text("⚠️ No models loaded in the configuration.").await {
                    warn!("/config empty error: {:?}", e);
                }
            } else {
                let mut msg = String::from("⚙️ Loaded models:\n");
                for model in &notifier.config.models {
                    msg.push_str(&format!("🔸 {} [{}]\n", model.query, model.category_id));
                }
                if let Err(e) = notifier.notify_text(&msg).await {
                    warn!("/config notify error: {:?}", e);
                }
            }
        },
        "/force_notify" => {
            match notifier.storage.lock().await.get_last_offer() {
                Ok(Some(offer)) => {
                    match notifier.notify(&offer).await {
                        Ok(_) => {
                            let _ = notifier.storage.lock().await.mark_notified(&offer.id);
                        },
                        Err(e) => {
                            if let Err(se) = notifier.notify_text(&format!("❌ Error sending: {:?}", e)).await {
                                warn!("/force_notify send error: {:?}", se);
                            }
                        }
                    }
                },
                _ => {
                    if let Err(e) = notifier.notify_text("❌ No last offer available for notification.").await {
                        warn!("/force_notify notify error: {:?}", e);
                    }
                }
            }
        },
        _ => {
            if let Err(e) = notifier.notify_text("🤖 Unknown command. Type /help for a list of commands.").await {
                warn!("Unknown command notify error: {:?}", e);
            }
        }
    }
}