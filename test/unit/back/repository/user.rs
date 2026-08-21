use super::context::{build_state, test_config};

use crate::repository::user::{
    UserWriteError, create_user, read_user, read_user_by_email_address_hash, update_user_email,
    update_user_name,
};

#[tokio::test]
async fn create_user_is_idempotent_on_email_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed");
    let first = create_user(&state.database, &hash).expect("first");
    let second = create_user(&state.database, &hash).expect("second");
    assert_eq!(first, second);
}

#[tokio::test]
async fn read_user_by_email_address_hash_returns_none_for_unknown() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let result = read_user_by_email_address_hash(&state.database, "missing").expect("lookup");
    assert_eq!(result, None);
}

#[tokio::test]
async fn read_user_returns_the_email_hash_and_default_name() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed");
    let user_id = create_user(&state.database, &hash).expect("user");
    let entry = read_user(&state.database, &user_id)
        .expect("read")
        .expect("entry");
    assert_eq!(entry.email_address_hash, hash);
    assert_eq!(entry.name, user_id.replace('-', ""));
}

#[tokio::test]
async fn read_user_returns_none_for_an_unknown_id() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let entry = read_user(&state.database, "missing").expect("read");
    assert_eq!(entry, None);
}

#[tokio::test]
async fn update_user_name_applies_the_new_name() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed");
    let user_id = create_user(&state.database, &hash).expect("user");
    update_user_name(&state.database, &user_id, "alice").expect("update");
    let entry = read_user(&state.database, &user_id)
        .expect("read")
        .expect("entry");
    assert_eq!(entry.name, "alice");
}

#[tokio::test]
async fn update_user_name_rejects_a_taken_name() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let first = create_user(
        &state.database,
        &nail_common::hash::hash("a@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("first");
    let second = create_user(
        &state.database,
        &nail_common::hash::hash("b@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("second");
    update_user_name(&state.database, &first, "alice").expect("first update");
    assert!(matches!(
        update_user_name(&state.database, &second, "alice"),
        Err(UserWriteError::AlreadyTaken)
    ));
}

#[tokio::test]
async fn update_user_name_reports_a_missing_user() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    assert!(matches!(
        update_user_name(&state.database, "missing", "alice"),
        Err(UserWriteError::UserMissing)
    ));
}

#[tokio::test]
async fn update_user_email_applies_the_new_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let old_hash =
        nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed");
    let new_hash =
        nail_common::hash::hash("alice-new@example.com".as_bytes()).expect("hash must succeed");
    let user_id = create_user(&state.database, &old_hash).expect("user");
    update_user_email(&state.database, &user_id, &old_hash, &new_hash).expect("update");
    let entry = read_user(&state.database, &user_id)
        .expect("read")
        .expect("entry");
    assert_eq!(entry.email_address_hash, new_hash);
}

#[tokio::test]
async fn update_user_email_rejects_a_taken_new_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let first = create_user(
        &state.database,
        &nail_common::hash::hash("a@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("first");
    create_user(
        &state.database,
        &nail_common::hash::hash("b@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("second");
    assert!(matches!(
        update_user_email(
            &state.database,
            &first,
            &nail_common::hash::hash("a@example.com".as_bytes()).expect("hash must succeed"),
            &nail_common::hash::hash("b@example.com".as_bytes()).expect("hash must succeed")
        ),
        Err(UserWriteError::AlreadyTaken)
    ));
}

#[tokio::test]
async fn update_user_email_rejects_a_mismatched_old_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = create_user(
        &state.database,
        &nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("user");
    assert!(matches!(
        update_user_email(
            &state.database,
            &user_id,
            &nail_common::hash::hash("someone-else@example.com".as_bytes())
                .expect("hash must succeed"),
            &nail_common::hash::hash("alice-new@example.com".as_bytes())
                .expect("hash must succeed")
        ),
        Err(UserWriteError::EmailMismatch)
    ));
}
