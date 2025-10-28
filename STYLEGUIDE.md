# KleinSniper Code Style Guide

## Language Requirements

**CRITICAL:** All code, comments, documentation, and commit messages **MUST** be in English only.

## General Principles

1. **English Only**: All code, comments, docstrings, variable names, error messages, and documentation must be in English
2. **Consistency**: Follow existing patterns in the codebase
3. **Clarity**: Write clear, self-documenting code with meaningful names
4. **Rust Idioms**: Follow Rust best practices and idioms

## File Organization

```
src/
├── main.rs              # Application entry point
├── config.rs            # Configuration structures
├── model.rs             # Core data models and error types
├── analyzer/            # Offer analysis logic
│   ├── mod.rs
│   ├── price_analysis.rs
│   ├── market_indicators.rs
│   └── lifecycle.rs
├── scraper/             # Web scraping
│   ├── mod.rs
│   ├── fetcher.rs
│   └── traits.rs
├── parser/              # HTML parsing
│   ├── mod.rs
│   └── klein_parser.rs
├── storage/             # Database layer
│   ├── mod.rs
│   └── sqlite.rs
├── notifier/            # Notification system
│   └── telegram/
└── normalizer.rs        # Data normalization
```

## Code Style

### Naming Conventions

- **Types/Structs**: `PascalCase`
  ```rust
  pub struct TelegramNotifier { }
  pub enum ScraperError { }
  ```

- **Functions/Methods**: `snake_case`
  ```rust
  pub fn load_config(path: &str) -> Result<AppConfig, Box<dyn std::error::Error>>
  async fn fetch_with_retry(&self, url: &str) -> Result<String, ScraperError>
  ```

- **Constants**: `SCREAMING_SNAKE_CASE`
  ```rust
  pub const DEFAULT_STEP: u32 = 50;
  const MAX_RETRIES: u32 = 3;
  ```

- **Variables**: `snake_case`
  ```rust
  let base_scraper = ScraperImpl::with_config(...);
  let user_agents = config.scraper.user_agents;
  ```

### Comments

- **Documentation comments** (`///`) for public APIs:
  ```rust
  /// Clean up old offers based on retention period
  ///
  /// # Arguments
  /// * `retention_days` - Number of days to keep offers (0 = keep forever)
  ///
  /// # Returns
  /// Number of deleted records
  pub fn cleanup_old_offers(&self, retention_days: u32) -> Result<usize, StorageError>
  ```

- **Inline comments** (`//`) for implementation details:
  ```rust
  // Apply rate limiting before making request
  self.rate_limiter.until_ready().await;
  
  // Expected format returned by datetime('now') - "%Y-%m-%d %H:%M:%S"
  let notified_at_naive = NaiveDateTime::parse_from_str(&notified_at_str, "%Y-%m-%d %H:%M:%S")?;
  ```

### Error Handling

Use `thiserror` for custom error types:

```rust
#[derive(Debug, Error)]
pub enum ScraperError {
    #[error("🌐 HTTP error: {0}")]
    HttpError(String),
    
    #[error("❌ Invalid server response: {0}")]
    InvalidResponse(String),
    
    #[error("📄 HTML parsing error: {0}")]
    HtmlParseError(String),
}
```

### Async Functions

- Use `async`/`await` for I/O operations
- Prefer `tokio::spawn` for concurrent tasks
- Use `Arc` for shared state across async tasks

```rust
pub async fn notify(&self, offer: &Offer) -> Result<(), NotifyError> {
    sender::send_offer(self, offer).await
}
```

### Configuration

- Use `serde` for serialization/deserialization
- Provide default values with `#[serde(default = "...")]`
- Implement comprehensive validation

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct ScraperConfig {
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    
    #[serde(default = "default_delay_seconds")]
    pub delay_seconds: u64,
}

fn default_max_pages() -> usize { 20 }
fn default_delay_seconds() -> u64 { 1 }
```

## Module Organization

### Re-exports

Use `mod.rs` to re-export public items:

```rust
// analyzer/mod.rs
pub mod price_analysis;
pub mod market_indicators;
pub mod lifecycle;

pub use price_analysis::AnalyzerImpl;
```

### Visibility

- Use `pub` for public API
- Use `pub(crate)` for internal cross-module access
- Use private (no keyword) for module-internal items

```rust
pub struct TelegramNotifier { ... }           // Public API
pub(crate) rate_limiter: Arc<...>             // Internal access
fn internal_helper() { ... }                   // Private
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_calculation() {
        let analyzer = AnalyzerImpl::new();
        let offers = vec![...];
        let stats = analyzer.calculate_stats(&offers);
        
        assert_eq!(stats.avg_price, 100.0);
    }
}
```

## Dependencies

- **Minimize dependencies**: Only add what's necessary
- **Prefer well-maintained crates**: Check download counts and last update
- **Pin versions**: Use specific version numbers, not wildcards

## Database

- Use **SQLite** with `rusqlite`
- Enable **WAL mode** for better concurrency
- Add **indexes** for frequently queried columns
- Implement **cleanup/retention** policies

```rust
// Enable WAL mode
conn.execute("PRAGMA journal_mode=WAL", [])?;
conn.execute("PRAGMA synchronous=NORMAL", [])?;

// Create indexes
CREATE INDEX IF NOT EXISTS idx_offers_model ON offers(model);
CREATE INDEX IF NOT EXISTS idx_offers_price ON offers(price);
```

## Logging

Use `tracing` for structured logging:

```rust
use tracing::{info, warn, error};

info!("Starting database cleanup...");
warn!("Retry attempt {} failed", attempt);
error!("Critical error: {:?}", e);
```

## Git Commit Messages

- Use **English only**
- Format: `<type>: <description>`
- Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`

Examples:
```
feat: add database auto-cleanup functionality
fix: resolve rate limiter cloning issue
refactor: migrate Telegram bot to teloxide library
docs: update README with new configuration options
```

## Code Review Checklist

- [ ] All comments and documentation in English
- [ ] No hardcoded values (use config)
- [ ] Proper error handling (no unwrap in prod code)
- [ ] Tests added for new functionality
- [ ] Documentation updated
- [ ] No compiler warnings
- [ ] Code formatted with `rustfmt`
- [ ] Lints pass with `clippy`

## Formatting

Run before commit:
```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
```

## Common Patterns

### Shared State
```rust
let storage = Arc::new(Mutex::new(SqliteStorage::new("data.db")?));
let notifier = Arc::new(TelegramNotifier::new(...));
```

### Graceful Shutdown
```rust
tokio::select! {
    _ = main_loop() => { info!("Main loop ended"); }
    _ = tokio::signal::ctrl_c() => { info!("Shutting down..."); }
}
```

### Retry Logic
```rust
let backoff = ExponentialBackoff::default();
retry(backoff, operation).await
```

---

**Remember**: Code is read more often than written. Prioritize clarity and maintainability.

