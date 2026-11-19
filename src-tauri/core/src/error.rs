use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    /// Input failed validation. The message names the field and the rule.
    #[error("{0}")]
    Invalid(String),

    /// Text matched a secret-shaped pattern. Nothing was stored.
    #[error(
        "{field} looks like it contains {class}; describe it in prose instead (nothing was stored)"
    )]
    Unsafe { field: String, class: &'static str },

    /// Judgement is paused; agent participation is off.
    #[error("Judgement is paused; nothing was stored. Continue without it.")]
    Paused,

    #[error("{0}")]
    NotFound(String),

    /// The operation conflicts with current state (e.g. the learning was already decided).
    #[error("{0}")]
    Conflict(String),

    /// The store is missing, unreadable, or not a Judgement store.
    #[error("{0}")]
    Unavailable(String),

    #[error("store schema is version {found} but this build expects {expected}; open Judgement to upgrade the store")]
    SchemaMismatch { found: i64, expected: i64 },

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    pub fn invalid(message: impl Into<String>) -> Self {
        Error::Invalid(message.into())
    }

    /// True for SQLite failures that mean the store itself can't be used
    /// (locked past busy_timeout, corrupt, not a database), as opposed to a bug.
    pub fn is_store_unavailable(&self) -> bool {
        match self {
            Error::Unavailable(_) | Error::SchemaMismatch { .. } => true,
            Error::Sqlite(rusqlite::Error::SqliteFailure(e, _)) => matches!(
                e.code,
                rusqlite::ErrorCode::DatabaseBusy
                    | rusqlite::ErrorCode::DatabaseLocked
                    | rusqlite::ErrorCode::DatabaseCorrupt
                    | rusqlite::ErrorCode::NotADatabase
                    | rusqlite::ErrorCode::CannotOpen
                    | rusqlite::ErrorCode::ReadOnly
            ),
            _ => false,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
