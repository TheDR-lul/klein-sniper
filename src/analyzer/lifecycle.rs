use crate::model::Offer;
use crate::model::OfferLifecycle;
use std::collections::HashMap;

/// Builds lifecycle data from a list of offers.
/// Groups offers by their id and tracks price changes along with the earliest and latest timestamps.
pub async fn build_lifecycle_data(offers: &[Offer]) -> Vec<OfferLifecycle> {
    let mut grouped: HashMap<String, OfferLifecycle> = HashMap::new();

    for offer in offers {
        // If an offer with the same id hasn't been seen yet, create a new OfferLifecycle.
        let entry = grouped.entry(offer.id.clone()).or_insert_with(|| OfferLifecycle {
            price: offer.price,
            first_seen: offer.fetched_at,
            last_seen: offer.fetched_at,
            price_changes: 0,
        });

        // If the price has changed (accounting for floating point precision), record the change.
        if (offer.price - entry.price).abs() > f64::EPSILON {
            entry.price_changes += 1; 
            entry.price = offer.price;
        }

        // Update the first seen and last seen timestamps.
        if offer.fetched_at < entry.first_seen {
            entry.first_seen = offer.fetched_at;
        }
        if offer.fetched_at > entry.last_seen {
            entry.last_seen = offer.fetched_at;
        }
    }

    grouped.into_values().collect()
}