use std::time::Duration;

use anyhow::Context;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::config::EmailerConfig;
use crate::error::SendEmailError;
use crate::{BoxFuture, EmailSender};

pub struct SmtpClient {
    config: EmailerConfig,
}

impl SmtpClient {
    #[must_use]
    pub fn new(config: EmailerConfig) -> Self {
        Self { config }
    }
}

impl EmailSender for SmtpClient {
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
            let task =
                tokio::task::spawn_blocking(move || send_blocking(&config, &to, &subject, &body));
            match tokio::time::timeout(wall_clock, task).await {
                Ok(Ok(result)) => result,
                Ok(Err(join_error)) => Err(SendEmailError::Transport(format!(
                    "email send task failed: {join_error}"
                ))),
                Err(_) => Err(SendEmailError::Transport(format!(
                    "email send timed out after {wall_clock:?}"
                ))),
            }
        })
    }
}

fn send_blocking(
    config: &EmailerConfig,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), SendEmailError> {
    let from: Mailbox = if config.from_name.is_empty() {
        config
            .from_email
            .parse()
            .map_err(|e| SendEmailError::Transport(format!("invalid from address: {e}")))?
    } else {
        let address: Mailbox = config
            .from_email
            .parse()
            .map_err(|e| SendEmailError::Transport(format!("invalid from address: {e}")))?;
        Mailbox::new(Some(config.from_name.clone()), address.email)
    };
    let to_mailbox: Mailbox = to
        .parse()
        .map_err(|e| SendEmailError::Transport(format!("invalid to address: {e}")))?;
    let email = Message::builder()
        .from(from)
        .to(to_mailbox)
        .subject(subject)
        .body(body.to_owned())
        .map_err(|e| SendEmailError::Transport(format!("failed to build message: {e}")))?;
    let mailer = build_transport(config)
        .map_err(|e| SendEmailError::Transport(format!("failed to build transport: {e}")))?;
    mailer
        .send(&email)
        .map(|_| ())
        .map_err(|e| SendEmailError::Transport(format!("failed to send to {to}: {e}")))
}

fn build_transport(config: &EmailerConfig) -> anyhow::Result<SmtpTransport> {
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
