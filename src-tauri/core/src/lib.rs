//! Judgement core: schema, migrations, validation, the unsafe-content check,
//! and every read/write operation. Linked into both the app and the CLI;
//! nothing writes to the store except through this crate.

pub mod error;
pub mod inception;
pub mod model;
pub mod ops;
pub mod repo;
pub mod safety;
pub mod store;
pub mod validate;

pub use error::{Error, Result};

/// UTC RFC3339 timestamp, seconds precision.
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn new_id() -> String {
    ulid::Ulid::new().to_string()
}

/// UTC RFC3339 timestamp `days` ago, for windowed queries like the gauge.
pub fn days_ago(days: i64) -> String {
    (chrono::Utc::now() - chrono::Duration::days(days))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
