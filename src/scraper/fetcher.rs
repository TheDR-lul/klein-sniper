use crate::model::{ScrapeRequest, ScraperError};
use crate::scraper::traits::Scraper;
use reqwest::{Client, header};
use rand::prelude::*;
use scraper::{Html, Selector};
use tokio::time::{sleep, Duration};

const USER_AGENTS: [&str; 5] = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.159 Safari/537.36",
    "Mozilla/5.0 (Windows NT 6.1; WOW64; rv:78.0) Gecko/20100101 Firefox/78.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/91.0.864.64 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.212 Safari/537.36",
];

pub struct ScraperImpl {
    pub client: Client,          
    pub category_id: String, 
    pub min_price: f64,          
    pub max_price: f64,          
}

impl ScraperImpl {
    pub fn new() -> Self {
        let random_user_agent = USER_AGENTS.choose(&mut rand::rng()).unwrap();

        let client = Client::builder()
            .user_agent(random_user_agent.to_string())
            .default_headers({
                let mut headers = header::HeaderMap::new();
                headers.insert(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9".parse().unwrap());
                headers.insert(header::ACCEPT_ENCODING, "gzip, deflate, br".parse().unwrap());
                headers
            })
            .build()
            .unwrap();

        Self {
            client,
            category_id: String::new(),
            min_price: 0.0,
            max_price: 0.0,
        }
    }

    /// Builds the URL for the request.
    /// If price filters are set (min_price > 0.0 or max_price > 0.0),
    /// then for the first page the URL is in the form:
    ///   https://www.kleinanzeigen.de/s-preis:{min_price}:{max_price}/{query}/{category_id}
    /// and for subsequent pages:
    ///   https://www.kleinanzeigen.de/s-preis:{min_price}:{max_price}/seite:{page}/{query}/{category_id}
    /// Otherwise, the basic URL format is used.
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
        sleep(Duration::from_secs(1)).await;
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
        let max_pages = 20;

        for page in 1..=max_pages {
            self.apply_delay().await;
            let url = self.build_url(req, page);
            tracing::info!("Fetching page {}: {}", page, url);

            let response = match self.client.get(&url).send().await {
                Ok(resp) => resp,
                Err(e) => return Err(ScraperError::HttpError(e.to_string())),
            };

            let status = response.status();
            let html = match response.text().await {
                Ok(t) => t,
                Err(e) => return Err(ScraperError::HttpError(e.to_string())),
            };

            if !status.is_success() {
                return Err(ScraperError::InvalidResponse(html));
            }

            let doc = Html::parse_document(&html);
            let items: Vec<_> = doc.select(&item_selector).collect();
            tracing::info!("Parsed {} items from page {}", items.len(), page);

            if items.is_empty() {
                tracing::info!("No items found on page {}, stopping.", page);
                break;
            }

            let first_ad_id = doc
                .select(&ad_id_selector)
                .next()
                .and_then(|n| n.value().attr("data-adid"))
                .map(|s| s.to_string());

            if let (Some(current), Some(last)) = (&first_ad_id, &last_first_ad_id) {
                if current == last {
                    tracing::info!("Duplicate first item detected on page {}, stopping.", page);
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