// Kleinanzeigen-specific HTML parsing
use crate::model::{Offer, ParserError};
use chrono::Utc;
use scraper::{Html, Selector};

pub trait Parser {
    fn parse(&self, html: &str) -> Result<Vec<Offer>, ParserError>;
}

pub struct KleinanzeigenParser;

impl KleinanzeigenParser {
    pub fn new() -> Self {
        Self
    }
}

impl Parser for KleinanzeigenParser {
    fn parse(&self, html: &str) -> Result<Vec<Offer>, ParserError> {
        let document = Html::parse_document(html);

        let item_selector = Selector::parse("article.aditem").map_err(|e| ParserError::HtmlParseError(e.to_string()))?;
        let title_selector = Selector::parse("a.ellipsis").unwrap();
        let price_selector = Selector::parse("p.aditem-main--middle--price-shipping").unwrap();

        let mut offers = Vec::new();

        for element in document.select(&item_selector) {
            let title_elem = element.select(&title_selector).next();
            let price_elem = element.select(&price_selector).next();

            if let (Some(title_node), Some(price_node)) = (title_elem, price_elem) {
                let title = title_node.inner_html().trim().to_string();
                let link = title_node.value().attr("href").unwrap_or("").to_string();
                let price_text = price_node.text().collect::<String>();
                let price = price_text.replace("€", "").replace(".", "").trim().replace(",", ".").parse::<f64>().unwrap_or(0.0);

                let offer = Offer {
                    id: link.clone(),
                    title,
                    description: String::new(), // пока пусто
                    price,
                    location: String::new(),
                    model: String::new(), // нормализуем позже
                    link: format!("https://www.kleinanzeigen.de{}", link),
                    posted_at: Utc::now(),
                    fetched_at: Utc::now(),
                };

                offers.push(offer);
            }
        }

        Ok(offers)
    }
}
