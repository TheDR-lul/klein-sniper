use crate::model::{ScrapeRequest, ScraperError};

#[async_trait::async_trait]
pub trait Scraper: Send + Sync {
    async fn fetch(&self, req: &ScrapeRequest) -> Result<String, ScraperError>;
}