pub mod config;
pub mod error;
pub mod rate_limit;
pub mod smtp;

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

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

    fn clone_box(&self) -> Box<dyn EmailSender>;
}

impl Clone for Box<dyn EmailSender> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub struct Emailer {
    sender: Arc<dyn EmailSender>,
    global: Option<Arc<rate_limit::GlobalLimiter>>,
    per_recipient: Option<Arc<rate_limit::PerRecipientLimiter>>,
}

impl Clone for Emailer {
    fn clone(&self) -> Self {
        Self {
            sender: Arc::clone(&self.sender),
            global: self.global.as_ref().map(Arc::clone),
            per_recipient: self.per_recipient.as_ref().map(Arc::clone),
        }
    }
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
        let sender = Arc::new(smtp::SmtpClient::new(config.clone()));
        Self::build(sender, config)
    }

    #[must_use]
    pub fn with_sender(sender: Arc<dyn EmailSender>, config: &EmailerConfig) -> Self {
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
        validate_email(to_where)?;
        validate_body(send_what)?;

        if let Some(ref g) = self.global {
            g.check()?;
        }
        if let Some(ref pr) = self.per_recipient {
            pr.check_key(&to_where.to_string())?;
        }

        let email_id = uuid::Uuid::now_v7().to_string();
        self.sender.send(to_where, &email_id, send_what).await?;
        Ok(email_id)
    }

    fn build(sender: Arc<dyn EmailSender>, config: &EmailerConfig) -> Self {
        Self {
            sender,
            global: rate_limit::build_global(config.global_max_per_minute).map(Arc::new),
            per_recipient: rate_limit::build_per_recipient(config.per_recipient_cooldown_secs)
                .map(Arc::new),
        }
    }
}

const MAX_EMAIL_ADDR_LEN: usize = 320;
const MAX_BODY_BYTES: usize = 1 << 20;

fn validate_email(email: &str) -> Result<(), SendEmailError> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err(SendEmailError::Validation(
            "recipient address must not be empty".into(),
        ));
    }
    if trimmed.len() > MAX_EMAIL_ADDR_LEN {
        return Err(SendEmailError::Validation(format!(
            "recipient address too long ({} > {MAX_EMAIL_ADDR_LEN})",
            trimmed.len(),
        )));
    }
    if !trimmed.contains('@') {
        return Err(SendEmailError::Validation(
            "recipient address must contain '@'".into(),
        ));
    }
    Ok(())
}

fn validate_body(body: &str) -> Result<(), SendEmailError> {
    if body.is_empty() {
        return Err(SendEmailError::Validation(
            "email body must not be empty".into(),
        ));
    }
    if body.len() > MAX_BODY_BYTES {
        return Err(SendEmailError::Validation(format!(
            "email body too long ({} > {MAX_BODY_BYTES})",
            body.len(),
        )));
    }
    Ok(())
}
