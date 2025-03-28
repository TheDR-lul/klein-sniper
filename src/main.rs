mod config;
mod model;

use config::load_config;
use model::ScrapeRequest;

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

    // 2. Инициализация модулей (заглушки)
    // let scraper = ScraperImpl::new();
    // let parser = ParserImpl::new();
    // let analyzer = AnalyzerImpl::new();
    // let storage = SqliteStorage::new("data.db").unwrap();
    // let notifier = TelegramNotifier::new(...);

    // 3. Основной цикл по моделям
    for model_cfg in config.models.iter() {
        println!("📦 Обработка модели: {}", model_cfg.query);

        let request = ScrapeRequest {
            query: model_cfg.query.clone(),
            category_id: model_cfg.category_id.clone(),
        };

        // 🔽 Ниже идут заглушки, замени на реальные вызовы
        println!("📡 Скрейпинг по запросу: '{}' в категории '{}'", request.query, request.category_id);

        println!("🔍 Парсинг HTML...");
        println!("📊 Анализ предложений...");
        println!("📈 Средняя цена: 635€ | Отправка уведомлений...");

        // Пример успешного уведомления
        println!("✅ Найдено выгодное предложение: ROG Ally за 499€");
        println!("🔗 https://www.kleinanzeigen.de/s-anzeige/...");
    }

    println!("🏁 Готово. Повтор через {} секунд.", config.check_interval_seconds);
}
