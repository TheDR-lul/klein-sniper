// Price analysis, deviation detection
use crate::model::{Offer, ModelStats};
use crate::config::ModelConfig;
use chrono::Utc;
use crate::analyzer::market_indicators::MarketAnalyzer;
use crate::analyzer::lifecycle::build_lifecycle_data;

pub trait Analyzer {
    fn calculate_stats(&self, offers: &[Offer]) -> ModelStats;
    fn find_deals(&self, offers: &[Offer], stats: &ModelStats, cfg: &ModelConfig) -> Vec<Offer>;
}

pub struct AnalyzerImpl;

impl AnalyzerImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AnalyzerImpl {
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
pub struct AnalysisResult {
    pub disappearance_map: std::collections::HashMap<crate::analyzer::market_indicators::PriceRange, chrono::Duration>,
    pub price_change_frequency: f64,
    pub rsi: f64,
}

impl AnalyzerImpl {
    pub async fn analyze_offers(&self, offers: &[crate::model::Offer]) -> AnalysisResult {
        let lifecycles = build_lifecycle_data(offers).await;
        let disappearance_map = MarketAnalyzer::disappearance_speed(&lifecycles);
        let freq = MarketAnalyzer::price_change_frequency(&lifecycles);
        let rsi = MarketAnalyzer::compute_rsi(&[lifecycles.iter().map(|o| o.price as f64).sum::<f64>() / lifecycles.len() as f64]);

        AnalysisResult {
            disappearance_map,
            price_change_frequency: freq,
            rsi,
        }
    }
}
