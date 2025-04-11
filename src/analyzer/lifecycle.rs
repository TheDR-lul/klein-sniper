use crate::model::Offer;
use crate::model::OfferLifecycle;
use std::collections::HashMap;

pub async fn build_lifecycle_data(offers: &[Offer]) -> Vec<OfferLifecycle> {
    let mut grouped: HashMap<String, OfferLifecycle> = HashMap::new();

    for offer in offers {
        let entry = grouped.entry(offer.id.clone()).or_insert_with(|| OfferLifecycle {
            price: offer.price,
            first_seen: offer.fetched_at,
            last_seen: offer.fetched_at,
            price_changes: 0,
        });

        if (offer.price - entry.price).abs() > f64::EPSILON {
            entry.price_changes += 1;
            entry.price = offer.price;
        }

        if offer.fetched_at < entry.first_seen {
            entry.first_seen = offer.fetched_at;
        }

        if offer.fetched_at > entry.last_seen {
            entry.last_seen = offer.fetched_at;
        }
    }

    grouped.into_values().collect()
}