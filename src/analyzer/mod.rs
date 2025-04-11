// Analyzer module: aggregates submodules for different aspects of analysis.

pub mod price_analysis;
pub mod market_indicators;
pub mod lifecycle;

// Re-export the main Analyzer implementation for ease of use.
pub use price_analysis::AnalyzerImpl;