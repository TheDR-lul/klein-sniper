use crate::model::{Offer, ParserError};
use crate::config::ModelConfig;
use scraper::{Html, Selector};
use chrono::Utc;
use tracing::info;
use std::sync::OnceLock;

/// Global cached selectors for parsing optimization
static ITEM_SELECTOR: OnceLock<Selector> = OnceLock::new();
static TITLE_SELECTOR: OnceLock<Selector> = OnceLock::new();
static PRICE_SELECTOR: OnceLock<Selector> = OnceLock::new();
static LOCATION_SELECTOR: OnceLock<Selector> = OnceLock::new();
static DESCRIPTION_SELECTOR: OnceLock<Selector> = OnceLock::new();
static USER_NAME_SELECTOR: OnceLock<Selector> = OnceLock::new();

pub struct KleinanzeigenParser;

impl KleinanzeigenParser {
    pub fn new() -> Self {
        // Initialize selectors when the parser is created
        Self::init_selectors();
        Self
    }
    
    /// Initializes all selectors once
    fn init_selectors() {
        ITEM_SELECTOR.get_or_init(|| {
            Selector::parse("li.ad-listitem").expect("Invalid item selector")
        });
        TITLE_SELECTOR.get_or_init(|| {
            Selector::parse("h2.text-module-begin a.ellipsis").expect("Invalid title selector")
        });
        PRICE_SELECTOR.get_or_init(|| {
            Selector::parse("p.aditem-main--middle--price-shipping--price").expect("Invalid price selector")
        });
        LOCATION_SELECTOR.get_or_init(|| {
            Selector::parse("div.aditem-main--top--left").expect("Invalid location selector")
        });
        DESCRIPTION_SELECTOR.get_or_init(|| {
            Selector::parse("p.aditem-main--middle--description").expect("Invalid description selector")
        });
        USER_NAME_SELECTOR.get_or_init(|| {
            Selector::parse("div.aditem-main--bottom span.ellipsis").expect("Invalid user name selector")
        });
    }

    pub fn parse_filtered(&self, html: &str, cfg: &ModelConfig) -> Result<Vec<Offer>, ParserError> {
        let document = Html::parse_document(html);
        
        // Используем кэшированные селекторы
        let item_selector = ITEM_SELECTOR.get().unwrap();
        let title_selector = TITLE_SELECTOR.get().unwrap();
        let price_selector = PRICE_SELECTOR.get().unwrap();
        let location_selector = LOCATION_SELECTOR.get().unwrap();
        let description_selector = DESCRIPTION_SELECTOR.get().unwrap();
        let user_name_selector = USER_NAME_SELECTOR.get().unwrap();

        let mut offers = Vec::new();

        for element in document.select(item_selector) {
            let title_elem = element.select(title_selector).next();
            if title_elem.is_none() {
                continue;
            }
            let title_node = title_elem.unwrap();

            let price_elem = element.select(price_selector).next();
            if price_elem.is_none() {
                continue;
            }
            let price_node = price_elem.unwrap();

            let title = title_node.inner_html().trim().to_string();
            let link_raw = title_node.value().attr("href").unwrap_or("");
            let link = format!("https://www.kleinanzeigen.de{}", link_raw);

            let path_segments: Vec<&str> = link_raw.split('/').collect();
            let last_segment = path_segments.last().unwrap_or(&"");
            let numeric_id = last_segment.split('-').next().unwrap_or("");
            let id = numeric_id.to_string();

            let price_text = price_node
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .replace("€", "")
                .replace(".", "")
                .replace(",", ".")
                .trim()
                .to_string();
            let price = price_text.parse::<f64>().unwrap_or(0.0);

            if price < cfg.min_price || price > cfg.max_price {
                continue;
            }

            let title_lower = title.to_lowercase();
            if !cfg.match_keywords.iter().any(|kw| title_lower.contains(&kw.to_lowercase())) {
                continue;
            }

            let location = element
                .select(location_selector)
                .next()
                .map(|n| n.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .unwrap_or_default();

            let description = element
                .select(description_selector)
                .next()
                .map(|n| n.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .unwrap_or_default();

            let user_name = element
                .select(user_name_selector)
                .last()
                .map(|n| n.text().collect::<String>().trim().to_string());

            let offer = Offer {
                id,
                title,
                description,
                price,
                location,
                model: cfg.query.clone(),
                link,
                posted_at: Utc::now(),
                fetched_at: Utc::now(),
                user_id: None,
                user_name,
                user_url: None,
            };

            offers.push(offer);
        }

        info!("Parsed {} offers from HTML", offers.len());
        Ok(offers)
    }
}