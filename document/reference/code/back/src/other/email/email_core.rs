use anyhow::{Context, Result};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::other::conf::SmtpConfig;

fn build_transport(config: &SmtpConfig) -> Result<SmtpTransport> {
    let transport = if config.starttls {
        let creds = Credentials::new(config.username.clone(), config.password.clone());
        SmtpTransport::starttls_relay(&config.host)
            .context("failed to init SMTP relay")?
            .port(config.port)
            .credentials(creds)
            .timeout(Some(std::time::Duration::from_secs(config.timeout_secs)))
            .build()
    } else {
        SmtpTransport::builder_dangerous(&config.host)
            .port(config.port)
            .timeout(Some(std::time::Duration::from_secs(config.timeout_secs)))
            .build()
    };
    Ok(transport)
}

pub async fn send_email(config: &SmtpConfig, to: &str, subject: &str, body: &str) -> Result<()> {
    let config = config.clone();
    let wall_clock_timeout = std::time::Duration::from_secs(config.wall_clock_timeout_secs);
    let to = to.to_string();
    let subject = subject.to_string();
    let body = body.to_string();
    let task =
        tokio::task::spawn_blocking(move || send_email_blocking(&config, &to, &subject, &body));
    tokio::time::timeout(wall_clock_timeout, task)
        .await
        .map_err(|_| anyhow::anyhow!("email send timed out after {:?}", wall_clock_timeout))?
        .map_err(|e| anyhow::anyhow!("email send task panicked or cancelled: {e}"))?
}

fn send_email_blocking(config: &SmtpConfig, to: &str, subject: &str, body: &str) -> Result<()> {
    let from: Mailbox = if config.from_name.is_empty() {
        config.from_email.parse().context("invalid from address")?
    } else {
        let addr: Mailbox = config.from_email.parse().context("invalid from address")?;
        Mailbox::new(Some(config.from_name.clone()), addr.email)
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
