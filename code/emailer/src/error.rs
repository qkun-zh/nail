use thiserror::Error;

#[derive(Debug, Error)]
pub enum SendEmailError {
    #[error("rate limited")]
    RateLimited,
    #[error("invalid input: {0}")]
    Validation(String),
    #[error("transport error: {0}")]
    Transport(String),
}

impl From<governor::NotUntil<governor::clock::QuantaInstant>> for SendEmailError {
    fn from(_: governor::NotUntil<governor::clock::QuantaInstant>) -> Self {
        Self::RateLimited
    }
}
