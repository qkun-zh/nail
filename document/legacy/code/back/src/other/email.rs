mod email_core;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::other::conf::SmtpConfig;

#[derive(Clone)]
pub struct EmailService {
    smtp: SmtpConfig,
    cooldown: Duration,
    last_sent: Arc<Mutex<HashMap<String, Instant>>>,
}

#[derive(Debug)]
pub enum SendEmailError {
    RateLimited,
    Smtp(anyhow::Error),
}

impl EmailService {
    pub fn new(smtp: SmtpConfig, cooldown_seconds: u64) -> Self {
        Self {
            smtp,
            cooldown: Duration::from_secs(cooldown_seconds),
            last_sent: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), SendEmailError> {
        {
            let mut last_sent = self
                .last_sent
                .lock()
                .map_err(|_| SendEmailError::Smtp(anyhow::anyhow!("email rate-lock poisoned")))?;
            if let Some(prev) = last_sent.get(to)
                && prev.elapsed() < self.cooldown
            {
                return Err(SendEmailError::RateLimited);
            }
            last_sent.insert(to.to_string(), Instant::now());
        }
        email_core::send_email(&self.smtp, to, subject, body)
            .await
            .map_err(SendEmailError::Smtp)
    }
}
