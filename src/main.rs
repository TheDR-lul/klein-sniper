mod config;
mod model;
mod scraper;
mod parser;
mod analyzer;
mod normalizer;
mod notifier;
mod storage;

use analyzer::AnalyzerImpl;
use config::load_config;
use model::ScrapeRequest;
use scraper::ScraperImpl;
use parser::KleinanzeigenParser;
use crate::analyzer::price_analysis::Analyzer;
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
    for model_cfg in config.models.iter() {
        println!("📦 Обработка модели: {}", model_cfg.query);

        let request = ScrapeRequest {
            query: model_cfg.query.clone(),
            category_id: model_cfg.category_id.clone(),
        };

        let html = match scraper.fetch(&request) {
            Ok(html) => html,
            Err(e) => {
                eprintln!("❌ Ошибка при получении HTML: {:?}", e);
                continue;
            }
        };

        let mut offers = match parser.parse(&html) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("❌ Ошибка парсинга: {:?}", e);
                continue;
            }
        };

        normalize_all(&mut offers, &config.models);

        let stats = analyzer.calculate_stats(&offers);
        let good_offers = analyzer.find_deals(&offers, &stats, model_cfg);

        println!(
            "📊 Модель: {} | Средняя цена: {:.2} € | Найдено выгодных: {}",
            stats.model,
            stats.avg_price,
            good_offers.len()
        );

        for offer in good_offers {
            println!("💸 {} — {:.2} €", offer.title, offer.price);
            println!("🔗 {}", offer.link);

            // Отправляем уведомление
            if let Err(e) = notifier.notify(&offer).await {
                eprintln!("⚠ Ошибка уведомления: {:?}", e);
            }
        }

        println!("-----------------------------\n");
    }

    println!("🏁 Готово. Повтор через {} секунд.", config.check_interval_seconds);
}