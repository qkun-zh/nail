use std::fmt;

#[derive(Debug)]
pub enum SendEmailError {
    RateLimited,
    Transport(String),
}

impl fmt::Display for SendEmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RateLimited => write!(f, "rate limited"),
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
