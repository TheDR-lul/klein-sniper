use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PriceRange(pub u32, pub u32);

#[derive(Debug)]
pub struct OfferLifecycle {
    pub price: u32,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub price_changes: u32,
}

pub struct MarketAnalyzer;

impl MarketAnalyzer {
    pub fn disappearance_speed(offers: &[OfferLifecycle]) -> HashMap<PriceRange, Duration> {
        let mut map: HashMap<PriceRange, Vec<Duration>> = HashMap::new();

        for offer in offers {
            let range = Self::get_price_range(offer.price);
            let lifespan = offer.last_seen - offer.first_seen;
            map.entry(range).or_default().push(lifespan);
        }

        map.into_iter()
            .map(|(range, durations)| {
                let total: Duration = durations.iter().copied().sum();
                let avg = total / (durations.len() as i32);
                (range, avg)
            })
            .collect()
    }

    /// Частота изменения цены на один оффер (по ID)
    pub fn price_change_frequency(offers: &[OfferLifecycle]) -> f64 {
        if offers.is_empty() {
            return 0.0;
        }
        let total_changes: u32 = offers.iter().map(|o| o.price_changes).sum();
        total_changes as f64 / offers.len() as f64
    }

    /// RSI (Relative Strength Index) для средней цены
    pub fn compute_rsi(avg_prices: &[f64]) -> f64 {
        if avg_prices.len() < 2 {
            return 0.0;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for w in avg_prices.windows(2) {
            let delta = w[1] - w[0];
            if delta > 0.0 {
                gains += delta;
            } else {
                losses -= delta;
            }
        }

        if gains + losses == 0.0 {
            return 50.0;
        }

        let rs = gains / losses.max(1e-6);
        100.0 - (100.0 / (1.0 + rs))
    }

    fn get_price_range(price: u32) -> PriceRange {
        let step = 50;
        let lower = price / step * step;
        PriceRange(lower, lower + step)
    }
}