use std::str::FromStr;
use std::time::Duration;

use anyhow::Context;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::EmailerConfig;
use crate::error::SendEmailError;
use crate::{BoxFuture, EmailSender};

pub struct SmtpClient {
    config: EmailerConfig,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpClient {
    pub fn new(config: &EmailerConfig) -> anyhow::Result<Self> {
        Ok(Self {
            config: config.clone(),
            transport: build_transport(config)?,
        })
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
            let from = from_mailbox(&self.config)?;
            let to = Mailbox::from_str(to)
                .map_err(|e| SendEmailError::Transport(format!("invalid to address: {e}")))?;
            let email = Message::builder()
                .from(from)
                .to(to)
                .subject(subject)
                .body(body.to_owned())
                .map_err(|e| SendEmailError::Transport(format!("failed to build message: {e}")))?;
            let wall_clock = Duration::from_secs(self.config.wall_clock_timeout_secs);
            match tokio::time::timeout(wall_clock, self.transport.send(email)).await {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(error)) => Err(SendEmailError::Transport(error.to_string())),
                Err(_) => Err(SendEmailError::Transport(format!(
                    "email send timed out after {wall_clock:?}"
                ))),
            }
        })
    }

    fn clone_box(&self) -> Box<dyn crate::EmailSender> {
        Box::new(Self {
            config: self.config.clone(),
            transport: self.transport.clone(),
        })
    }
}

fn from_mailbox(config: &EmailerConfig) -> Result<Mailbox, SendEmailError> {
    let address: Mailbox = config
        .from_email
        .parse()
        .map_err(|e| SendEmailError::Transport(format!("invalid from address: {e}")))?;
    if config.from_name.is_empty() {
        return Ok(address);
    }
    Ok(Mailbox::new(Some(config.from_name.clone()), address.email))
}

fn build_transport(config: &EmailerConfig) -> anyhow::Result<AsyncSmtpTransport<Tokio1Executor>> {
    let credentials = Credentials::new(config.username.clone(), config.password.clone());
    let timeout = Duration::from_secs(config.timeout_secs);
    if config.starttls {
        Ok(
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                .context("failed to init SMTP relay")?
                .port(config.port)
                .credentials(credentials)
                .timeout(Some(timeout))
                .build(),
        )
    } else {
        Ok(
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
                .port(config.port)
                .timeout(Some(timeout))
                .build(),
        )
    }
}
