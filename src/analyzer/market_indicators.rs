use chrono::Duration;
use std::collections::HashMap;
use crate::model::OfferLifecycle;

/// Represents a price range with a lower and upper bound.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PriceRange(pub u32, pub u32);

/// Provides various market indicators for offers, such as disappearance speed,
/// price change frequency, Relative Strength Index (RSI), price volatility,
/// and median lifespan.
pub struct MarketAnalyzer;

impl MarketAnalyzer {
    /// Default step size for the price range (50 units).
    const DEFAULT_STEP: u32 = 50;

    /// Calculates the average lifespan (disappearance speed) of offers for each price range.
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

    /// Calculates the frequency of price changes for offers (grouped by id).
    pub fn price_change_frequency(offers: &[OfferLifecycle]) -> f64 {
        if offers.is_empty() {
            return 0.0;
        }
        let total_changes: f64 = offers.iter().map(|o| o.price_changes as f64).sum();
        let freq = total_changes / offers.len() as f64;
        (freq * 100.0).round() / 100.0
    }

    /// Calculates the Relative Strength Index (RSI) for a series of average prices.
    /// Returns 0.0 if less than two prices are provided.
    pub fn compute_rsi(avg_prices: &[f64]) -> f64 {
        if avg_prices.len() < 2 {
            return 0.0;
        }
        let mut gains = 0.0;
        let mut losses = 0.0;
        for window in avg_prices.windows(2) {
            let delta = window[1] - window[0];
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

    /// Returns the price range for a given price using the default step.
    fn get_price_range(price: f64) -> PriceRange {
        Self::get_price_range_with_step(price, Self::DEFAULT_STEP)
    }

    /// Returns the price range for a given price and step.
    pub fn get_price_range_with_step(price: f64, step: u32) -> PriceRange {
        let price_int = price.round() as u32;
        let lower = price_int / step * step;
        PriceRange(lower, lower + step)
    }

    /// Calculates price volatility (standard deviation) of offers for each price range.
    pub fn price_volatility(offers: &[OfferLifecycle]) -> HashMap<PriceRange, f64> {
        let mut map: HashMap<PriceRange, Vec<f64>> = HashMap::new();
        for offer in offers {
            let range = Self::get_price_range(offer.price);
            map.entry(range).or_default().push(offer.price);
        }
        map.into_iter()
            .map(|(range, prices)| {
                let count = prices.len() as f64;
                let mean = prices.iter().sum::<f64>() / count;
                let variance = prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / count;
                (range, variance.sqrt())
            })
            .collect()
    }

    /// Calculates the median lifespan for each price range.
    pub fn lifespan_median(offers: &[OfferLifecycle]) -> HashMap<PriceRange, Duration> {
        let mut map: HashMap<PriceRange, Vec<Duration>> = HashMap::new();
        for offer in offers {
            let range = Self::get_price_range(offer.price);
            let lifespan = offer.last_seen - offer.first_seen;
            map.entry(range).or_default().push(lifespan);
        }
        map.into_iter()
            .map(|(range, mut durations)| {
                // Sort durations by their value in seconds.
                durations.sort_by_key(|d| d.num_seconds());
                let mid = durations.len() / 2;
                let median = if durations.len() % 2 == 0 {
                    let d1 = durations[mid - 1];
                    let d2 = durations[mid];
                    d1 + (d2 - d1) / 2
                } else {
                    durations[mid]
                };
                (range, median)
            })
            .collect()
    }

    /// Calculates the moving average of a slice of f64 data using the specified window size.
    pub fn moving_average(data: &[f64], window_size: usize) -> Vec<f64> {
        if window_size == 0 || data.len() < window_size {
            return Vec::new();
        }
        data.windows(window_size)
            .map(|window| window.iter().sum::<f64>() / window_size as f64)
            .collect()
    }

    /// Calculates the Pearson correlation coefficient between two slices.
    /// Returns None if slices have different lengths or are empty.
    pub fn compute_correlation(x: &[f64], y: &[f64]) -> Option<f64> {
        if x.len() != y.len() || x.is_empty() {
            return None;
        }
        let n = x.len() as f64;
        let mean_x = x.iter().sum::<f64>() / n;
        let mean_y = y.iter().sum::<f64>() / n;
        let numerator: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| (xi - mean_x) * (yi - mean_y)).sum();
        let denominator_x: f64 = x.iter().map(|xi| (xi - mean_x).powi(2)).sum();
        let denominator_y: f64 = y.iter().map(|yi| (yi - mean_y).powi(2)).sum();
        let denominator = (denominator_x * denominator_y).sqrt();
        if denominator == 0.0 {
            None
        } else {
            Some(numerator / denominator)
        }
    }
}
