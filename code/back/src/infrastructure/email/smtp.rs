use std::time::Duration;

use anyhow::{Context, Result};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::infrastructure::config::smtp::SmtpConfig;
use crate::infrastructure::email::{BoxFuture, EmailSender, SendEmailError};

#[derive(Clone)]
pub struct SmtpSender {
    config: SmtpConfig,
}

impl SmtpSender {
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }
}

impl EmailSender for SmtpSender {
    fn send<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        body: &'a str,
    ) -> BoxFuture<'a, Result<(), SendEmailError>> {
        Box::pin(async move {
            let config = self.config.clone();
            let to = to.to_string();
            let subject = subject.to_string();
            let body = body.to_string();
            let wall_clock = Duration::from_secs(config.wall_clock_timeout_secs);
            let task = tokio::task::spawn_blocking(move || {
                send_blocking(&config, &to, &subject, &body)
            });
            match tokio::time::timeout(wall_clock, task).await {
                Ok(Ok(result)) => result.map_err(SendEmailError::Transport),
                Ok(Err(join_error)) => Err(SendEmailError::Transport(anyhow::anyhow!(
                    "email send task failed: {join_error}"
                ))),
                Err(_) => Err(SendEmailError::Transport(anyhow::anyhow!(
                    "email send timed out after {wall_clock:?}"
                ))),
            }
        })
    }
}

fn send_blocking(config: &SmtpConfig, to: &str, subject: &str, body: &str) -> Result<()> {
    let from: Mailbox = if config.from_name.is_empty() {
        config.from_email.parse().context("invalid from address")?
    } else {
        let address: Mailbox = config
            .from_email
            .parse()
            .context("invalid from address")?;
        Mailbox::new(Some(config.from_name.clone()), address.email)
    };
    let to_mailbox: Mailbox = to.parse().context("invalid to address")?;
    let email = Message::builder()
        .from(from)
        .to(to_mailbox)
        .subject(subject)
        .body(body.to_owned())?;
    let mailer = build_transport(config)?;
    mailer
        .send(&email)
        .with_context(|| format!("failed to send email to {to} via {}", config.host))?;
    Ok(())
}

fn build_transport(config: &SmtpConfig) -> Result<SmtpTransport> {
    if config.starttls {
        let credentials = Credentials::new(config.username.clone(), config.password.clone());
        Ok(SmtpTransport::starttls_relay(&config.host)
            .context("failed to init SMTP relay")?
            .port(config.port)
            .credentials(credentials)
            .timeout(Some(Duration::from_secs(config.timeout_secs)))
            .build())
    } else {
        Ok(SmtpTransport::builder_dangerous(&config.host)
            .port(config.port)
            .timeout(Some(Duration::from_secs(config.timeout_secs)))
            .build())
    }
}
