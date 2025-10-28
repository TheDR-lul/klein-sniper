//! # Parser Module
//!
//! HTML parsing functionality for extracting structured offer data from kleinanzeigen.de.
//!
//! ## Features
//! - Optimized CSS selector caching for performance
//! - Keyword-based filtering during parsing
//! - Price range validation
//! - Extraction of offer metadata (title, price, location, description, seller info)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use klein_sniper::parser::KleinanzeigenParser;
//! use klein_sniper::config::ModelConfig;
//!
//! let parser = KleinanzeigenParser::new();
//! let offers = parser.parse_filtered(&html, &config)?;
//! ```

pub mod klein_parser;

pub use klein_parser::KleinanzeigenParser;