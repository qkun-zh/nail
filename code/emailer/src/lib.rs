pub mod config;
pub mod error;
pub mod rate_limit;
pub mod smtp;

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::str::FromStr;
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
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let config = EmailerConfig::load(path)?;
        Self::new(&config)
    }

    pub fn new(config: &EmailerConfig) -> anyhow::Result<Self> {
        let sender = smtp::SmtpClient::new(config)?;
        Ok(Self::build(Arc::new(sender), config))
    }

    #[must_use]
    pub fn with_sender(sender: Arc<dyn EmailSender>, config: &EmailerConfig) -> Self {
        Self::build(sender, config)
    }

    pub async fn send(&self, to_where: &str, send_what: &str) -> Result<String, SendEmailError> {
        self.gc();

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

        self.gc();
        Ok(email_id)
    }

    pub fn gc(&self) {
        if let Some(ref pr) = self.per_recipient {
            pr.retain_recent();
            pr.shrink_to_fit();
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.per_recipient.as_ref().map_or(0, |pr| pr.len())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.per_recipient.as_ref().is_none_or(|pr| pr.is_empty())
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
    if lettre::message::Mailbox::from_str(trimmed).is_err() {
        return Err(SendEmailError::Validation(
            "recipient address is not a valid email address".into(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockSender {
        call_count: AtomicUsize,
    }

    impl MockSender {
        fn new() -> Self {
            Self {
                call_count: AtomicUsize::new(0),
            }
        }
    }

    impl EmailSender for MockSender {
        fn send<'a>(
            &'a self,
            _to: &'a str,
            _subject: &'a str,
            _body: &'a str,
        ) -> BoxFuture<'a, Result<(), SendEmailError>> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(()) })
        }

        fn clone_box(&self) -> Box<dyn EmailSender> {
            Box::new(Self {
                call_count: AtomicUsize::new(self.call_count.load(Ordering::SeqCst)),
            })
        }
    }

    struct FailingSender;

    impl EmailSender for FailingSender {
        fn send<'a>(
            &'a self,
            _to: &'a str,
            _subject: &'a str,
            _body: &'a str,
        ) -> BoxFuture<'a, Result<(), SendEmailError>> {
            Box::pin(async { Err(SendEmailError::Transport("smtp down".into())) })
        }

        fn clone_box(&self) -> Box<dyn EmailSender> {
            Box::new(Self)
        }
    }

    fn test_config() -> EmailerConfig {
        EmailerConfig {
            host: "smtp.example.com".into(),
            port: 587,
            username: "user".into(),
            password: "pass".into(),
            from_email: "from@example.com".into(),
            from_name: "test".into(),
            timeout_secs: 5,
            wall_clock_timeout_secs: 10,
            starttls: true,
            per_recipient_cooldown_secs: 0,
            global_max_per_minute: 0,
        }
    }

    fn limited_config() -> EmailerConfig {
        EmailerConfig {
            per_recipient_cooldown_secs: 60,
            global_max_per_minute: 2,
            ..test_config()
        }
    }

    #[test]
    fn reject_empty_email() {
        assert!(validate_email("").is_err());
        assert!(validate_email("  ").is_err());
    }

    #[test]
    fn reject_no_at_sign() {
        assert!(validate_email("userexample.com").is_err());
    }

    #[test]
    fn reject_too_long() {
        let long = format!("{}@example.com", "a".repeat(321));
        assert!(validate_email(&long).is_err());
    }

    #[test]
    fn accept_valid_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email(" a@b.c ").is_ok());
    }

    #[test]
    fn reject_empty_body() {
        assert!(validate_body("").is_err());
    }

    #[test]
    fn reject_oversized_body() {
        let big = "x".repeat(MAX_BODY_BYTES + 1);
        assert!(validate_body(&big).is_err());
    }

    #[test]
    fn accept_normal_body() {
        assert!(validate_body("hello").is_ok());
        assert!(validate_body(&"x".repeat(MAX_BODY_BYTES)).is_ok());
    }

    #[test]
    fn error_display() {
        assert_eq!(SendEmailError::RateLimited.to_string(), "rate limited");
        assert!(
            SendEmailError::Validation("bad".into())
                .to_string()
                .contains("bad")
        );
        assert!(
            SendEmailError::Transport("smtp".into())
                .to_string()
                .contains("smtp")
        );
    }

    #[tokio::test]
    async fn send_returns_email_id() {
        let sender = Arc::new(MockSender::new());
        let emailer = Emailer::with_sender(sender.clone(), &test_config());
        let id = emailer.send("user@example.com", "hi").await.unwrap();
        assert!(!id.is_empty());
        assert_eq!(sender.call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn send_propagates_transport_error() {
        let sender = Arc::new(FailingSender);
        let emailer = Emailer::with_sender(sender, &test_config());
        let err = emailer.send("u@x.com", "body").await.unwrap_err();
        assert!(matches!(err, SendEmailError::Transport(_)));
    }

    #[tokio::test]
    async fn invalid_input_does_not_consume_rate_limit() {
        let sender = Arc::new(MockSender::new());
        let emailer = Emailer::with_sender(sender.clone(), &limited_config());
        assert!(emailer.send("", "body").await.is_err());
        assert!(emailer.send("bad", "body").await.is_err());
        assert!(emailer.send("ok@x.com", "").await.is_err());
        assert_eq!(emailer.len(), 0);
    }

    #[tokio::test]
    async fn per_recipient_rate_limit() {
        let sender = Arc::new(MockSender::new());
        let emailer = Emailer::with_sender(sender.clone(), &limited_config());
        assert!(emailer.send("a@x.com", "m1").await.is_ok());
        let err = emailer.send("a@x.com", "m2").await.unwrap_err();
        assert!(matches!(err, SendEmailError::RateLimited));
        assert_eq!(emailer.len(), 1);
    }

    #[tokio::test]
    async fn global_rate_limit() {
        let sender = Arc::new(MockSender::new());
        let emailer = Emailer::with_sender(sender.clone(), &limited_config());
        assert!(emailer.send("a@x.com", "m1").await.is_ok());
        assert!(emailer.send("b@x.com", "m2").await.is_ok());
        let err = emailer.send("c@x.com", "m3").await.unwrap_err();
        assert!(matches!(err, SendEmailError::RateLimited));
    }

    #[tokio::test]
    async fn different_recipients_independent() {
        let sender = Arc::new(MockSender::new());
        let emailer = Emailer::with_sender(sender.clone(), &limited_config());
        assert!(emailer.send("a@x.com", "m1").await.is_ok());
        assert!(emailer.send("b@x.com", "m2").await.is_ok());
        assert_eq!(emailer.len(), 2);
    }

    #[tokio::test]
    async fn gc_removes_stale_entries() {
        let sender = Arc::new(MockSender::new());
        let mut cfg = test_config();
        cfg.per_recipient_cooldown_secs = 1;
        let emailer = Emailer::with_sender(sender, &cfg);

        emailer.send("a@x.com", "m1").await.unwrap();
        assert_eq!(emailer.len(), 1);

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        emailer.gc();
        assert_eq!(emailer.len(), 0);
    }

    #[tokio::test]
    async fn gc_preserves_active_entries() {
        let sender = Arc::new(MockSender::new());
        let mut cfg = test_config();
        cfg.per_recipient_cooldown_secs = 10;
        let emailer = Emailer::with_sender(sender, &cfg);

        emailer.send("a@x.com", "m1").await.unwrap();
        emailer.gc();
        assert_eq!(emailer.len(), 1);
    }

    #[tokio::test]
    async fn auto_gc_before_send_cleans_stale() {
        let sender = Arc::new(MockSender::new());
        let mut cfg = test_config();
        cfg.per_recipient_cooldown_secs = 1;
        let emailer = Emailer::with_sender(sender, &cfg);

        emailer.send("a@x.com", "m1").await.unwrap();
        assert_eq!(emailer.len(), 1);

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        emailer.send("a@x.com", "m2").await.unwrap();
        assert_eq!(emailer.len(), 1);
    }

    #[test]
    fn gc_without_per_recipient_does_not_panic() {
        let emailer = Emailer::with_sender(Arc::new(MockSender::new()), &test_config());
        emailer.gc();
        assert!(emailer.is_empty());
    }

    #[test]
    fn len_is_zero_when_no_limiter() {
        let emailer = Emailer::with_sender(Arc::new(MockSender::new()), &test_config());
        assert_eq!(emailer.len(), 0);
        assert!(emailer.is_empty());
    }

    #[tokio::test]
    async fn zero_cooldown_allows_burst() {
        let sender = Arc::new(MockSender::new());
        let emailer = Emailer::with_sender(sender.clone(), &test_config());
        for i in 0..100 {
            emailer.send("a@x.com", &i.to_string()).await.unwrap();
        }
        assert_eq!(sender.call_count.load(Ordering::SeqCst), 100);
    }
}
