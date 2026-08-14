use std::sync::Arc;

use super::context::RecordingSender;
use crate::infrastructure::email::{EmailSender, RateLimitedSender, SendEmailError};

#[derive(Clone)]
struct FailingSender;

impl EmailSender for FailingSender {
    fn send<'a>(
        &'a self,
        _to: &'a str,
        _subject: &'a str,
        _body: &'a str,
    ) -> crate::infrastructure::email::BoxFuture<'a, Result<(), SendEmailError>> {
        Box::pin(async move { Err(SendEmailError::Transport(anyhow::anyhow!("smtp down"))) })
    }
}

#[tokio::test]
async fn rate_limited_sender_blocks_repeat_sends_within_the_cooldown() {
    let recorder = RecordingSender::default();
    let sender = RateLimitedSender::new(Arc::new(recorder.clone()), 60);

    sender
        .send_email("alice@example.com", "subject", "body")
        .await
        .expect("first send");
    let second = sender
        .send_email("alice@example.com", "subject", "body")
        .await;
    assert!(matches!(second, Err(SendEmailError::RateLimited)));
    assert_eq!(recorder.sent.lock().expect("lock").len(), 1);
}

#[tokio::test]
async fn rate_limited_sender_allows_different_recipients() {
    let recorder = RecordingSender::default();
    let sender = RateLimitedSender::new(Arc::new(recorder.clone()), 60);

    sender
        .send_email("alice@example.com", "subject", "body")
        .await
        .expect("first");
    sender
        .send_email("bob@example.com", "subject", "body")
        .await
        .expect("second");
    assert_eq!(recorder.sent.lock().expect("lock").len(), 2);
}

#[tokio::test]
async fn zero_cooldown_disables_rate_limiting() {
    let recorder = RecordingSender::default();
    let sender = RateLimitedSender::new(Arc::new(recorder.clone()), 0);

    sender
        .send_email("alice@example.com", "subject", "body")
        .await
        .expect("first");
    sender
        .send_email("alice@example.com", "subject", "body")
        .await
        .expect("second");
    assert_eq!(recorder.sent.lock().expect("lock").len(), 2);
}

#[tokio::test]
async fn transport_failure_is_propagated() {
    let sender = RateLimitedSender::new(Arc::new(FailingSender), 0);
    let result = sender
        .send_email("alice@example.com", "subject", "body")
        .await;
    assert!(matches!(result, Err(SendEmailError::Transport(_))));
}
