use super::context::{build_state, test_config};

use crate::repository::user::{
    UserWriteError, create_user, read_user, read_user_by_email_address_hash, update_user_email,
    update_user_name,
};

#[tokio::test]
async fn create_user_is_idempotent_on_email_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("alice@example.com");
    let first = create_user(&state.database, &hash).await.expect("first");
    let second = create_user(&state.database, &hash).await.expect("second");
    assert_eq!(first, second);
}

#[tokio::test]
async fn read_user_by_email_address_hash_returns_none_for_unknown() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let result = read_user_by_email_address_hash(&state.database, "missing")
        .await
        .expect("lookup");
    assert_eq!(result, None);
}

#[tokio::test]
async fn read_user_returns_the_email_hash_and_default_name() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("alice@example.com");
    let user_id = create_user(&state.database, &hash).await.expect("user");
    let entry = read_user(&state.database, &user_id)
        .await
        .expect("read")
        .expect("entry");
    assert_eq!(entry.email_address_hash, hash);
    assert_eq!(entry.name, user_id.replace('-', ""));
}

#[tokio::test]
async fn read_user_returns_none_for_an_unknown_id() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let entry = read_user(&state.database, "missing").await.expect("read");
    assert_eq!(entry, None);
}

#[tokio::test]
async fn update_user_name_applies_the_new_name() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("alice@example.com");
    let user_id = create_user(&state.database, &hash).await.expect("user");
    update_user_name(&state.database, &user_id, "alice")
        .await
        .expect("update");
    let entry = read_user(&state.database, &user_id)
        .await
        .expect("read")
        .expect("entry");
    assert_eq!(entry.name, "alice");
}

#[tokio::test]
async fn update_user_name_rejects_a_taken_name() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let first = create_user(&state.database, &nail_common::hash::email("a@example.com"))
        .await
        .expect("first");
    let second = create_user(&state.database, &nail_common::hash::email("b@example.com"))
        .await
        .expect("second");
    update_user_name(&state.database, &first, "alice")
        .await
        .expect("first update");
    assert!(matches!(
        update_user_name(&state.database, &second, "alice").await,
        Err(UserWriteError::AlreadyTaken)
    ));
}

#[tokio::test]
async fn update_user_name_reports_a_missing_user() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    assert!(matches!(
        update_user_name(&state.database, "missing", "alice").await,
        Err(UserWriteError::UserMissing)
    ));
}

#[tokio::test]
async fn update_user_email_applies_the_new_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let old_hash = nail_common::hash::email("alice@example.com");
    let new_hash = nail_common::hash::email("alice-new@example.com");
    let user_id = create_user(&state.database, &old_hash).await.expect("user");
    update_user_email(&state.database, &user_id, &old_hash, &new_hash)
        .await
        .expect("update");
    let entry = read_user(&state.database, &user_id)
        .await
        .expect("read")
        .expect("entry");
    assert_eq!(entry.email_address_hash, new_hash);
}

#[tokio::test]
async fn update_user_email_rejects_a_taken_new_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let first = create_user(&state.database, &nail_common::hash::email("a@example.com"))
        .await
        .expect("first");
    create_user(&state.database, &nail_common::hash::email("b@example.com"))
        .await
        .expect("second");
    assert!(matches!(
        update_user_email(
            &state.database,
            &first,
            &nail_common::hash::email("a@example.com"),
            &nail_common::hash::email("b@example.com")
        )
        .await,
        Err(UserWriteError::AlreadyTaken)
    ));
}

#[tokio::test]
async fn update_user_email_rejects_a_mismatched_old_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = create_user(&state.database, &nail_common::hash::email("alice@example.com"))
        .await
        .expect("user");
    assert!(matches!(
        update_user_email(
            &state.database,
            &user_id,
            &nail_common::hash::email("someone-else@example.com"),
            &nail_common::hash::email("alice-new@example.com")
        )
        .await,
        Err(UserWriteError::EmailMismatch)
    ));
}
