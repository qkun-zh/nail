use cache::UserId;

use super::context::TestCtx;
use crate::logic::error::LogicError;
use crate::logic::session::{cache_key, create_session, normalize_token, read_session};

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
fn hash_canonical_token_matches_cache_key() {
    let token = uuid::Uuid::now_v7().to_string();
    let via_logic = crate::logic::session::hash_canonical_token(&token).expect("hash");
    assert_eq!(via_logic, cache_key(&token).expect("cache key"));
}

#[test]
fn hash_token_normalizes_then_hashes() {
    let token = uuid::Uuid::now_v7().to_string();
    let key = crate::logic::session::hash_token(
        &format!(" {token}\n"),
        LogicError::bad_request("invalid"),
    )
    .expect("hash");
    assert_eq!(key, cache_key(&token).expect("cache key"));
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
    let user_id = uuid::Uuid::now_v7().to_string();
    let token = uuid::Uuid::now_v7().to_string();
    let key = cache_key(&token).expect("cache key");
    context
        .state
        .cache
        .session
        .insert(&key, UserId::new(user_id.clone()).expect("user id"));
    assert_eq!(
        read_session(&context.state, &token).expect("session"),
        user_id
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
    let user_id = uuid::Uuid::now_v7().to_string();
    let session_token = create_session(&context.state, &user_id).expect("create");
    let key = cache_key(&session_token).expect("cache key");
    let entry = context.state.cache.session.read(&key).expect("entry");
    assert_eq!(entry.as_str(), user_id);
}

#[tokio::test]
async fn delete_session_removes_the_session_token() {
    let context = TestCtx::new().await.expect("test context");
    let token = uuid::Uuid::now_v7().to_string();
    let key = cache_key(&token).expect("cache key");
    context.state.cache.session.insert(
        &key,
        UserId::new(uuid::Uuid::now_v7().to_string()).expect("user id"),
    );

    crate::logic::session::delete_session(&context.state, &token).expect("delete");
    assert!(context.state.cache.session.read(&key).is_none());
}

#[tokio::test]
async fn delete_session_requires_a_valid_session() {
    let context = TestCtx::new().await.expect("test context");
    let error = crate::logic::session::delete_session(&context.state, "not-a-uuid").unwrap_err();
    assert_eq!(error, LogicError::unauthorized("invalid session"));
}

#[tokio::test]
async fn read_user_name_returns_the_account_name() {
    let context = TestCtx::new().await.expect("test context");
    let email_hash =
        nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed");
    let user_id = crate::repository::user::create_user(&context.state.database, &email_hash)
        .await
        .expect("create user");
    let token = uuid::Uuid::now_v7().to_string();
    let key = cache_key(&token).expect("cache key");
    context
        .state
        .cache
        .session
        .insert(&key, UserId::new(user_id.clone()).expect("user id"));

    let name = crate::logic::session::read_user_name(&context.state, &token)
        .await
        .expect("name");
    assert_eq!(name, user_id.replace('-', ""));
}
