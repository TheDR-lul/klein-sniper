//! # Scraper Module
//!
//! Web scraping functionality with advanced features for reliability and performance.
//!
//! ## Features
//! - **Retry Logic**: Exponential backoff for failed requests
//! - **Rate Limiting**: Configurable requests per minute to avoid blocking
//! - **Random User-Agents**: Rotating user agents for stealth
//! - **Multi-page Support**: Automatic pagination with duplicate detection
//! - **Timeout Handling**: Configurable request timeouts
//!
//! ## Usage
//!
//! ```rust,no_run
//! use klein_sniper::scraper::{Scraper, ScraperImpl};
//! use klein_sniper::model::ScrapeRequest;
//!
//! let scraper = ScraperImpl::with_config(&user_agents, 20, 2, 3, 30);
//! let html = scraper.fetch(&request).await?;
//! ```

pub mod fetcher;
pub mod traits;

pub use fetcher::ScraperImpl;
pub use traits::Scraper;
