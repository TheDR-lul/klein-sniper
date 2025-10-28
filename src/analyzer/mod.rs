//! # Analyzer Module
//!
//! This module provides comprehensive offer analysis capabilities including:
//! - **Price Analysis**: Statistical analysis of offer prices (mean, std deviation, deal detection)
//! - **Market Indicators**: Advanced market metrics (RSI, volatility, disappearance speed)
//! - **Lifecycle Tracking**: Monitoring offer lifecycle and market dynamics
//!
//! ## Usage
//!
//! ```rust,no_run
//! use klein_sniper::analyzer::{AnalyzerImpl, price_analysis::Analyzer};
//! use klein_sniper::model::Offer;
//!
//! let analyzer = AnalyzerImpl::new();
//! let stats = analyzer.calculate_stats(&offers);
//! let deals = analyzer.find_deals(&offers, &stats, &config);
//! ```

pub mod price_analysis;
pub mod market_indicators;
pub mod lifecycle;

// Re-export the main Analyzer implementation for ease of use.
pub use price_analysis::AnalyzerImpl;