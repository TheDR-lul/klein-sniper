// Utility functions
use chrono::{DateTime, Utc};
use std::time::Duration;
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use crate::model::Offer;
use std::collections::HashMap;
use tracing::warn;

/// Parse datetime from RFC3339 string
pub fn parse_datetime(date_str: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(date_str)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Convert string to kebab-case
pub fn to_kebab_case(text: &str) -> String {
    text.to_lowercase().replace(' ', "-")
}

/// Create exponential backoff configuration for retries
pub fn create_backoff() -> ExponentialBackoff {
    ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_secs(1))
        .with_max_interval(Duration::from_secs(30))
        .with_multiplier(2.0)
        .with_max_elapsed_time(Some(Duration::from_secs(300))) // 5 minutes max
        .build()
}

/// Format duration in human-readable format
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Filter out suspicious sellers who post many similar offers (potential spam/scam)
/// 
/// # Parameters
/// - `offers`: List of offers to filter
/// - `max_offers_per_seller`: Maximum number of offers from one seller (default: 5)
/// - `min_price_variance`: Minimum price variance to not be considered spam (default: 10.0)
/// 
/// # Returns
/// Filtered list of offers with suspicious sellers removed
pub fn filter_suspicious_sellers(offers: Vec<Offer>, max_offers_per_seller: usize, min_price_variance: f64) -> Vec<Offer> {
    // Group offers by seller (user_id or user_name)
    let mut seller_offers: HashMap<String, Vec<Offer>> = HashMap::new();
    
    for offer in offers {
        let seller_key = offer.user_id.clone()
            .or_else(|| offer.user_name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        
        seller_offers.entry(seller_key).or_insert_with(Vec::new).push(offer);
    }
    
    let mut filtered = Vec::new();
    
    for (seller, seller_offer_list) in seller_offers {
        if seller == "unknown" {
            // Keep offers without seller info
            filtered.extend(seller_offer_list);
            continue;
        }
        
        // Check if seller has too many offers
        if seller_offer_list.len() > max_offers_per_seller {
            // Calculate price variance
            let prices: Vec<f64> = seller_offer_list.iter().map(|o| o.price).collect();
            let avg_price = prices.iter().sum::<f64>() / prices.len() as f64;
            let variance = prices.iter()
                .map(|p| (p - avg_price).abs())
                .sum::<f64>() / prices.len() as f64;
            
            if variance < min_price_variance {
                warn!(
                    "🚫 Filtering suspicious seller '{}': {} offers with low price variance ({:.2}€)",
                    seller, seller_offer_list.len(), variance
                );
                // Skip this seller's offers (suspected spam)
                continue;
            }
        }
        
        // Keep legitimate offers
        filtered.extend(seller_offer_list);
    }
    
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_offer(id: &str, user_id: Option<String>, price: f64) -> Offer {
        Offer {
            id: id.to_string(),
            title: "Test Offer".to_string(),
            description: "Test".to_string(),
            price,
            location: "Berlin".to_string(),
            model: "TestModel".to_string(),
            link: "https://test.com".to_string(),
            posted_at: Utc::now(),
            fetched_at: Utc::now(),
            user_id,
            user_name: None,
            user_url: None,
        }
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("Hello World"), "hello-world");
        assert_eq!(to_kebab_case("ROG Ally"), "rog-ally");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(45)), "45s");
        assert_eq!(format_duration(Duration::from_secs(125)), "2m 5s");
        assert_eq!(format_duration(Duration::from_secs(7325)), "2h 2m 5s");
    }
    
    #[test]
    fn test_filter_suspicious_sellers_normal() {
        let offers = vec![
            create_test_offer("1", Some("seller1".to_string()), 100.0),
            create_test_offer("2", Some("seller2".to_string()), 200.0),
            create_test_offer("3", Some("seller3".to_string()), 150.0),
        ];
        
        let filtered = filter_suspicious_sellers(offers, 5, 10.0);
        assert_eq!(filtered.len(), 3); // All kept
    }
    
    #[test]
    fn test_filter_suspicious_sellers_spam() {
        let offers = vec![
            create_test_offer("1", Some("spammer".to_string()), 100.0),
            create_test_offer("2", Some("spammer".to_string()), 101.0),
            create_test_offer("3", Some("spammer".to_string()), 102.0),
            create_test_offer("4", Some("spammer".to_string()), 100.5),
            create_test_offer("5", Some("spammer".to_string()), 101.5),
            create_test_offer("6", Some("spammer".to_string()), 100.2),
            create_test_offer("7", Some("legitimate".to_string()), 200.0),
        ];
        
        let filtered = filter_suspicious_sellers(offers, 5, 10.0);
        // Spammer has 6 offers with low variance (< 10.0), should be filtered
        assert_eq!(filtered.len(), 1); // Only legitimate seller kept
    }
    
    #[test]
    fn test_filter_suspicious_sellers_high_variance() {
        let offers = vec![
            create_test_offer("1", Some("seller".to_string()), 100.0),
            create_test_offer("2", Some("seller".to_string()), 200.0),
            create_test_offer("3", Some("seller".to_string()), 150.0),
            create_test_offer("4", Some("seller".to_string()), 250.0),
            create_test_offer("5", Some("seller".to_string()), 120.0),
            create_test_offer("6", Some("seller".to_string()), 300.0),
        ];
        
        let filtered = filter_suspicious_sellers(offers, 5, 10.0);
        // High price variance, legitimate seller
        assert_eq!(filtered.len(), 6); // All kept
    }
}