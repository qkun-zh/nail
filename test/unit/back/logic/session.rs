use super::context::TestCtx;
use crate::logic::error::LogicError;
use crate::logic::session::{create_session, normalize_token, read_session};
use crate::repository::cache::{SessionTokenEntry, token_key};

#[test]
fn normalize_token_strips_whitespace_and_requires_a_uuid() {
    let uuid = uuid::Uuid::now_v7().to_string();
    assert_eq!(
        normalize_token(&format!(" {uuid}\n")).as_deref(),
        Some(uuid.as_str())
    );
    assert_eq!(normalize_token("not-a-uuid"), None);
    assert_eq!(normalize_token(""), None);
}

#[test]
fn hash_canonical_token_matches_the_repository_token_key() {
    let token = uuid::Uuid::now_v7().to_string();
    let via_logic = crate::logic::session::hash_canonical_token(&token).expect("hash");
    assert_eq!(via_logic, token_key(&token).expect("token key"));
}

#[test]
fn hash_token_normalizes_then_hashes() {
    let token = uuid::Uuid::now_v7().to_string();
    let key = crate::logic::session::hash_token(
        &format!(" {token}\n"),
        LogicError::bad_request("invalid"),
    )
    .expect("hash");
    assert_eq!(key, token_key(&token).expect("token key"));
}

#[test]
fn hash_token_rejects_a_non_uuid_payload_with_the_given_error() {
    let error = crate::logic::session::hash_token(
        "not-a-uuid",
        LogicError::bad_request("invalid or expired token"),
    )
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("invalid or expired token"));
}

#[test]
fn hash_token_rejects_an_empty_payload_with_the_given_error() {
    let error =
        crate::logic::session::hash_token("", LogicError::bad_request("invalid delete token"))
            .unwrap_err();
    assert_eq!(error, LogicError::bad_request("invalid delete token"));
}

#[tokio::test]
async fn read_session_returns_the_user_id_for_a_known_token() {
    let context = TestCtx::new().await.expect("test context");
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.session.insert(
        &key,
        SessionTokenEntry {
            user_id: "user-123".to_string(),
        },
    );
    assert_eq!(
        read_session(&context.state, &token).expect("session"),
        "user-123"
    );
}

#[tokio::test]
async fn read_session_rejects_garbage_and_unknown_tokens() {
    let context = TestCtx::new().await.expect("test context");
    assert_eq!(
        read_session(&context.state, "not-a-uuid").unwrap_err(),
        LogicError::unauthorized("invalid session")
    );
    let unknown = uuid::Uuid::now_v7().to_string();
    assert_eq!(
        read_session(&context.state, &unknown).unwrap_err(),
        LogicError::unauthorized("invalid session")
    );
}

#[tokio::test]
async fn create_session_stores_a_token_for_the_user() {
    let context = TestCtx::new().await.expect("test context");
    let session_token = create_session(&context.state, "user-123").expect("create");
    let key = token_key(&session_token).expect("token key");
    let entry = context.state.caches.session.read(&key).expect("entry");
    assert_eq!(entry.user_id, "user-123");
}

#[tokio::test]
async fn delete_session_removes_the_session_token() {
    let context = TestCtx::new().await.expect("test context");
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.session.insert(
        &key,
        SessionTokenEntry {
            user_id: "user-123".to_string(),
        },
    );

    let pow = context.issued_pow("delete-session-nonce");
    crate::logic::session::delete_session(&context.state, &pow, &token).expect("delete");
    assert!(context.state.caches.session.read(&key).is_none());
}

#[tokio::test]
async fn delete_session_requires_a_valid_session() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.issued_pow("delete-session-nonce");
    let error =
        crate::logic::session::delete_session(&context.state, &pow, "not-a-uuid").unwrap_err();
    assert_eq!(error, LogicError::unauthorized("invalid session"));
}

#[tokio::test]
async fn read_user_name_returns_the_account_name() {
    let context = TestCtx::new().await.expect("test context");
    let email_hash = nail_common::hash::email("alice@example.com");
    let user_id = crate::repository::user::create_user(&context.state.graph, &email_hash)
        .await
        .expect("create user");
    let token = uuid::Uuid::now_v7().to_string();
    let key = token_key(&token).expect("token key");
    context.state.caches.session.insert(
        &key,
        SessionTokenEntry {
            user_id: user_id.clone(),
        },
    );

    let name = crate::logic::session::read_user_name(&context.state, &token)
        .await
        .expect("name");
    assert_eq!(name, user_id.replace('-', ""));
}
