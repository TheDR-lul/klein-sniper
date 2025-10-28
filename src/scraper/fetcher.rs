use crate::model::{ScrapeRequest, ScraperError};
use crate::scraper::traits::Scraper;
use reqwest::{Client, header};
use rand::prelude::*;
use scraper::{Html, Selector};
use tokio::time::{sleep, Duration};
use governor::{Quota, RateLimiter, clock::DefaultClock, state::{InMemoryState, NotKeyed}};
use std::num::NonZeroU32;
use std::sync::Arc;
use backoff::{ExponentialBackoff, future::retry};
use tracing::{info, warn};

pub struct ScraperImpl {
    pub client: Client,          
    pub category_id: String, 
    pub min_price: f64,          
    pub max_price: f64,
    pub max_pages: usize,
    pub delay_seconds: u64,
    pub max_retries: u32,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl ScraperImpl {
    pub fn new() -> Self {
        Self::with_config(&[], 20, 1, 3, 30)
    }
    
    pub fn with_config(
        user_agents: &[String],
        max_pages: usize,
        delay_seconds: u64,
        max_retries: u32,
        requests_per_minute: u32,
    ) -> Self {
        let default_agents = vec![
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string(),
        ];
        
        let agents = if user_agents.is_empty() { &default_agents } else { user_agents };
        let random_user_agent = agents.choose(&mut rand::rng()).unwrap();

        let client = Client::builder()
            .user_agent(random_user_agent.as_str())
            .timeout(Duration::from_secs(30))
            .default_headers({
                let mut headers = header::HeaderMap::new();
                headers.insert(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9".parse().unwrap());
                headers.insert(header::ACCEPT_ENCODING, "gzip, deflate, br".parse().unwrap());
                headers
            })
            .build()
            .unwrap();

        // Create rate limiter: requests_per_minute requests per 60 seconds
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            client,
            category_id: String::new(),
            min_price: 0.0,
            max_price: 0.0,
            max_pages,
            delay_seconds,
            max_retries,
            rate_limiter,
        }
    }

    /// Build the URL for the request
    fn build_url(&self, req: &ScrapeRequest, page: usize) -> String {
        let kebab_query = req.query.to_lowercase().replace(" ", "-");
        if self.min_price > 0.0 || self.max_price > 0.0 {
            if page == 1 {
                format!(
                    "https://www.kleinanzeigen.de/s-preis:{0}:{1}/{2}/{3}",
                    self.min_price, self.max_price, kebab_query, self.category_id
                )
            } else {
                format!(
                    "https://www.kleinanzeigen.de/s-preis:{0}:{1}/seite:{2}/{3}/{4}",
                    self.min_price, self.max_price, page, kebab_query, self.category_id
                )
            }
        } else {
            if page == 1 {
                format!("https://www.kleinanzeigen.de/s-{0}/{1}", kebab_query, self.category_id)
            } else {
                format!("https://www.kleinanzeigen.de/s-seite:{0}/{1}/{2}", page, kebab_query, self.category_id)
            }
        }
    }

    async fn apply_delay(&self) {
        sleep(Duration::from_secs(self.delay_seconds)).await;
    }
    
    /// Fetch URL with retry logic and rate limiting
    async fn fetch_with_retry(&self, url: &str) -> Result<String, ScraperError> {
        // Wait for rate limiter
        self.rate_limiter.until_ready().await;
        
        if self.max_retries == 0 {
            // No retry logic, fetch once
            return self.fetch_once(url).await;
        }
        
        // Use exponential backoff for retries
        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(300)), // 5 minutes max
            ..Default::default()
        };
        
        let operation = || async {
            match self.fetch_once(url).await {
                Ok(html) => Ok(html),
                Err(e) => {
                    warn!("Fetch failed for {}: {:?}, retrying...", url, e);
                    Err(backoff::Error::transient(e))
                }
            }
        };
        
        retry(backoff, operation)
            .await
            .map_err(|e| match e {
                backoff::Error::Permanent(err) => err,
                backoff::Error::Transient { err, .. } => err,
            })
    }
    
    /// Single fetch attempt without retry
    async fn fetch_once(&self, url: &str) -> Result<String, ScraperError> {
        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| ScraperError::HttpError(e.to_string()))?;
        
        let status = response.status();
        
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| "unknown".into());
            return Err(ScraperError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, body
            )));
        }
        
        response
            .text()
            .await
            .map_err(|e| ScraperError::HttpError(e.to_string()))
    }
}

#[async_trait::async_trait]
impl Scraper for ScraperImpl {
    async fn fetch(&self, req: &ScrapeRequest) -> Result<String, ScraperError> {
        let mut full_html = String::new();
        let item_selector = Selector::parse("li.ad-listitem")
            .map_err(|e| ScraperError::HtmlParseError(e.to_string()))?;
        let ad_id_selector = Selector::parse("article.aditem").unwrap();

        let mut last_first_ad_id: Option<String> = None;

        for page in 1..=self.max_pages {
            self.apply_delay().await;
            let url = self.build_url(req, page);
            info!("Fetching page {} with retry logic: {}", page, url);

            // Fetch with retry and rate limiting
            let html = match self.fetch_with_retry(&url).await {
                Ok(html) => html,
                Err(e) => {
                    warn!("Failed to fetch page {}: {:?}", page, e);
                    return Err(e);
                }
            };

            let doc = Html::parse_document(&html);
            let items: Vec<_> = doc.select(&item_selector).collect();
            info!("Parsed {} items from page {}", items.len(), page);

            if items.is_empty() {
                info!("No items found on page {}, stopping.", page);
                break;
            }

            let first_ad_id = doc
                .select(&ad_id_selector)
                .next()
                .and_then(|n| n.value().attr("data-adid"))
                .map(|s| s.to_string());

            if let (Some(current), Some(last)) = (&first_ad_id, &last_first_ad_id) {
                if current == last {
                    info!("Duplicate first item detected on page {}, stopping.", page);
                    break;
                }
            }
            last_first_ad_id = first_ad_id;

            full_html.push_str(&html);
        }

        if full_html.is_empty() {
            Err(ScraperError::HtmlParseError("Empty HTML collected".into()))
        } else {
            Ok(full_html)
        }
    }
}
