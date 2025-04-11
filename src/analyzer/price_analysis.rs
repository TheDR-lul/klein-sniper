use crate::model::{Offer, ModelStats};
use crate::config::ModelConfig;
use chrono::Utc;
use crate::analyzer::market_indicators::{MarketAnalyzer, PriceRange};
use crate::analyzer::lifecycle::build_lifecycle_data;

/// Trait defining the interface for an offer analyzer.
pub trait Analyzer {
    fn calculate_stats(&self, offers: &[Offer]) -> ModelStats;
    fn find_deals(&self, offers: &[Offer], stats: &ModelStats, cfg: &ModelConfig) -> Vec<Offer>;
}

/// Implementation of the offer analyzer.
pub struct AnalyzerImpl;

impl AnalyzerImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AnalyzerImpl {
    /// Calculates statistical metrics for offers (average price and standard deviation).
    fn calculate_stats(&self, offers: &[Offer]) -> ModelStats {
        let prices: Vec<f64> = offers.iter().map(|o| o.price).filter(|&p| p > 0.0).collect();
        let count = prices.len() as f64;
        let avg = prices.iter().sum::<f64>() / count;
        let stddev = (prices.iter().map(|p| (p - avg).powi(2)).sum::<f64>() / count).sqrt();
    
        ModelStats {
            model: offers.first().map(|o| o.model.clone()).unwrap_or_else(|| "unknown".into()),
            avg_price: avg,
            std_dev: stddev,
            last_updated: Utc::now(),
        }
    }
    
    /// Filters offers based on configuration thresholds and statistical metrics.
    fn find_deals(&self, offers: &[Offer], stats: &ModelStats, cfg: &ModelConfig) -> Vec<Offer> {
        let mut result = Vec::new();
    
        for offer in offers {
            if offer.price < cfg.min_price || offer.price > cfg.max_price {
                continue;
            }
    
            let is_under_percent = offer.price < stats.avg_price * (1.0 - cfg.deviation_threshold);
            let is_under_absolute = (stats.avg_price - offer.price) >= cfg.min_price_delta;
    
            if is_under_percent || is_under_absolute {
                result.push(offer.clone());
            }
        }
    
        result
    }
}

/// Structure representing the overall analysis result.
pub struct AnalysisResult {
    pub disappearance_map: std::collections::HashMap<PriceRange, chrono::Duration>,
    pub price_change_frequency: f64,
    pub rsi: f64,
}

impl AnalyzerImpl {
    /// Asynchronously analyzes offers by building lifecycle data and computing various market indicators.
    /// The RSI is now computed based on the full series of prices extracted from the lifecycles.
    pub async fn analyze_offers(&self, offers: &[Offer]) -> AnalysisResult {
        // Build lifecycle data for offers.
        let lifecycles = build_lifecycle_data(offers).await;
        
        // Calculate the disappearance map per price range.
        let disappearance_map = MarketAnalyzer::disappearance_speed(&lifecycles);
        
        // Calculate the price change frequency.
        let freq = MarketAnalyzer::price_change_frequency(&lifecycles);
        
        // Extract a series of prices from lifecycles to compute RSI.
        let mut price_series: Vec<f64> = lifecycles.iter().map(|o| o.price).collect();
        // Optionally, sort the price series for a more robust RSI calculation.
        price_series.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let rsi = MarketAnalyzer::compute_rsi(&price_series);
    
        AnalysisResult {
            disappearance_map,
            price_change_frequency: freq,
            rsi,
        }
    }
}