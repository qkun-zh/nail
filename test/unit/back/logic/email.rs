use nail_common::request::{EmailReadIntent, EmailReadRequest};

use super::context::TestCtx;
use crate::logic::email::{normalize_email, parse_intent, validate_email};
use crate::logic::error::LogicError;

#[test]
fn parse_intent_maps_the_wire_values() {
    assert_eq!(parse_intent("authenticate"), Some(EmailReadIntent::Authenticate));
    assert_eq!(parse_intent("change_email"), Some(EmailReadIntent::ChangeEmail));
    assert_eq!(parse_intent("deregister"), Some(EmailReadIntent::Deregister));
    assert_eq!(parse_intent("bogus"), None);
    assert_eq!(parse_intent(""), None);
}

#[test]
fn normalize_email_trims_and_lowercases() {
    assert_eq!(normalize_email("  Alice@Example.COM  "), "alice@example.com");
    assert_eq!(normalize_email("alice@example.com"), "alice@example.com");
}

#[test]
fn validate_email_accepts_allowed_domains_case_insensitively() {
    let allowed = vec!["example.com".to_string(), "test.org".to_string()];
    assert!(validate_email("alice@example.com", &allowed));
    assert!(validate_email("bob@EXAMPLE.COM", &allowed));
    assert!(validate_email("c@test.org", &allowed));
}

#[test]
fn validate_email_rejects_disallowed_or_malformed_addresses() {
    let allowed = vec!["example.com".to_string()];
    assert!(!validate_email("alice@other.org", &allowed));
    assert!(!validate_email("not-an-email", &allowed));
    assert!(!validate_email("", &allowed));
    assert!(!validate_email(&"a".repeat(255), &allowed));
}

#[tokio::test]
async fn authenticate_branch_sends_and_caches_a_token() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.issued_pow("alice@example.com");
    let request = EmailReadRequest {
        pow: Some(pow),
        old_email_pow: None,
        new_email_pow: None,
    };
    let data =
        crate::logic::email::handle_email_read(&context.state, EmailReadIntent::Authenticate, request)
            .await
            .expect("email read");
    let subject = data["email_subject"].as_str().expect("subject");
    assert!(!subject.is_empty());

    let messages = context.emails();
    assert_eq!(messages.len(), 1);
    let (to, message_subject, body) = &messages[0];
    assert_eq!(to, "alice@example.com");
    assert_eq!(message_subject, subject);
    let token_key = crate::repository::cache::token_key(body).expect("token key");
    assert!(context.state.caches.authenticate.read(&token_key).is_some());
}

#[tokio::test]
async fn authenticate_branch_requires_a_pow() {
    let context = TestCtx::new().await.expect("test context");
    let request = EmailReadRequest::default();
    let error =
        crate::logic::email::handle_email_read(&context.state, EmailReadIntent::Authenticate, request)
            .await
            .unwrap_err();
    assert_eq!(error, LogicError::bad_request("pow is required"));
}

#[tokio::test]
async fn authenticate_branch_rejects_a_disallowed_domain_without_burning_the_challenge() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.issued_pow("alice@other.org");
    let request = EmailReadRequest {
        pow: Some(pow.clone()),
        old_email_pow: None,
        new_email_pow: None,
    };
    let error =
        crate::logic::email::handle_email_read(&context.state, EmailReadIntent::Authenticate, request)
            .await
            .unwrap_err();
    assert_eq!(error, LogicError::bad_request("email domain not allowed"));
    assert!(context.emails().is_empty());
    assert!(context.state.caches.challenge.consume(&pow.challenge.id.to_string()).is_some());
}

#[tokio::test]
async fn unsupported_intents_are_rejected_explicitly() {
    let context = TestCtx::new().await.expect("test context");
    for intent in [EmailReadIntent::ChangeEmail, EmailReadIntent::Deregister] {
        let error = crate::logic::email::handle_email_read(
            &context.state,
            intent,
            EmailReadRequest::default(),
        )
        .await
        .unwrap_err();
        assert_eq!(error, LogicError::bad_request("email intent is not supported yet"));
    }
}
