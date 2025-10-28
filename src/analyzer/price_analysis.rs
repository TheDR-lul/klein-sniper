use crate::model::{Offer, ModelStats};
use crate::config::ModelConfig;
use chrono::Utc;
use crate::analyzer::market_indicators::{MarketAnalyzer, PriceRange};
use crate::analyzer::lifecycle::build_lifecycle_data;
use std::collections::HashMap;
use tracing::info;

/// Scored offer with quality rating (0-100)
#[derive(Debug, Clone)]
pub struct ScoredOffer {
    pub offer: Offer,
    pub score: f64,
    pub score_breakdown: ScoreBreakdown,
}

/// Breakdown of score components for transparency
#[derive(Debug, Clone)]
pub struct ScoreBreakdown {
    pub base_score: f64,
    pub percentile_bonus: f64,
    pub median_bonus: f64,
    pub volatility_penalty: f64,
    pub freshness_bonus: f64,
    pub total: f64,
}

/// Extended statistics with median and percentiles
#[derive(Debug, Clone)]
pub struct ExtendedStats {
    pub avg_price: f64,
    pub median_price: f64,
    pub std_dev: f64,
    pub percentile_10: f64,
    pub percentile_25: f64,
    pub percentile_75: f64,
    pub percentile_90: f64,
}

/// Trait defining the interface for an offer analyzer.
pub trait Analyzer {
    fn calculate_stats(&self, offers: &[Offer]) -> ModelStats;
    fn find_deals(&self, offers: &[Offer], stats: &ModelStats, cfg: &ModelConfig) -> Vec<ScoredOffer>;
    /// Expanded deal finder using additional market indicators.
    fn find_deals_expanded(
        &self,
        offers: &[Offer],
        stats: &ModelStats,
        cfg: &ModelConfig,
        analysis: &AnalysisResult,
        volatility_threshold: f64,
    ) -> Vec<ScoredOffer>;
}

/// Implementation of the offer analyzer.
pub struct AnalyzerImpl;

impl AnalyzerImpl {
    pub fn new() -> Self {
        Self
    }
    
    /// Calculate extended statistics including median and percentiles
    fn calculate_extended_stats(&self, offers: &[Offer]) -> ExtendedStats {
        let mut prices: Vec<f64> = offers
            .iter()
            .map(|o| o.price)
            .filter(|&p| p > 0.0)
            .collect();
        
        prices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let count = prices.len() as f64;
        let avg = prices.iter().sum::<f64>() / count;
        let stddev = (prices
            .iter()
            .map(|p| (p - avg).powi(2))
            .sum::<f64>() / count)
            .sqrt();
        
        let median = Self::calculate_percentile(&prices, 50.0);
        let p10 = Self::calculate_percentile(&prices, 10.0);
        let p25 = Self::calculate_percentile(&prices, 25.0);
        let p75 = Self::calculate_percentile(&prices, 75.0);
        let p90 = Self::calculate_percentile(&prices, 90.0);
        
        ExtendedStats {
            avg_price: avg,
            median_price: median,
            std_dev: stddev,
            percentile_10: p10,
            percentile_25: p25,
            percentile_75: p75,
            percentile_90: p90,
        }
    }
    
    /// Calculate percentile value from sorted price array
    fn calculate_percentile(sorted_prices: &[f64], percentile: f64) -> f64 {
        if sorted_prices.is_empty() {
            return 0.0;
        }
        
        let index = (percentile / 100.0) * (sorted_prices.len() - 1) as f64;
        let lower = index.floor() as usize;
        let upper = index.ceil() as usize;
        let fraction = index - lower as f64;
        
        if lower == upper {
            sorted_prices[lower]
        } else {
            sorted_prices[lower] * (1.0 - fraction) + sorted_prices[upper] * fraction
        }
    }
    
    /// Score an offer based on multiple factors (0-100 scale)
    /// Higher score = better deal
    fn score_offer(&self, offer: &Offer, extended_stats: &ExtendedStats, cfg: &ModelConfig) -> Option<ScoredOffer> {
        let price = offer.price;
        
        // Base score starts at 50
        let base_score = 50.0;
        let mut percentile_bonus = 0.0;
        let mut median_bonus = 0.0;
        let volatility_penalty = 0.0; // Will be used in extended version
        let mut freshness_bonus = 0.0;
        
        // 1. Percentile bonus: +30 for top 10%, +20 for top 25%
        if price <= extended_stats.percentile_10 {
            percentile_bonus = 30.0;
        } else if price <= extended_stats.percentile_25 {
            percentile_bonus = 20.0;
        }
        
        // 2. Median deviation bonus: +25 if significantly below median
        let median_deviation = (extended_stats.median_price - price) / extended_stats.median_price;
        if median_deviation > 0.25 {
            median_bonus = 25.0;
        } else if median_deviation > 0.15 {
            median_bonus = 15.0;
        } else if median_deviation > 0.05 {
            median_bonus = 5.0;
        }
        
        // 3. Freshness bonus: +15 for offers < 1 hour old
        let age = Utc::now().signed_duration_since(offer.fetched_at);
        if age.num_hours() < 1 {
            freshness_bonus = 15.0;
        } else if age.num_hours() < 6 {
            freshness_bonus = 5.0;
        }
        
        // Calculate total score
        let total_score = base_score 
            + percentile_bonus 
            + median_bonus 
            + freshness_bonus 
            - volatility_penalty;
        
        let breakdown = ScoreBreakdown {
            base_score,
            percentile_bonus,
            median_bonus,
            volatility_penalty,
            freshness_bonus,
            total: total_score,
        };
        
        // Only return offers that meet minimum thresholds
        if price < cfg.min_price || price > cfg.max_price {
            return None;
        }
        
        // Require at least 60 points to be considered a "deal"
        if total_score < 60.0 {
            return None;
        }
        
        Some(ScoredOffer {
            offer: offer.clone(),
            score: total_score,
            score_breakdown: breakdown,
        })
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

    /// Filters offers based on advanced scoring system
    fn find_deals(&self, offers: &[Offer], _stats: &ModelStats, cfg: &ModelConfig) -> Vec<ScoredOffer> {
        let extended_stats = self.calculate_extended_stats(offers);
        
        info!("📊 Extended Stats:");
        info!("   Median: {:.2} €", extended_stats.median_price);
        info!("   Avg: {:.2} €", extended_stats.avg_price);
        info!("   P10: {:.2} € | P25: {:.2} €", extended_stats.percentile_10, extended_stats.percentile_25);
        info!("   P75: {:.2} € | P90: {:.2} €", extended_stats.percentile_75, extended_stats.percentile_90);
        
        let mut scored_offers: Vec<ScoredOffer> = offers
            .iter()
            .filter_map(|offer| self.score_offer(offer, &extended_stats, cfg))
            .collect();
        
        // Sort by score descending (best deals first)
        scored_offers.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        // Log top scored offers
        for (i, scored) in scored_offers.iter().take(5).enumerate() {
            info!(
                "🏆 #{} Score: {:.1} | Price: {:.2} € | {} [P:{:.0} M:{:.0} F:{:.0}]",
                i + 1,
                scored.score,
                scored.offer.price,
                scored.offer.id,
                scored.score_breakdown.percentile_bonus,
                scored.score_breakdown.median_bonus,
                scored.score_breakdown.freshness_bonus
            );
        }
        
        scored_offers
    }

    /// Filters offers using both the basic criteria and additional market indicators.
    /// Uses scoring system with volatility penalty applied
    fn find_deals_expanded(
        &self,
        offers: &[Offer],
        _stats: &ModelStats,
        cfg: &ModelConfig,
        analysis: &AnalysisResult,
        volatility_threshold: f64,
    ) -> Vec<ScoredOffer> {
        let extended_stats = self.calculate_extended_stats(offers);
        
        let mut scored_offers: Vec<ScoredOffer> = offers
            .iter()
            .filter_map(|offer| {
                let mut scored = self.score_offer(offer, &extended_stats, cfg)?;
                
                // Apply volatility penalty
                let range = MarketAnalyzer::get_price_range_with_step(offer.price, MarketAnalyzer::DEFAULT_STEP);
                if let Some(&volatility) = analysis.volatility_map.get(&range) {
                    if volatility > volatility_threshold {
                        // High volatility: -20 points
                        scored.score_breakdown.volatility_penalty = 20.0;
                        scored.score_breakdown.total -= 20.0;
                        scored.score -= 20.0;
                        
                        // Skip if score drops below threshold
                        if scored.score < 60.0 {
                            return None;
                        }
                    }
                }
                
                Some(scored)
            })
            .collect();
        
        // Sort by score descending
        scored_offers.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        scored_offers
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
