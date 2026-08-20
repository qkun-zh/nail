use nail_common::request::{CreateTokenRequest, TokenPurpose};

use super::context::TestCtx;
use crate::logic::email::{
    CreateTokenView, normalize_email, send_delete_user_email, send_update_user_email,
    update_user_email, validate_email,
};
use crate::logic::error::LogicError;
use crate::repository::cache::{SessionTokenEntry, token_key};

#[test]
fn normalize_email_trims_and_lowercases() {
    assert_eq!(
        normalize_email("  Alice@Example.COM  "),
        "alice@example.com"
    );
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
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.cache.session.insert(
        &key,
        SessionTokenEntry {
            user_id: user_id.clone(),
        },
    );
    (user_id, token)
}

#[tokio::test]
async fn create_user_token_sends_and_caches_a_token() {
    let context = TestCtx::new().await.expect("test context");
    let request = CreateTokenRequest {
        purpose: TokenPurpose::CreateUser,
        email: Some("alice@example.com".to_string()),
        old_email: None,
        new_email: None,
    };
    let data = crate::logic::email::create_token(&context.state, request, None)
        .await
        .expect("create token");
    let subject = match data {
        CreateTokenView::Subject(view) => view.email_subject,
        CreateTokenView::Subjects(_) => panic!("unexpected subjects"),
    };
    assert!(!subject.is_empty());

    let messages = context.emails();
    assert_eq!(messages.len(), 1);
    let (to, message_subject, body) = &messages[0];
    assert_eq!(to, "alice@example.com");
    assert_eq!(message_subject, &subject);
    let token_key = token_key(body).expect("token key");
    assert!(context.state.cache.create_user.read(&token_key).is_some());
}

#[tokio::test]
async fn create_user_token_requires_an_email() {
    let context = TestCtx::new().await.expect("test context");
    let request = CreateTokenRequest {
        purpose: TokenPurpose::CreateUser,
        email: None,
        old_email: None,
        new_email: None,
    };
    let error = crate::logic::email::create_token(&context.state, request, None)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::bad_request("email is required"));
}

#[tokio::test]
async fn create_user_token_rejects_a_disallowed_domain() {
    let context = TestCtx::new().await.expect("test context");
    let request = CreateTokenRequest {
        purpose: TokenPurpose::CreateUser,
        email: Some("alice@other.org".to_string()),
        old_email: None,
        new_email: None,
    };
    let error = crate::logic::email::create_token(&context.state, request, None)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::bad_request("email domain not allowed"));
    assert!(context.emails().is_empty());
}

#[tokio::test]
async fn change_email_requires_a_session() {
    let context = TestCtx::new().await.expect("test context");
    let request = CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        email: None,
        old_email: Some("alice@example.com".to_string()),
        new_email: Some("alice-new@example.com".to_string()),
    };
    let error = crate::logic::email::create_token(&context.state, request, None)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        LogicError::unauthorized("missing session-token header")
    );
}

#[tokio::test]
async fn change_email_sends_two_emails_and_caches_the_token_hashes() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, session_token) = session_for(&context, "alice@example.com").await;
    let request = CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        email: None,
        old_email: Some("alice@example.com".to_string()),
        new_email: Some("alice-new@example.com".to_string()),
    };
    let data = crate::logic::email::create_token(&context.state, request, Some(session_token))
        .await
        .expect("create token");
    let CreateTokenView::Subjects(view) = data else {
        panic!("expected subjects");
    };
    assert!(!view.old_email_subject.is_empty());
    assert!(!view.new_email_subject.is_empty());

    let messages = context.emails();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].0, "alice@example.com");
    assert_eq!(messages[1].0, "alice-new@example.com");

    let entry = context
        .state
        .cache
        .email_update
        .read(&user_id)
        .expect("entry");
    let old_hash =
        nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed");
    let new_hash =
        nail_common::hash::hash("alice-new@example.com".as_bytes()).expect("hash must succeed");
    assert_eq!(entry.old_email_hash, old_hash);
    assert_eq!(entry.new_email_hash, new_hash);
}

#[tokio::test]
async fn change_email_rejects_a_mismatched_old_email() {
    let context = TestCtx::new().await.expect("test context");
    let (_, session_token) = session_for(&context, "alice@example.com").await;
    let request = CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        email: None,
        old_email: Some("someone@example.com".to_string()),
        new_email: Some("alice-new@example.com".to_string()),
    };
    let error = crate::logic::email::create_token(&context.state, request, Some(session_token))
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
    let request = CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        email: None,
        old_email: Some("alice@example.com".to_string()),
        new_email: Some("alice@example.com".to_string()),
    };
    let error = crate::logic::email::create_token(&context.state, request, Some(session_token))
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
    let (user_id, _old_session) = session_for(&context, "alice@example.com").await;
    let old_token = uuid::Uuid::now_v7().to_string();
    let new_token = uuid::Uuid::now_v7().to_string();
    context.state.cache.email_update.insert(
        &user_id,
        crate::repository::cache::EmailUpdateTokenEntry {
            old_email_hash: nail_common::hash::hash("alice@example.com".as_bytes())
                .expect("hash must succeed"),
            new_email_hash: nail_common::hash::hash("alice-new@example.com".as_bytes())
                .expect("hash must succeed"),
            token_hash_from_old_email: token_key(&old_token).expect("old hash"),
            token_hash_from_new_email: token_key(&new_token).expect("new hash"),
        },
    );

    let new_session = update_user_email(&context.state, &user_id, &old_token, &new_token)
        .await
        .expect("update email");
    assert!(!new_session.is_empty());

    let entry = crate::repository::user::read_user(&context.state.database, &user_id)
        .await
        .expect("read")
        .expect("entry");
    assert_eq!(
        entry.email_address_hash,
        nail_common::hash::hash("alice-new@example.com".as_bytes()).expect("hash must succeed")
    );
}

#[tokio::test]
async fn delete_user_token_sends_and_caches_a_confirmation_token() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, session_token) = session_for(&context, "alice@example.com").await;
    let data = send_delete_user_email(&context.state, &user_id, "alice@example.com")
        .await
        .expect("deregister");
    assert!(!data.is_empty());

    let messages = context.emails();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].0, "alice@example.com");
    let _ = session_token;
}

#[tokio::test]
async fn send_update_user_email_rejects_a_taken_new_email() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    session_for(&context, "bob@example.com").await;
    let error = send_update_user_email(
        &context.state,
        &user_id,
        "alice@example.com",
        "bob@example.com",
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("new email is already used by another account")
    );
}

#[tokio::test]
async fn change_email_rejects_new_email_on_disallowed_domain() {
    let context = TestCtx::new().await.expect("test context");
    let (_, session_token) = session_for(&context, "alice@example.com").await;
    let request = CreateTokenRequest {
        purpose: TokenPurpose::UpdateUserEmail,
        email: None,
        old_email: Some("alice@example.com".to_string()),
        new_email: Some("alice@other.org".to_string()),
    };
    let error = crate::logic::email::create_token(&context.state, request, Some(session_token))
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::bad_request("email domain not allowed"));
}

#[tokio::test]
async fn update_user_email_rejects_same_old_and_new_token() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let same_token = uuid::Uuid::now_v7().to_string();
    let other_token = uuid::Uuid::now_v7().to_string();
    context.state.cache.email_update.insert(
        &user_id,
        crate::repository::cache::EmailUpdateTokenEntry {
            old_email_hash: nail_common::hash::hash("alice@example.com".as_bytes())
                .expect("hash must succeed"),
            new_email_hash: nail_common::hash::hash("alice-new@example.com".as_bytes())
                .expect("hash must succeed"),
            token_hash_from_old_email: token_key(&same_token).expect("hash"),
            token_hash_from_new_email: token_key(&other_token).expect("hash"),
        },
    );
    let error = update_user_email(&context.state, &user_id, &same_token, &same_token)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("old token and new token must be different")
    );
}

#[tokio::test]
async fn update_user_email_rejects_token_mismatch() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let old_token = uuid::Uuid::now_v7().to_string();
    let new_token = uuid::Uuid::now_v7().to_string();
    let wrong_token = uuid::Uuid::now_v7().to_string();
    context.state.cache.email_update.insert(
        &user_id,
        crate::repository::cache::EmailUpdateTokenEntry {
            old_email_hash: nail_common::hash::hash("alice@example.com".as_bytes())
                .expect("hash must succeed"),
            new_email_hash: nail_common::hash::hash("alice-new@example.com".as_bytes())
                .expect("hash must succeed"),
            token_hash_from_old_email: token_key(&old_token).expect("hash"),
            token_hash_from_new_email: token_key(&new_token).expect("hash"),
        },
    );
    let error = update_user_email(&context.state, &user_id, &old_token, &wrong_token)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::bad_request("token mismatch"));
}

#[tokio::test]
async fn send_delete_user_email_rejects_mismatched_email() {
    let context = TestCtx::new().await.expect("test context");
    let (user_id, _) = session_for(&context, "alice@example.com").await;
    let error = send_delete_user_email(&context.state, &user_id, "bob@example.com")
        .await
        .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("email does not match your account")
    );
}

#[tokio::test]
async fn delete_user_token_requires_session() {
    let context = TestCtx::new().await.expect("test context");
    let request = CreateTokenRequest {
        purpose: TokenPurpose::DeleteUser,
        email: Some("alice@example.com".to_string()),
        old_email: None,
        new_email: None,
    };
    let error = crate::logic::email::create_token(&context.state, request, None)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        LogicError::unauthorized("missing session-token header")
    );
}
