use agdb::DbError;
use thiserror::Error;

use crate::kinds::NodeKind;

#[derive(Debug, Error)]
pub enum Error {
    #[error("not found: {} {id}", kind.key())]
    NotFound { kind: NodeKind, id: String },
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("panic: {0}")]
    Panic(String),
    #[error("storage error: {0}")]
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

impl From<DbError> for Error {
    fn from(error: DbError) -> Self {
        Self::Storage(error.to_string())
    }
}
