use nail_common::request::{EmailReadIntent, EmailReadRequest};

use super::context::TestCtx;
use crate::logic::email::{
    send_delete_user_email, update_user_email, send_update_user_email, normalize_email,
    parse_intent, validate_email,
};
use crate::logic::error::LogicError;
use crate::repository::cache::{SessionTokenEntry, token_key};

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

async fn session_for(context: &TestCtx, email: &str) -> (String, String) {
    let user_id = crate::repository::user::find_or_create_user(
        &context.state.graph,
        &nail_common::hash::email(email),
    )
    .await
    .expect("user");
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.session.insert(
        &key,
        SessionTokenEntry {
            user_id: user_id.clone(),
        },
    );
    (user_id, token)
}

#[tokio::test]
async fn create_user_intent_sends_and_caches_a_token() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.issued_pow("alice@example.com");
    let request = EmailReadRequest {
        pow: Some(pow),
        old_email_pow: None,
        new_email_pow: None,
    };
    let data = crate::logic::email::read_email(
        &context.state,
        EmailReadIntent::Authenticate,
        request,
        None,
    )
    .await
    .expect("email read");
    let subject = data["email_subject"].as_str().expect("subject");
    assert!(!subject.is_empty());

    let messages = context.emails();
    assert_eq!(messages.len(), 1);
    let (to, message_subject, body) = &messages[0];
    assert_eq!(to, "alice@example.com");
    assert_eq!(message_subject, subject);
    let token_key = token_key(body).expect("token key");
    assert!(context.state.caches.create_user.read(&token_key).is_some());
}

#[tokio::test]
async fn create_user_intent_requires_a_pow() {
    let context = TestCtx::new().await.expect("test context");
    let request = EmailReadRequest::default();
    let error = crate::logic::email::read_email(
        &context.state,
        EmailReadIntent::Authenticate,
        request,
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("pow is required"));
}

#[tokio::test]
async fn create_user_intent_rejects_a_disallowed_domain_without_burning_the_challenge() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.issued_pow("alice@other.org");
    let request = EmailReadRequest {
        pow: Some(pow.clone()),
        old_email_pow: None,
        new_email_pow: None,
    };
    let error = crate::logic::email::read_email(
        &context.state,
        EmailReadIntent::Authenticate,
        request,
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("email domain not allowed"));
    assert!(context.emails().is_empty());
    assert!(context.state.caches.challenge.consume(&pow.challenge.id.to_string()).is_some());
}

#[tokio::test]
async fn change_email_requires_a_session() {
    let context = TestCtx::new().await.expect("test context");
    let error = crate::logic::email::read_email(
        &context.state,
        EmailReadIntent::ChangeEmail,
        EmailReadRequest::default(),
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::unauthorized("missing session-token header"));
}

#[tokio::test]
async fn change_email_sends_two_emails_and_caches_the_token_hashes() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, session_token) = session_for(&context, "alice@example.com").await;
    let old_pow = context.issued_pow("alice@example.com");
    let new_pow = context.issued_pow("alice-new@example.com");
    let request = EmailReadRequest {
        pow: None,
        old_email_pow: Some(old_pow),
        new_email_pow: Some(new_pow),
    };
    let data = crate::logic::email::read_email(
        &context.state,
        EmailReadIntent::ChangeEmail,
        request,
        Some(session_token),
    )
    .await
    .expect("email read");
    assert!(data["old_email_subject"].as_str().is_some());
    assert!(data["new_email_subject"].as_str().is_some());

    let messages = context.emails();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].0, "alice@example.com");
    assert_eq!(messages[1].0, "alice-new@example.com");

    let entry = context.state.caches.email_update.read(&user_id).expect("entry");
    let old_hash = nail_common::hash::email("alice@example.com");
    let new_hash = nail_common::hash::email("alice-new@example.com");
    assert_eq!(entry.old_email_address_hash, old_hash);
    assert_eq!(entry.new_email_address_hash, new_hash);
}

#[tokio::test]
async fn change_email_rejects_a_mismatched_old_email() {
    let context = TestCtx::new().await.expect("test context");
    let (_, session_token) = session_for(&context, "alice@example.com").await;
    let request = EmailReadRequest {
        pow: None,
        old_email_pow: Some(context.issued_pow("someone@example.com")),
        new_email_pow: Some(context.issued_pow("alice-new@example.com")),
    };
    let error = crate::logic::email::read_email(
        &context.state,
        EmailReadIntent::ChangeEmail,
        request,
        Some(session_token),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("old email does not match your current email")
    );
}

#[tokio::test]
async fn change_email_rejects_same_old_and_new_email() {
    let context = TestCtx::new().await.expect("test context");
    let (_, session_token) = session_for(&context, "alice@example.com").await;
    let request = EmailReadRequest {
        pow: None,
        old_email_pow: Some(context.issued_pow("alice@example.com")),
        new_email_pow: Some(context.issued_pow("alice@example.com")),
    };
    let error = crate::logic::email::read_email(
        &context.state,
        EmailReadIntent::ChangeEmail,
        request,
        Some(session_token),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("new email must be different from old email")
    );
}

#[tokio::test]
async fn update_user_email_updates_email_and_returns_a_new_session() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, old_session) = session_for(&context, "alice@example.com").await;
    let old_token = uuid::Uuid::now_v7().to_string();
    let new_token = uuid::Uuid::now_v7().to_string();
    context.state.caches.email_update.insert(
        &user_id,
        crate::repository::cache::EmailUpdateTokenEntry {
            old_email_address_hash: nail_common::hash::email("alice@example.com"),
            new_email_address_hash: nail_common::hash::email("alice-new@example.com"),
            token_from_old_email_hash: token_key(&old_token).expect("old hash"),
            token_from_new_email_hash: token_key(&new_token).expect("new hash"),
        },
    );

    let payload = format!("{old_token}\n{new_token}");
    let pow = context.issued_pow(&payload);
    let new_session = update_user_email(
        &context.state,
        &user_id,
        &pow,
        &old_token,
        &new_token,
    )
    .await
    .expect("confirm");

    let entry = crate::repository::user::read_user(&context.state.graph, &user_id)
        .await
        .expect("read")
        .expect("entry");
    assert_eq!(entry.email_address_hash, nail_common::hash::email("alice-new@example.com"));

    let old_key = token_key(&old_session).expect("old session key");
    assert!(context.state.caches.session.read(&old_key).is_none());

    let new_key = token_key(&new_session).expect("new session key");
    assert_eq!(
        context.state.caches.session.read(&new_key).expect("entry").user_id,
        user_id
    );
}

#[tokio::test]
async fn update_user_email_rejects_token_mismatch() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let old_token = uuid::Uuid::now_v7().to_string();
    let new_token = uuid::Uuid::now_v7().to_string();
    context.state.caches.email_update.insert(
        &user_id,
        crate::repository::cache::EmailUpdateTokenEntry {
            old_email_address_hash: nail_common::hash::email("alice@example.com"),
            new_email_address_hash: nail_common::hash::email("alice-new@example.com"),
            token_from_old_email_hash: token_key(&old_token).expect("old hash"),
            token_from_new_email_hash: token_key(&new_token).expect("new hash"),
        },
    );

    let wrong_old = uuid::Uuid::now_v7().to_string();
    let payload = format!("{wrong_old}\n{new_token}");
    let pow = context.issued_pow(&payload);
    let error = update_user_email(&context.state, &user_id, &pow, &wrong_old, &new_token)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::bad_request("token mismatch"));
}

#[tokio::test]
async fn delete_intent_requires_a_session() {
    let context = TestCtx::new().await.expect("test context");
    let error = crate::logic::email::read_email(
        &context.state,
        EmailReadIntent::Deregister,
        EmailReadRequest::default(),
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::unauthorized("missing session-token header"));
}

#[tokio::test]
async fn delete_intent_sends_a_confirmation_and_caches_the_token() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, session_token) = session_for(&context, "alice@example.com").await;
    let pow = context.issued_pow("alice@example.com");
    let request = EmailReadRequest {
        pow: Some(pow),
        old_email_pow: None,
        new_email_pow: None,
    };
    let data = crate::logic::email::read_email(
        &context.state,
        EmailReadIntent::Deregister,
        request,
        Some(session_token),
    )
    .await
    .expect("email read");
    let subject = data["email_subject"].as_str().expect("subject");

    let messages = context.emails();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].0, "alice@example.com");
    assert_eq!(messages[0].1, subject);

    let token_key = token_key(&messages[0].2).expect("token key");
    let entry = context.state.caches.delete_user.read(&token_key).expect("entry");
    assert_eq!(entry.user_id, user_id);
}

#[tokio::test]
async fn delete_intent_rejects_a_mismatched_email() {
    let context = TestCtx::new().await.expect("test context");
    let (_, session_token) = session_for(&context, "alice@example.com").await;
    let pow = context.issued_pow("someone-else@example.com");
    let request = EmailReadRequest {
        pow: Some(pow),
        old_email_pow: None,
        new_email_pow: None,
    };
    let error = crate::logic::email::read_email(
        &context.state,
        EmailReadIntent::Deregister,
        request,
        Some(session_token),
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("email does not match your account"));
}

#[tokio::test]
async fn send_update_user_email_rejects_a_taken_new_email() {
    let context = TestCtx::new().await.expect("test context");
    let (_, session_token) = session_for(&context, "alice@example.com").await;
    crate::repository::user::find_or_create_user(
        &context.state.graph,
        &nail_common::hash::email("bob@example.com"),
    )
    .await
    .expect("bob");
    let request = EmailReadRequest {
        pow: None,
        old_email_pow: Some(context.issued_pow("alice@example.com")),
        new_email_pow: Some(context.issued_pow("bob@example.com")),
    };
    let error = crate::logic::email::read_email(
        &context.state,
        EmailReadIntent::ChangeEmail,
        request,
        Some(session_token),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("new email is already used by another account")
    );
}

#[tokio::test]
async fn send_update_user_email_direct_returns_both_subjects() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let (old_subject, new_subject) = send_update_user_email(
        &context.state,
        &user_id,
        &context.issued_pow("alice@example.com"),
        &context.issued_pow("alice-new@example.com"),
    )
    .await
    .expect("send");
    assert!(!old_subject.is_empty());
    assert!(!new_subject.is_empty());
}

#[tokio::test]
async fn send_delete_user_email_direct_sends_confirmation() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let subject = send_delete_user_email(&context.state, &user_id, &context.issued_pow("alice@example.com"))
        .await
        .expect("request");
    assert!(!subject.is_empty());
    assert_eq!(context.emails().len(), 1);
}
