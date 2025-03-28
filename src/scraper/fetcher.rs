use crate::model::{ScrapeRequest, ScraperError};
use crate::scraper::traits::Scraper;
use reqwest::{Client, header};
use rand::prelude::*; // Правильный импорт для использования choose
use tokio::time::{sleep, Duration};

const USER_AGENTS: [&str; 5] = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.159 Safari/537.36",
    "Mozilla/5.0 (Windows NT 6.1; WOW64; rv:78.0) Gecko/20100101 Firefox/78.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/91.0.864.64 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.212 Safari/537.36",
];

pub struct ScraperImpl {
    client: Client,
}

impl ScraperImpl {
    pub fn new() -> Self {
        // Случайно выбираем User-Agent из списка
        let random_user_agent = USER_AGENTS.choose(&mut rand::rng()).unwrap(); 

        let client = Client::builder()
            .user_agent(random_user_agent.to_string())  // Исправление: конвертируем строку в строку типа String
            .default_headers({
                let mut headers = header::HeaderMap::new();
                headers.insert(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9".parse().unwrap());
                headers.insert(header::ACCEPT_ENCODING, "gzip, deflate, br".parse().unwrap());
                headers
            })
            .build()
            .unwrap();

        Self { client }
    }

    fn build_url(&self, req: &ScrapeRequest) -> String {
        let kebab_query = req.query.to_lowercase().replace(" ", "-");
        format!("https://www.kleinanzeigen.de/s-{}/{}", kebab_query, req.category_id)
    }

    async fn apply_delay(&self) {
        sleep(Duration::from_secs(1)).await;
    }
}

#[async_trait::async_trait]
impl Scraper for ScraperImpl {
    async fn fetch(&self, req: &ScrapeRequest) -> Result<String, ScraperError> {
        self.apply_delay().await;

        let url = self.build_url(req);

        // Логируем URL, по которому будет осуществлен запрос
        tracing::info!("Fetching URL: {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| ScraperError::HttpError(e.to_string()))?;

        let status = response.status();
        let html = response
            .text()
            .await
            .map_err(|e| ScraperError::HttpError(e.to_string()))?;

        if !status.is_success() {
            return Err(ScraperError::InvalidResponse(html));
        }

        Ok(html)
    }
}