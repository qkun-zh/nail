use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    #[error("access denied")]
    Denied,
    #[error("resource not found")]
    NotFound,
    #[error("authorization error: {0}")]
    Internal(String),
}
