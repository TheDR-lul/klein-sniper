/// Professional reseller analysis tools
/// 
/// This module provides advanced analytics for professional resellers including:
/// - Price history tracking and trend detection
/// - Profit calculations with fees and costs
/// - Market saturation analysis
/// - Speed metrics for offer turnover

use crate::model::Offer;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use tracing::info;

/// Price trend direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PriceTrend {
    Rising,      // Market heating up (bad for buying)
    Falling,     // Market cooling down (good for buying)
    Stable,      // No significant change
    Volatile,    // Unpredictable fluctuations
}

/// Price history entry
#[derive(Debug, Clone)]
pub struct PriceHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub avg_price: f64,
    pub median_price: f64,
    pub offer_count: usize,
    pub std_dev: f64,
}

/// Price trend analysis result
#[derive(Debug, Clone)]
pub struct PriceTrendAnalysis {
    pub current_avg: f64,
    pub week_ago_avg: Option<f64>,
    pub trend: PriceTrend,
    pub change_percent: f64,
    pub prediction_next_week: f64,
    pub buy_recommendation: BuyRecommendation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuyRecommendation {
    StrongBuy,    // Prices falling, good time
    Buy,          // Stable prices, okay to buy
    Hold,         // Rising prices, wait
    Avoid,        // Volatile market, risky
}

/// Profit calculation parameters
#[derive(Debug, Clone)]
pub struct ProfitParams {
    pub platform_fee_percent: f64,  // e.g., 5% for Kleinanzeigen
    pub shipping_cost: f64,          // Fixed shipping cost
    pub time_cost_per_hour: f64,    // Your time value
    pub estimated_hours: f64,        // Time to resell
    pub target_sell_price: f64,     // Expected selling price
}

impl Default for ProfitParams {
    fn default() -> Self {
        Self {
            platform_fee_percent: 5.0,
            shipping_cost: 5.0,
            time_cost_per_hour: 15.0,  // 15€/hour opportunity cost
            estimated_hours: 2.0,       // 2 hours to list, sell, ship
            target_sell_price: 0.0,     // Must be set
        }
    }
}

/// Profit calculation result
#[derive(Debug, Clone)]
pub struct ProfitAnalysis {
    pub buy_price: f64,
    pub sell_price: f64,
    pub platform_fee: f64,
    pub shipping_cost: f64,
    pub time_cost: f64,
    pub total_costs: f64,
    pub gross_profit: f64,
    pub net_profit: f64,
    pub roi_percent: f64,
    pub is_profitable: bool,
}

/// Speed metrics for offer turnover
#[derive(Debug, Clone)]
pub struct SpeedMetrics {
    pub price_range: (f64, f64),
    pub avg_lifespan_hours: f64,
    pub disappearance_rate: f64,  // offers/hour
    pub is_hot_market: bool,       // High demand indicator
}

/// Market saturation analysis
#[derive(Debug, Clone)]
pub struct SaturationAnalysis {
    pub current_offers_count: usize,
    pub avg_offers_last_week: f64,
    pub saturation_level: SaturationLevel,
    pub competition_intensity: f64,  // 0-100
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SaturationLevel {
    Low,      // Few offers, high demand
    Medium,   // Balanced market
    High,     // Many offers, low demand
    Extreme,  // Oversaturated, hard to sell
}

/// Calculate profit for a potential purchase
pub fn calculate_profit(buy_price: f64, params: &ProfitParams) -> ProfitAnalysis {
    let platform_fee = params.target_sell_price * (params.platform_fee_percent / 100.0);
    let time_cost = params.time_cost_per_hour * params.estimated_hours;
    let total_costs = buy_price + platform_fee + params.shipping_cost + time_cost;
    let gross_profit = params.target_sell_price - buy_price;
    let net_profit = params.target_sell_price - total_costs;
    let roi_percent = if buy_price > 0.0 {
        (net_profit / buy_price) * 100.0
    } else {
        0.0
    };

    ProfitAnalysis {
        buy_price,
        sell_price: params.target_sell_price,
        platform_fee,
        shipping_cost: params.shipping_cost,
        time_cost,
        total_costs,
        gross_profit,
        net_profit,
        roi_percent,
        is_profitable: net_profit > 0.0,
    }
}

/// Analyze price trend from historical data
pub fn analyze_price_trend(history: &[PriceHistoryEntry]) -> Option<PriceTrendAnalysis> {
    if history.is_empty() {
        return None;
    }

    // Sort by timestamp
    let mut sorted = history.to_vec();
    sorted.sort_by_key(|e| e.timestamp);

    let current = sorted.last()?;
    let current_avg = current.avg_price;

    // Look for entry from ~7 days ago
    let week_ago = Utc::now() - Duration::days(7);
    let week_ago_entry = sorted
        .iter()
        .find(|e| (e.timestamp - week_ago).num_hours().abs() < 24);

    let (trend, change_percent, buy_rec) = if let Some(old) = week_ago_entry {
        let change = ((current_avg - old.avg_price) / old.avg_price) * 100.0;
        
        let trend = if change.abs() < 3.0 {
            PriceTrend::Stable
        } else if change < -5.0 {
            PriceTrend::Falling
        } else if change > 5.0 {
            PriceTrend::Rising
        } else if current.std_dev > current_avg * 0.2 {
            PriceTrend::Volatile
        } else {
            PriceTrend::Stable
        };

        let recommendation = match trend {
            PriceTrend::Falling => BuyRecommendation::StrongBuy,
            PriceTrend::Stable => BuyRecommendation::Buy,
            PriceTrend::Rising => BuyRecommendation::Hold,
            PriceTrend::Volatile => BuyRecommendation::Avoid,
        };

        (trend, change, recommendation)
    } else {
        (PriceTrend::Stable, 0.0, BuyRecommendation::Buy)
    };

    // Simple linear prediction
    let prediction = if sorted.len() >= 2 {
        let recent = &sorted[sorted.len().saturating_sub(3)..];
        let avg_change = recent.windows(2)
            .map(|w| w[1].avg_price - w[0].avg_price)
            .sum::<f64>() / (recent.len() - 1).max(1) as f64;
        current_avg + (avg_change * 7.0) // Project 7 days forward
    } else {
        current_avg
    };

    info!(
        "📈 Price Trend: {:?} | Change: {:+.1}% | Rec: {:?}",
        trend, change_percent, buy_rec
    );

    Some(PriceTrendAnalysis {
        current_avg,
        week_ago_avg: week_ago_entry.map(|e| e.avg_price),
        trend,
        change_percent,
        prediction_next_week: prediction,
        buy_recommendation: buy_rec,
    })
}

/// Calculate speed metrics for offers in price range
pub fn calculate_speed_metrics(
    offers: &[Offer],
    price_range: (f64, f64),
    now: DateTime<Utc>,
) -> SpeedMetrics {
    let range_offers: Vec<_> = offers
        .iter()
        .filter(|o| o.price >= price_range.0 && o.price < price_range.1)
        .collect();

    if range_offers.is_empty() {
        return SpeedMetrics {
            price_range,
            avg_lifespan_hours: 0.0,
            disappearance_rate: 0.0,
            is_hot_market: false,
        };
    }

    // Calculate average age
    let total_age_hours: f64 = range_offers
        .iter()
        .map(|o| (now - o.posted_at).num_hours() as f64)
        .sum();
    
    let avg_lifespan = total_age_hours / range_offers.len() as f64;
    
    // Estimate disappearance rate (offers per hour)
    // Assuming we track when offers disappear in the future
    let disappearance_rate = if avg_lifespan > 0.0 {
        range_offers.len() as f64 / avg_lifespan
    } else {
        0.0
    };

    // Hot market if offers disappear quickly (< 24 hours average)
    let is_hot = avg_lifespan < 24.0 && disappearance_rate > 0.5;

    info!(
        "⚡ Speed Metrics [{:.0}-{:.0}€]: Avg lifespan: {:.1}h | Rate: {:.2} offers/h | Hot: {}",
        price_range.0, price_range.1, avg_lifespan, disappearance_rate, is_hot
    );

    SpeedMetrics {
        price_range,
        avg_lifespan_hours: avg_lifespan,
        disappearance_rate,
        is_hot_market: is_hot,
    }
}

/// Analyze market saturation
pub fn analyze_saturation(current_count: usize, historical_avg: f64) -> SaturationAnalysis {
    let ratio = if historical_avg > 0.0 {
        current_count as f64 / historical_avg
    } else {
        1.0
    };

    let (level, intensity) = if ratio < 0.7 {
        (SaturationLevel::Low, 25.0)
    } else if ratio < 1.3 {
        (SaturationLevel::Medium, 50.0)
    } else if ratio < 2.0 {
        (SaturationLevel::High, 75.0)
    } else {
        (SaturationLevel::Extreme, 95.0)
    };

    info!(
        "📊 Market Saturation: {:?} ({} offers vs {:.0} avg) | Intensity: {:.0}%",
        level, current_count, historical_avg, intensity
    );

    SaturationAnalysis {
        current_offers_count: current_count,
        avg_offers_last_week: historical_avg,
        saturation_level: level,
        competition_intensity: intensity,
    }
}

/// Generate recommendation message for Telegram
pub fn format_reseller_analysis(
    trend: &PriceTrendAnalysis,
    profit: &ProfitAnalysis,
    speed: &SpeedMetrics,
    saturation: &SaturationAnalysis,
) -> String {
    let trend_emoji = match trend.trend {
        PriceTrend::Falling => "📉",
        PriceTrend::Rising => "📈",
        PriceTrend::Stable => "➡️",
        PriceTrend::Volatile => "📊",
    };

    let rec_emoji = match trend.buy_recommendation {
        BuyRecommendation::StrongBuy => "🟢",
        BuyRecommendation::Buy => "🟡",
        BuyRecommendation::Hold => "🟠",
        BuyRecommendation::Avoid => "🔴",
    };

    let sat_emoji = match saturation.saturation_level {
        SaturationLevel::Low => "🟢",
        SaturationLevel::Medium => "🟡",
        SaturationLevel::High => "🟠",
        SaturationLevel::Extreme => "🔴",
    };

    format!(
        "💼 <b>Reseller Analysis</b>\n\n\
         {} <b>Market Trend:</b> {:?} ({:+.1}%)\n\
         {} <b>Recommendation:</b> {:?}\n\
         <b>Predicted Price:</b> {:.2}€ (next week)\n\n\
         💰 <b>Profit Analysis</b> (if sell at {:.0}€):\n\
         • Net Profit: <b>{:.2}€</b>\n\
         • ROI: <b>{:.1}%</b>\n\
         • Total Costs: {:.2}€\n\n\
         ⚡ <b>Speed:</b> {:.1}h avg | {} Market\n\
         {} <b>Saturation:</b> {:?} ({} offers)\n\n\
         💡 <b>Action:</b> {}",
        trend_emoji,
        trend.trend,
        trend.change_percent,
        rec_emoji,
        trend.buy_recommendation,
        trend.prediction_next_week,
        profit.sell_price,
        profit.net_profit,
        profit.roi_percent,
        profit.total_costs,
        speed.avg_lifespan_hours,
        if speed.is_hot_market { "🔥 HOT" } else { "❄️ Cold" },
        sat_emoji,
        saturation.saturation_level,
        saturation.current_offers_count,
        get_action_recommendation(trend, profit, speed, saturation)
    )
}

fn get_action_recommendation(
    trend: &PriceTrendAnalysis,
    profit: &ProfitAnalysis,
    speed: &SpeedMetrics,
    saturation: &SaturationAnalysis,
) -> &'static str {
    if !profit.is_profitable {
        return "❌ Not profitable, skip";
    }

    match (trend.buy_recommendation, speed.is_hot_market, saturation.saturation_level) {
        (BuyRecommendation::StrongBuy, true, SaturationLevel::Low) => {
            "🚀 BUY NOW! Perfect conditions"
        }
        (BuyRecommendation::StrongBuy, _, _) | (BuyRecommendation::Buy, true, _) => {
            "✅ Good opportunity, act fast"
        }
        (BuyRecommendation::Buy, false, SaturationLevel::Medium) => {
            "👍 Decent deal, consider buying"
        }
        (BuyRecommendation::Hold, _, _) => {
            "⏸️ Wait for better prices"
        }
        (BuyRecommendation::Avoid, _, _) | (_, _, SaturationLevel::Extreme) => {
            "⛔ Risky market, avoid"
        }
        _ => "🤔 Neutral, use your judgment"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profit_calculation() {
        let params = ProfitParams {
            target_sell_price: 400.0,
            ..Default::default()
        };

        let profit = calculate_profit(250.0, &params);
        
        assert!(profit.is_profitable);
        assert!(profit.roi_percent > 0.0);
        assert_eq!(profit.buy_price, 250.0);
    }

    #[test]
    fn test_unprofitable_deal() {
        let params = ProfitParams {
            target_sell_price: 260.0,
            ..Default::default()
        };

        let profit = calculate_profit(250.0, &params);
        
        // With fees, shipping, time cost, this should be unprofitable
        assert!(!profit.is_profitable);
    }
}

