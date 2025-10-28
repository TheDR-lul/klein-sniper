//! # Notifier Module
//!
//! Telegram bot integration for sending notifications and handling commands.
//!
//! ## Features
//! - **teloxide Framework**: Modern async Telegram bot library
//! - **Command Handler**: Interactive bot commands (/ping, /status, /top5, etc.)
//! - **HTML Formatting**: Rich message formatting with links and emojis
//! - **Manual Refresh**: Trigger re-scanning via /refresh command
//! - **Statistics**: Real-time bot statistics and uptime tracking
//!
//! ## Usage
//!
//! ```rust,no_run
//! use klein_sniper::notifier::TelegramNotifier;
//!
//! let notifier = TelegramNotifier::new(token, chat_id, storage, config, notify);
//! notifier.notify(&offer).await?;
//! TelegramNotifier::spawn_listener(Arc::new(notifier));
//! ```

pub mod telegram;

pub use telegram::TelegramNotifier;