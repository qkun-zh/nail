pub mod config;
pub mod error;
pub mod rate_limit;
pub mod smtp;

use std::future::Future;
use std::path::Path;
use std::pin::Pin;

pub use config::EmailerConfig;
pub use error::SendEmailError;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait EmailSender: Send + Sync + 'static {
    fn send<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        body: &'a str,
    ) -> BoxFuture<'a, Result<(), SendEmailError>>;
}

pub struct Emailer {
    sender: Box<dyn EmailSender>,
    global: rate_limit::GlobalLimiter,
    per_recipient: rate_limit::PerRecipientLimiter,
}

impl Emailer {
    /// Load configuration from a TOML file and build an [`Emailer`].
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, parsed, or validated.
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let config = EmailerConfig::load(path)?;
        Ok(Self::new(&config))
    }

    #[must_use]
    pub fn new(config: &EmailerConfig) -> Self {
        let sender = Box::new(smtp::SmtpClient::new(config.clone()));
        Self::build(sender, config)
    }

    #[must_use]
    pub fn with_sender(sender: Box<dyn EmailSender>, config: &EmailerConfig) -> Self {
        Self::build(sender, config)
    }

    /// Send an email and return its id.
    ///
    /// # Errors
    ///
    /// Returns [`SendEmailError::RateLimited`] if global or per-recipient
    /// rate limits are exceeded, or [`SendEmailError::Transport`] on
    /// SMTP failure.
    pub async fn send(&self, to_where: &str, send_what: &str) -> Result<String, SendEmailError> {
        self.global.check()?;
        let key = to_where.to_string();
        self.per_recipient.check_key(&key)?;

        let email_id = uuid::Uuid::now_v7().to_string();
        self.sender.send(to_where, &email_id, send_what).await?;
        Ok(email_id)
    }

    fn build(sender: Box<dyn EmailSender>, config: &EmailerConfig) -> Self {
        Self {
            sender,
            global: rate_limit::build_global(config.global_max_per_minute),
            per_recipient: rate_limit::build_per_recipient(config.per_recipient_cooldown_secs),
        }
    }
}
