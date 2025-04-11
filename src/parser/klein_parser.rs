use crate::model::{Offer, ParserError};
use crate::config::ModelConfig;
use scraper::{Html, Selector};
use chrono::Utc;
use tracing::info;

pub struct KleinanzeigenParser;

impl KleinanzeigenParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_filtered(&self, html: &str, cfg: &ModelConfig) -> Result<Vec<Offer>, ParserError> {
        let document = Html::parse_document(html);
        let item_selector = Selector::parse("li.ad-listitem")
            .map_err(|e| ParserError::HtmlParseError(e.to_string()))?;
        let title_selector = Selector::parse("h2.text-module-begin a.ellipsis")
            .map_err(|e| ParserError::HtmlParseError(e.to_string()))?;
        let price_selector = Selector::parse("p.aditem-main--middle--price-shipping--price")
            .map_err(|e| ParserError::HtmlParseError(e.to_string()))?;
        let location_selector = Selector::parse("div.aditem-main--top--left")
            .map_err(|e| ParserError::HtmlParseError(e.to_string()))?;
        let description_selector = Selector::parse("p.aditem-main--middle--description")
            .map_err(|e| ParserError::HtmlParseError(e.to_string()))?;
        let user_name_selector = Selector::parse("div.aditem-main--bottom span.ellipsis")
            .map_err(|e| ParserError::HtmlParseError(e.to_string()))?;

        let mut offers = Vec::new();

        for element in document.select(&item_selector) {
            let title_elem = element.select(&title_selector).next();
            if title_elem.is_none() {
                continue;
            }
            let title_node = title_elem.unwrap();

            let price_elem = element.select(&price_selector).next();
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
                .select(&location_selector)
                .next()
                .map(|n| n.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .unwrap_or_default();

            let description = element
                .select(&description_selector)
                .next()
                .map(|n| n.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .unwrap_or_default();

            let user_name = element
                .select(&user_name_selector)
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