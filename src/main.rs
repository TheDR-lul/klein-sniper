mod config;
mod model;
mod scraper;
mod parser;
mod analyzer;
mod normalizer;
mod notifier;
mod storage;

use analyzer::AnalyzerImpl;
use crate::analyzer::price_analysis::Analyzer;
use config::load_config;
use model::ScrapeRequest;
use scraper::ScraperImpl;
use parser::KleinanzeigenParser;
use normalizer::normalize_all;
use notifier::TelegramNotifier;
use storage::SqliteStorage;

#[tokio::main]
async fn main() {
    // 1. Загрузка конфигурации
    let config = match load_config("config.json") {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("❌ Ошибка загрузки конфигурации: {e}");
            return;
        }
    };

    println!("🔄 Старт: анализ {} модели(ей)...", config.models.len());

    // 2. Инициализация модулей
    let scraper = ScraperImpl::new();
    let parser = KleinanzeigenParser::new();
    let analyzer = AnalyzerImpl::new();
    let storage = SqliteStorage::new("data.db").unwrap();
    let notifier = TelegramNotifier::new(config.telegram_bot_token.clone(), config.telegram_chat_id);

    // 3. Основной цикл по моделям
    for model_cfg in &config.models {
        println!("📦 Обработка модели: {}", model_cfg.query);

        let request = ScrapeRequest {
            query: model_cfg.query.clone(),
            category_id: model_cfg.category_id.clone(),
        };

        // Предыдущая статистика из БД
        if let Ok(Some(prev_stats)) = storage.get_stats(&model_cfg.query) {
            println!(
                "📂 Старая средняя цена: {:.2} € (обновлено: {})",
                prev_stats.avg_price,
                prev_stats.last_updated
            );
        }        

        let html = match scraper.fetch(&request).await {
            Ok(html) => html,
            Err(e) => {
                match e {
                    model::ScraperError::Timeout => eprintln!("⏱ Таймаут при загрузке страницы"),
                    model::ScraperError::HttpError(msg) => eprintln!("🌐 HTTP ошибка: {}", msg),
                    model::ScraperError::InvalidResponse => eprintln!("📄 Некорректный ответ от сервера"),
                }
                continue;
            }
        };

        let mut offers = match parser.parse(&html) {
            Ok(o) => o,
            Err(e) => {
                match e {
                    model::ParserError::HtmlParseError(msg) => eprintln!("❌ Парсинг HTML: {}", msg),
                    model::ParserError::MissingField(field) => eprintln!("⚠️ Отсутствует поле: {}", field),
                }
                continue;
            }
        };

        normalize_all(&mut offers, &config.models);

        for offer in &offers {
            if let Err(e) = storage.save_offer(offer) {
                eprintln!("⚠ Ошибка сохранения в БД: {:?}", e);
            }
        }

        let stats = analyzer.calculate_stats(&offers);

        if let Err(e) = storage.update_stats(&stats) {
            eprintln!("⚠ Ошибка обновления статистики: {:?}", e);
        }

        let good_offers = analyzer.find_deals(&offers, &stats, model_cfg);

        println!(
            "📊 {} | Средняя цена: {:.2} € | Стандартное отклонение: {:.2} | Найдено: {}",
            stats.model,
            stats.avg_price,
            stats.std_dev,
            good_offers.len()
        );

        for offer in good_offers {
            println!("💸 {} — {:.2} €", offer.title, offer.price);
            println!("📍 {} | 📝 {}", offer.location, offer.description);
            println!("🔗 {}", offer.link);

            match storage.is_notified(&offer.id) {
                Ok(true) => {
                    println!("⏩ Уже отправляли ранее, пропускаем.");
                    continue;
                }
                Ok(false) => {} // идём дальше
                Err(e) => {
                    eprintln!("⚠ Ошибка проверки уведомления: {:?}", e);
                    continue;
                }
            }

            if let Err(e) = notifier.notify(&offer).await {
                eprintln!("⚠ Ошибка при отправке в Telegram: {:?}", e);
            } else if let Err(e) = storage.mark_notified(&offer.id) {
                eprintln!("⚠ Ошибка при отметке уведомления: {:?}", e);
            }
        }

        println!("-----------------------------\n");
    }

    println!("🏁 Готово. Повтор через {} секунд.", config.check_interval_seconds);
}
