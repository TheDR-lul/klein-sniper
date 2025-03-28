// Utility functions
use chrono::{DateTime, Utc};

/// Преобразует строку в `DateTime<Utc>`, если возможно.
pub fn parse_datetime(date_str: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(date_str)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Преобразует строку в kebab-case.
pub fn to_kebab_case(text: &str) -> String {
    text.to_lowercase().replace(" ", "-")
}