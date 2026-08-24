use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    #[error("access denied")]
    Denied,
    #[error("resource not found")]
    NotFound,
    #[error("invalid authorization request: {0}")]
    InvalidRequest(String),
    #[error("authorization error: {0}")]
    Internal(String),
}
