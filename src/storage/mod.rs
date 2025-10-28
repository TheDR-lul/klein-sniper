//! # Storage Module
//!
//! Persistent storage layer using SQLite with optimizations and automatic cleanup.
//!
//! ## Features
//! - **WAL Mode**: Write-Ahead Logging for better concurrency
//! - **Automatic Cleanup**: Configurable retention periods for offers and stats
//! - **Indexed Queries**: Optimized database indexes for fast searches
//! - **Notification Tracking**: Deduplication of sent notifications
//! - **Statistics Storage**: Persistent model statistics and analytics
//!
//! ## Usage
//!
//! ```rust,no_run
//! use klein_sniper::storage::SqliteStorage;
//!
//! let storage = SqliteStorage::new("data.db")?;
//! storage.save_offer(&offer)?;
//! let stats = storage.get_stats("iPhone 13")?;
//! ```

pub mod sqlite;

pub use sqlite::SqliteStorage;