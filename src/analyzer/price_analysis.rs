use crate::model::{Offer, ModelStats};
use crate::config::ModelConfig;
use chrono::Utc;
use crate::analyzer::market_indicators::{MarketAnalyzer, PriceRange};
use crate::analyzer::lifecycle::build_lifecycle_data;
use std::collections::HashMap;

/// Trait defining the interface for an offer analyzer.
pub trait Analyzer {
    fn calculate_stats(&self, offers: &[Offer]) -> ModelStats;
    fn find_deals(&self, offers: &[Offer], stats: &ModelStats, cfg: &ModelConfig) -> Vec<Offer>;
    /// Expanded deal finder using additional market indicators.
    fn find_deals_expanded(
        &self,
        offers: &[Offer],
        stats: &ModelStats,
        cfg: &ModelConfig,
        analysis: &AnalysisResult,
        volatility_threshold: f64,
    ) -> Vec<Offer>;
}

/// Implementation of the offer analyzer.
pub struct AnalyzerImpl;

impl AnalyzerImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AnalyzerImpl {
    /// Calculates basic statistical metrics from offers: average price and standard deviation.
    fn calculate_stats(&self, offers: &[Offer]) -> ModelStats {
        let prices: Vec<f64> = offers
            .iter()
            .map(|o| o.price)
            .filter(|&p| p > 0.0)
            .collect();
        let count = prices.len() as f64;
        let avg = prices.iter().sum::<f64>() / count;
        let stddev = (prices
            .iter()
            .map(|p| (p - avg).powi(2))
            .sum::<f64>() / count)
            .sqrt();

        ModelStats {
            model: offers
                .first()
                .map(|o| o.model.clone())
                .unwrap_or_else(|| "unknown".into()),
            avg_price: avg,
            std_dev: stddev,
            last_updated: Utc::now(),
        }
    }

    /// Filters offers based on basic configuration thresholds and statistical metrics.
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

    /// Filters offers using both the basic criteria and additional market indicators.
    /// In addition to the basic filters, it checks the price volatility in the offer's price range.
    /// If volatility is too high (i.e., market is unstable), the offer is skipped.
    fn find_deals_expanded(
        &self,
        offers: &[Offer],
        stats: &ModelStats,
        cfg: &ModelConfig,
        analysis: &AnalysisResult,
        volatility_threshold: f64,
    ) -> Vec<Offer> {
        let mut result = Vec::new();
        
        for offer in offers {
            // Basic price range filtering
            if offer.price < cfg.min_price || offer.price > cfg.max_price {
                continue;
            }
            let base_condition = {
                let under_percent = offer.price < stats.avg_price * (1.0 - cfg.deviation_threshold);
                let under_absolute = (stats.avg_price - offer.price) >= cfg.min_price_delta;
                under_percent || under_absolute
            };
            if !base_condition {
                continue;
            }
            // Determine the price range for the offer
            let range = MarketAnalyzer::get_price_range_with_step(offer.price, MarketAnalyzer::DEFAULT_STEP);
            // Check the volatility for this price range
            if let Some(&volatility) = analysis.volatility_map.get(&range) {
                // If volatility is high, skip this offer (market is too unstable)
                if volatility > volatility_threshold {
                    continue;
                }
            }
            // Additional filtering based on median lifespan could be added here.
            // For example, if the offer's lifespan (if available) significantly deviates from the median,
            // mark it as an outlier or adjust the scoring.
            result.push(offer.clone());
        }
        result
    }
}

/// Structure representing the overall analysis result.
pub struct AnalysisResult {
    /// Average lifespan (disappearance speed) for each price range.
    pub disappearance_map: HashMap<PriceRange, chrono::Duration>,
    /// Frequency of price changes across offers.
    pub price_change_frequency: f64,
    /// Relative Strength Index computed from offer prices.
    pub rsi: f64,
    /// Price volatility (standard deviation) for each price range.
    pub volatility_map: HashMap<PriceRange, f64>,
    /// Median lifespan of offers for each price range.
    pub lifespan_median: HashMap<PriceRange, chrono::Duration>,
}

impl AnalyzerImpl {
    /// Asynchronously analyzes offers by building lifecycle data and computing various market indicators.
    /// It calculates basic metrics (average price, stddev) and then computes extended indicators:
    /// disappearance map, price change frequency, RSI, price volatility and median lifespan.
    pub async fn analyze_offers(&self, offers: &[Offer]) -> AnalysisResult {
        // Build lifecycle data for offers.
        let lifecycles = build_lifecycle_data(offers).await;
        
        // Calculate disappearance map for each price range.
        let disappearance_map = MarketAnalyzer::disappearance_speed(&lifecycles);
        
        // Calculate the frequency of price changes.
        let freq = MarketAnalyzer::price_change_frequency(&lifecycles);
        
        // Compute RSI using the series of prices extracted from the lifecycles.
        let mut price_series: Vec<f64> = lifecycles.iter().map(|o| o.price).collect();
        price_series.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let rsi = MarketAnalyzer::compute_rsi(&price_series);
        
        // New extended calculations:
        let volatility_map = MarketAnalyzer::price_volatility(&lifecycles);
        let lifespan_median = MarketAnalyzer::lifespan_median(&lifecycles);
        
        AnalysisResult {
            disappearance_map,
            price_change_frequency: freq,
            rsi,
            volatility_map,
            lifespan_median,
        }
    }
}
