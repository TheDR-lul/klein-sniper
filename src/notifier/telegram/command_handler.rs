use crate::notifier::telegram::TelegramNotifier;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tracing::{info, warn};

/// Определение команд бота
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Доступные команды:")]
pub enum Command {
    #[command(description = "проверка соединения")]
    Ping,
    #[command(description = "статус анализатора")]
    Status,
    #[command(description = "список команд")]
    Help,
    #[command(description = "последняя выгодная сделка")]
    Last,
    #[command(description = "топ-5 офферов")]
    Top5,
    #[command(description = "средние цены")]
    Avg,
    #[command(description = "текущая конфигурация")]
    Config,
    #[command(description = "ручной перезапуск")]
    Refresh,
    #[command(description = "время работы сервиса")]
    Uptime,
    #[command(description = "принудительная отправка")]
    ForceNotify,
    #[command(description = "статистика базы данных")]
    DbStats,
}

/// Запускает бота и обрабатывает команды
pub async fn run_bot(notifier: Arc<TelegramNotifier>) {
    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
            let notifier = notifier.clone();
            async move {
                handle_command(bot, msg, cmd, notifier).await;
                Ok(())
            }
        });

    Dispatcher::builder(notifier.bot.clone(), handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

/// Обработчик команд
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
            "✅ Я онлайн!".to_string()
        }
        Command::Status => {
            "📊 Анализатор работает. Ожидание следующей проверки.".to_string()
        }
        Command::Help => {
            Command::descriptions().to_string()
        }
        Command::Refresh => {
            info!("/refresh command received, triggering refresh...");
            notifier.refresh_notify.notify_one();
            "🔄 Принудительный перезапуск инициирован.".to_string()
        }
        Command::Uptime => {
            let uptime = notifier.start_time.elapsed();
            format!(
                "⏱ Время работы: {:02}:{:02}:{:02}",
                uptime.as_secs() / 3600,
                (uptime.as_secs() % 3600) / 60,
                uptime.as_secs() % 60
            )
        }
        Command::Last => {
            match notifier.storage.lock().await.get_last_offer() {
                Ok(Some(offer)) => {
                    format!(
                        "🕵️ Последний оффер:\n📦 {}\n💰 {:.2} €\n📍 {}\n🔗 {}",
                        offer.title, offer.price, offer.location, offer.link
                    )
                }
                Ok(None) => "📭 Нет офферов в базе данных.".to_string(),
                Err(e) => {
                    warn!("/last error: {:?}", e);
                    format!("❌ Ошибка: {:?}", e)
                }
            }
        }
        Command::Top5 => {
            match notifier.storage.lock().await.get_top5_offers() {
                Ok(offers) if !offers.is_empty() => {
                    let mut msg = String::from("🏆 Топ-5 лучших офферов:\n");
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
                Ok(_) => "📭 Нет офферов в базе данных.".to_string(),
                Err(e) => {
                    warn!("/top5 error: {:?}", e);
                    format!("❌ Ошибка: {:?}", e)
                }
            }
        }
        Command::Avg => {
            match notifier.storage.lock().await.get_average_prices() {
                Ok(prices) if !prices.is_empty() => {
                    let mut msg = String::from("📊 Средние цены по моделям:\n");
                    for (model, price) in prices {
                        msg.push_str(&format!("🔹 {} — {:.2} €\n", model, price));
                    }
                    msg
                }
                Ok(_) => "📭 Нет статистики по моделям.".to_string(),
                Err(e) => {
                    warn!("/avg error: {:?}", e);
                    format!("❌ Ошибка: {:?}", e)
                }
            }
        }
        Command::Config => {
            if notifier.config.models.is_empty() {
                "⚠️ Не загружено ни одной модели в конфигурации.".to_string()
            } else {
                let mut msg = String::from("⚙️ Загруженные модели:\n");
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
                            "✅ Уведомление отправлено!".to_string()
                        }
                        Err(e) => {
                            warn!("/force_notify send error: {:?}", e);
                            format!("❌ Ошибка отправки: {:?}", e)
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
