use crate::model::Offer;
use chrono::{Datelike, Timelike, Weekday};
use std::collections::HashMap;
use tracing::info;

/// Timing patterns for offers
#[derive(Debug, Clone)]
pub struct TimingAnalysis {
    pub best_hour: u32,
    pub best_day: Weekday,
    pub hourly_distribution: HashMap<u32, usize>,
    pub daily_distribution: HashMap<Weekday, usize>,
    pub avg_offers_per_hour: f64,
}

/// Analyze posting times to find patterns
pub fn analyze_posting_times(offers: &[Offer]) -> Option<TimingAnalysis> {
    if offers.is_empty() {
        return None;
    }

    let mut hourly: HashMap<u32, usize> = HashMap::new();
    let mut daily: HashMap<Weekday, usize> = HashMap::new();

    for offer in offers {
        let hour = offer.posted_at.hour();
        let weekday = offer.posted_at.weekday();

        *hourly.entry(hour).or_insert(0) += 1;
        *daily.entry(weekday).or_insert(0) += 1;
    }

    // Find peak times
    let best_hour = hourly
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(h, _)| *h)
        .unwrap_or(12);

    let best_day = daily
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(d, _)| *d)
        .unwrap_or(Weekday::Mon);

    let total_hours = hourly.len() as f64;
    let avg_offers_per_hour = if total_hours > 0.0 {
        offers.len() as f64 / total_hours
    } else {
        0.0
    };

    info!(
        "📊 Timing Analysis: Best hour: {}:00 | Best day: {:?} | Avg offers/hour: {:.1}",
        best_hour, best_day, avg_offers_per_hour
    );

    Some(TimingAnalysis {
        best_hour,
        best_day,
        hourly_distribution: hourly,
        daily_distribution: daily,
        avg_offers_per_hour,
    })
}

/// Get recommendations for best checking times
pub fn get_recommendations(analysis: &TimingAnalysis) -> String {
    let peak_hours: Vec<u32> = analysis
        .hourly_distribution
        .iter()
        .filter(|(_, count)| **count as f64 > analysis.avg_offers_per_hour * 0.8)
        .map(|(h, _)| *h)
        .collect();

    let peak_days: Vec<Weekday> = analysis
        .daily_distribution
        .iter()
        .filter(|(_, count)| {
            **count as f64
                > (analysis.daily_distribution.values().sum::<usize>() as f64
                    / analysis.daily_distribution.len() as f64)
                    * 0.8
        })
        .map(|(d, _)| *d)
        .collect();

    let mut recommendations = format!(
        "🕐 <b>Best Time to Check:</b>\n\n\
         ⏰ Peak Hour: {}:00\n\
         📅 Peak Day: {:?}\n\n\
         🔥 <b>High Activity Hours:</b> ",
        analysis.best_hour, analysis.best_day
    );

    if !peak_hours.is_empty() {
        recommendations.push_str(&format!(
            "{}\n\n",
            peak_hours
                .iter()
                .map(|h| format!("{}:00", h))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    recommendations.push_str("📊 <b>High Activity Days:</b> ");
    if !peak_days.is_empty() {
        recommendations.push_str(&format!(
            "{}\n\n",
            peak_days
                .iter()
                .map(|d| format!("{:?}", d))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    recommendations.push_str(&format!(
        "💡 Tip: Check more frequently during peak times for best deals!\n\
         Average: {:.1} new offers per hour",
        analysis.avg_offers_per_hour
    ));

    recommendations
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_timing_analysis() {
        let offers = vec![
            Offer {
                posted_at: Utc::now(),
                ..Default::default()
            },
            Offer {
                posted_at: Utc::now(),
                ..Default::default()
            },
        ];

        let analysis = analyze_posting_times(&offers);
        assert!(analysis.is_some());

        let analysis = analysis.unwrap();
        assert!(analysis.best_hour < 24);
        assert!(analysis.avg_offers_per_hour > 0.0);
    }

    #[test]
    fn test_empty_offers() {
        let offers: Vec<Offer> = vec![];
        let analysis = analyze_posting_times(&offers);
        assert!(analysis.is_none());
    }
}

