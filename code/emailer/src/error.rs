use std::fmt;

#[derive(Debug)]
pub enum SendEmailError {
    RateLimited,
    Validation(String),
    Transport(String),
}

impl fmt::Display for SendEmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RateLimited => write!(f, "rate limited"),
            Self::Validation(msg) => write!(f, "invalid input: {msg}"),
            Self::Transport(msg) => write!(f, "transport error: {msg}"),
        }
    }
}

impl std::error::Error for SendEmailError {}

impl From<governor::NotUntil<governor::clock::QuantaInstant>> for SendEmailError {
    fn from(_: governor::NotUntil<governor::clock::QuantaInstant>) -> Self {
        Self::RateLimited
    }
}
