// Utility functions
use chrono::{DateTime, Utc};
use std::time::Duration;
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};

/// Parse datetime from RFC3339 string
pub fn parse_datetime(date_str: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(date_str)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Convert string to kebab-case
pub fn to_kebab_case(text: &str) -> String {
    text.to_lowercase().replace(' ', "-")
}

/// Create exponential backoff configuration for retries
pub fn create_backoff() -> ExponentialBackoff {
    ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_secs(1))
        .with_max_interval(Duration::from_secs(30))
        .with_multiplier(2.0)
        .with_max_elapsed_time(Some(Duration::from_secs(300))) // 5 minutes max
        .build()
}

/// Format duration in human-readable format
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("Hello World"), "hello-world");
        assert_eq!(to_kebab_case("ROG Ally"), "rog-ally");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(45)), "45s");
        assert_eq!(format_duration(Duration::from_secs(125)), "2m 5s");
        assert_eq!(format_duration(Duration::from_secs(7325)), "2h 2m 5s");
    }
}