use crate::model::{ScrapeRequest, ScraperError};
use crate::scraper::traits::Scraper;
use reqwest::{Client, header};
use tokio::time::{sleep, Duration};

pub struct ScraperImpl {
    client: Client,
}

impl ScraperImpl {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) KleinSniperBot/0.1")
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
