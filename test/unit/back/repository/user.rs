use super::context::{build_state, test_config};

use crate::repository::user::{
    find_or_create_user, find_user_by_email_address_hash, read_user,
};

#[tokio::test]
async fn find_or_create_user_is_idempotent_on_email_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("alice@example.com");
    let first = find_or_create_user(&state.graph, &hash).await.expect("first");
    let second = find_or_create_user(&state.graph, &hash).await.expect("second");
    assert_eq!(first, second);
}

#[tokio::test]
async fn find_user_by_email_hash_returns_none_for_unknown() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let result = find_user_by_email_address_hash(&state.graph, "missing")
        .await
        .expect("lookup");
    assert_eq!(result, None);
}

#[tokio::test]
async fn read_user_returns_the_email_hash_and_default_name() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("alice@example.com");
    let user_id = find_or_create_user(&state.graph, &hash).await.expect("user");
    let entry = read_user(&state.graph, &user_id).await.expect("read").expect("entry");
    assert_eq!(entry.email_address_hash, hash);
    assert_eq!(entry.name, user_id.replace('-', ""));
}

#[tokio::test]
async fn read_user_returns_none_for_an_unknown_id() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let entry = read_user(&state.graph, "missing").await.expect("read");
    assert_eq!(entry, None);
}
