use super::context::{build_state, test_config};

use crate::repository::user::{
    UserWriteError, find_or_create_user, find_user_by_email_address_hash, list_users, read_user,
    update_user_email, update_user_name,
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

#[tokio::test]
async fn list_users_returns_users_sorted_by_id_desc() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    find_or_create_user(&state.graph, &nail_common::hash::email("a@example.com"))
        .await
        .expect("first");
    find_or_create_user(&state.graph, &nail_common::hash::email("b@example.com"))
        .await
        .expect("second");

    let (page, total) = list_users(&state.graph, 10, 0).await.expect("list");
    assert_eq!(total, 3);
    assert_eq!(page.len(), 3);
    let ids: Vec<&str> = page.iter().map(|row| row.id.as_str()).collect();
    let mut sorted = ids.clone();
    sorted.sort_by(|left, right| right.cmp(left));
    assert_eq!(ids, sorted);
}

#[tokio::test]
async fn list_users_paginates_by_limit_and_offset() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let (page, total) = list_users(&state.graph, 1, 0).await.expect("list");
    assert_eq!(total, 1);
    assert_eq!(page.len(), 1);
}

#[tokio::test]
async fn update_user_name_applies_the_new_name() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("alice@example.com");
    let user_id = find_or_create_user(&state.graph, &hash).await.expect("user");
    update_user_name(&state.graph, &user_id, "alice").await.expect("update");
    let entry = read_user(&state.graph, &user_id).await.expect("read").expect("entry");
    assert_eq!(entry.name, "alice");
}

#[tokio::test]
async fn update_user_name_rejects_a_taken_name() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let first = find_or_create_user(&state.graph, &nail_common::hash::email("a@example.com"))
        .await
        .expect("first");
    let second = find_or_create_user(&state.graph, &nail_common::hash::email("b@example.com"))
        .await
        .expect("second");
    update_user_name(&state.graph, &first, "alice").await.expect("first update");
    assert!(matches!(
        update_user_name(&state.graph, &second, "alice").await,
        Err(UserWriteError::AlreadyTaken)
    ));
}

#[tokio::test]
async fn update_user_name_reports_a_missing_user() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    assert!(matches!(
        update_user_name(&state.graph, "missing", "alice").await,
        Err(UserWriteError::UserMissing)
    ));
}

#[tokio::test]
async fn update_user_email_applies_the_new_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let old_hash = nail_common::hash::email("alice@example.com");
    let new_hash = nail_common::hash::email("alice-new@example.com");
    let user_id = find_or_create_user(&state.graph, &old_hash).await.expect("user");
    update_user_email(&state.graph, &user_id, &old_hash, &new_hash)
        .await
        .expect("update");
    let entry = read_user(&state.graph, &user_id).await.expect("read").expect("entry");
    assert_eq!(entry.email_address_hash, new_hash);
}

#[tokio::test]
async fn update_user_email_rejects_a_taken_new_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let first = find_or_create_user(&state.graph, &nail_common::hash::email("a@example.com"))
        .await
        .expect("first");
    find_or_create_user(&state.graph, &nail_common::hash::email("b@example.com"))
        .await
        .expect("second");
    assert!(matches!(
        update_user_email(
            &state.graph,
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
    let user_id = find_or_create_user(&state.graph, &nail_common::hash::email("alice@example.com"))
        .await
        .expect("user");
    assert!(matches!(
        update_user_email(
            &state.graph,
            &user_id,
            &nail_common::hash::email("someone-else@example.com"),
            &nail_common::hash::email("alice-new@example.com")
        )
        .await,
        Err(UserWriteError::EmailMismatch)
    ));
}
