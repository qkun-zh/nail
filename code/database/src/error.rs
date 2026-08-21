use std::fmt;

use agdb::DbError;

use crate::kinds::NodeKind;

/// Single error type for every database operation.
///
/// Rule: an absent required entity is `NotFound`; a present entity with
/// inconsistent or missing row keys is `Invalid`.
#[derive(Debug)]
pub enum Error {
    NotFound { kind: NodeKind, id: String },
    Conflict(String),
    Invalid(String),
    Storage(String),
}

impl Error {
    pub(crate) fn not_found(kind: NodeKind, id: impl Into<String>) -> Self {
        Self::NotFound {
            kind,
            id: id.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { kind, id } => write!(f, "not found: {} {id}", kind.key()),
            Self::Conflict(message) => write!(f, "conflict: {message}"),
            Self::Invalid(message) => write!(f, "invalid: {message}"),
            Self::Storage(message) => write!(f, "storage error: {message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<DbError> for Error {
    fn from(error: DbError) -> Self {
        Self::Storage(error.to_string())
    }
}
