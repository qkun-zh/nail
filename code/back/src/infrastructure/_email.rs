pub mod smtp;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use smtp::SmtpSender;

use crate::infrastructure::config::smtp::SmtpConfig;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug)]
pub enum SendEmailError {
    RateLimited,
    Transport(anyhow::Error),
}

pub trait EmailSender: Send + Sync + 'static {
    fn send<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        body: &'a str,
    ) -> BoxFuture<'a, Result<(), SendEmailError>>;
}

#[derive(Clone)]
pub struct RateLimitedSender {
    inner: Arc<dyn EmailSender>,
    cooldown: Duration,
    last_sent: Arc<Mutex<HashMap<String, Instant>>>,
}

impl RateLimitedSender {
    pub fn new(inner: Arc<dyn EmailSender>, cooldown_seconds: u64) -> Self {
        Self {
            inner,
            cooldown: Duration::from_secs(cooldown_seconds),
            last_sent: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn smtp(config: &SmtpConfig, cooldown_seconds: u64) -> Self {
        Self::new(Arc::new(SmtpSender::new(config.clone())), cooldown_seconds)
    }

    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), SendEmailError> {
        if let Some(previous) = self
            .last_sent
            .lock()
            .map_err(|_| SendEmailError::Transport(anyhow::anyhow!("email rate-lock poisoned")))?
            .get(to)
            && previous.elapsed() < self.cooldown
        {
            return Err(SendEmailError::RateLimited);
        }
        self.last_sent
            .lock()
            .map_err(|_| SendEmailError::Transport(anyhow::anyhow!("email rate-lock poisoned")))?
            .insert(to.to_string(), Instant::now());
        self.inner.send(to, subject, body).await
    }
}
