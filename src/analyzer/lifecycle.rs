
use crate::model::Offer;
use crate::analyzer::market_indicators::OfferLifecycle;
use chrono::Utc;
use std::collections::HashMap;

pub async fn build_lifecycle_data(offers: &[Offer]) -> Vec<OfferLifecycle> {
    let mut grouped: HashMap<String, OfferLifecycle> = HashMap::new();

    for offer in offers {
        let entry = grouped.entry(offer.id.clone()).or_insert_with(|| OfferLifecycle {
            price: offer.price,
            first_seen: offer.timestamp,
            last_seen: offer.timestamp,
            price_changes: 0,
        });

        if offer.price != entry.price {
            entry.price_changes += 1;
        }

        if offer.timestamp < entry.first_seen {
            entry.first_seen = offer.timestamp;
        }

        if offer.timestamp > entry.last_seen {
            entry.last_seen = offer.timestamp;
        }

        entry.price = offer.price;
    }

    grouped.into_values().collect()
}
