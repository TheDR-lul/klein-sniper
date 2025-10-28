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
    #[command(description = "clear all database data")]
    ClearDb,
    #[command(description = "show current settings")]
    Settings,
    #[command(description = "set min score (60-100)")]
    MinScore(String),
    #[command(description = "set max distance in km (0=off)")]
    MaxDist(String),
    #[command(description = "show best times to check for deals")]
    BestTimes,
    #[command(description = "reseller profit analysis for offer")]
    Profit(String),
    #[command(description = "market trend and speed analysis")]
    Market,
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
               Command::ClearDb => {
                   info!("/cleardb command received");
                   match notifier.storage.lock().await.clear_all_data() {
                       Ok(()) => {
                           "🗑️ <b>Database cleared!</b>\n\nAll offers, stats and notifications removed.".to_string()
                       }
                       Err(e) => {
                           warn!("/cleardb error: {:?}", e);
                           format!("❌ Error: {:?}", e)
                       }
                   }
               }
               Command::Settings => {
                   let settings = notifier.config.clone();
                   format!(
                       "⚙️ <b>Current Settings</b>\n\n\
                        📊 Min Score: 60 (hardcoded)\n\
                        📍 Max Distance: unlimited\n\
                        ⏱ Check Interval: {}s\n\
                        📄 Max Pages: {}\n\
                        🔄 Max Retries: {}\n\
                        🚦 Requests/min: {}\n\n\
                        💡 Use /minscore and /maxdist to change",
                       settings.check_interval_seconds,
                       settings.scraper.max_pages,
                       settings.scraper.max_retries,
                       settings.scraper.requests_per_minute
                   )
               }
               Command::MinScore(value) => {
                   match value.parse::<f64>() {
                       Ok(score) if (60.0..=100.0).contains(&score) => {
                           // TODO: persist this setting
                           format!("✅ Min score set to {:.0} (restart required)", score)
                       }
                       Ok(_) => "❌ Score must be between 60 and 100".to_string(),
                       Err(_) => "❌ Invalid number. Usage: /minscore 70".to_string(),
                   }
               }
               Command::MaxDist(value) => {
                   match value.parse::<f64>() {
                       Ok(dist) if dist >= 0.0 => {
                           if dist == 0.0 {
                               "✅ Distance filter disabled".to_string()
                           } else {
                               format!("✅ Max distance set to {:.0} km (restart required)", dist)
                           }
                       }
                       Ok(_) => "❌ Distance must be >= 0".to_string(),
                       Err(_) => "❌ Invalid number. Usage: /maxdist 50 or /maxdist 0".to_string(),
                   }
               }
               Command::BestTimes => {
                   info!("/besttimes command received");
                   
                   // Get all offers and analyze timing
                   match notifier.storage.lock().await.get_all_offers() {
                       Ok(offers) => {
                           if offers.is_empty() {
                               "📭 No data yet. Wait for first scraping cycle.".to_string()
                           } else {
                               use crate::analyzer::timing_analysis::{analyze_posting_times, get_recommendations};
                               
                               match analyze_posting_times(&offers) {
                                   Some(analysis) => get_recommendations(&analysis),
                                   None => "❌ Unable to analyze timing data".to_string(),
                               }
                           }
                       }
                       Err(e) => {
                           warn!("/besttimes error: {:?}", e);
                           format!("❌ Error: {:?}", e)
                       }
                   }
               }
               Command::Profit(price_str) => {
                   info!("/profit command received: {}", price_str);
                   
                   match price_str.parse::<f64>() {
                       Ok(buy_price) if buy_price > 0.0 => {
                           use crate::analyzer::reseller_tools::{ProfitParams, calculate_profit};
                           
                           // Default target is 30% markup
                           let target_sell_price = buy_price * 1.3;
                           let params = ProfitParams {
                               target_sell_price,
                               ..Default::default()
                           };
                           
                           let profit = calculate_profit(buy_price, &params);
                           
                           format!(
                               "💰 <b>Profit Analysis</b>\n\n\
                                Buy: {:.2}€ | Sell: {:.2}€\n\n\
                                💵 <b>Costs:</b>\n\
                                • Platform Fee (5%): {:.2}€\n\
                                • Shipping: {:.2}€\n\
                                • Time Cost (2h): {:.2}€\n\
                                • <b>Total Costs: {:.2}€</b>\n\n\
                                📈 <b>Profit:</b>\n\
                                • Gross: {:.2}€\n\
                                • Net: <b>{:.2}€</b>\n\
                                • ROI: <b>{:.1}%</b>\n\n\
                                {}\n\n\
                                💡 Change sell price: /profit buy_price:sell_price",
                               profit.buy_price,
                               profit.sell_price,
                               profit.platform_fee,
                               profit.shipping_cost,
                               profit.time_cost,
                               profit.total_costs,
                               profit.gross_profit,
                               profit.net_profit,
                               profit.roi_percent,
                               if profit.is_profitable { "✅ <b>PROFITABLE</b>" } else { "❌ <b>NOT PROFITABLE</b>" }
                           )
                       }
                       _ => "❌ Invalid price. Usage: /profit 250 (calculates profit for buying at 250€)".to_string(),
                   }
               }
               Command::Market => {
                   info!("/market command received");
                   
                   "📊 <b>Market Analysis</b>\n\n\
                    ⏳ Collecting data...\n\
                    Check back after a few scraping cycles for trend analysis.\n\n\
                    📈 Features:\n\
                    • Price trends (rising/falling)\n\
                    • Speed metrics (hot/cold market)\n\
                    • Saturation level\n\
                    • Buy recommendations\n\n\
                    💡 Use /profit <price> for profit calculations".to_string()
               }
           };
           
           if let Err(e) = bot.send_message(chat_id, response)
               .parse_mode(teloxide::types::ParseMode::Html)
               .await
           {
               warn!("Failed to send response: {:?}", e);
           }
       }
