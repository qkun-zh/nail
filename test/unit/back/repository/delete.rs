use super::context::{build_state, test_config};

use crate::repository::delete::hard_delete_user;

#[tokio::test]
async fn hard_delete_user_removes_the_user_node() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = crate::repository::user::find_or_create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");

    hard_delete_user(&state.graph, &user_id).await.expect("delete");

    let entry = crate::repository::user::read_user(&state.graph, &user_id)
        .await
        .expect("read");
    assert_eq!(entry, None);
}

#[tokio::test]
async fn hard_delete_user_is_idempotent_for_a_missing_user() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    hard_delete_user(&state.graph, "missing").await.expect("delete");
}
