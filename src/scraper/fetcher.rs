use crate::model::{ScrapeRequest, ScraperError};

use reqwest::Client;

pub trait Scraper {
    fn fetch(&self, req: &ScrapeRequest) -> Result<String, ScraperError>;
}

pub struct ScraperImpl {
    client: Client,
}

impl ScraperImpl {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) KleinSniperBot/0.1")
            .build()
            .unwrap();

        Self { client }
    }

    fn build_url(&self, req: &ScrapeRequest) -> String {
        let kebab_query = req.query.to_lowercase().replace(" ", "-");
        format!("https://www.kleinanzeigen.de/s-{}/{}", kebab_query, req.category_id)
    }
}

impl Scraper for ScraperImpl {
    fn fetch(&self, req: &ScrapeRequest) -> Result<String, ScraperError> {
        let url = self.build_url(req);

        let response = self.client.get(&url)
            .send()
            .map_err(|e| ScraperError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ScraperError::InvalidResponse);
        }

        response.text().map_err(|e| ScraperError::HttpError(e.to_string()))
    }
}
