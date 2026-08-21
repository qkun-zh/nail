use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Denied,
    NotFound,
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Denied => formatter.write_str("access denied"),
            Self::NotFound => formatter.write_str("resource not found"),
            Self::Internal(message) => write!(formatter, "authorization error: {message}"),
        }
    }
}

impl std::error::Error for Error {}
