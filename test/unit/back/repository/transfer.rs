use super::context::{build_state, test_config};

use crate::repository::transfer::transfer_account_assets;

#[tokio::test]
async fn transfer_account_assets_removes_the_user_node() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = crate::repository::user::find_or_create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");

    let outcome = transfer_account_assets(&state.graph, &user_id)
        .await
        .expect("transfer");
    assert_eq!(outcome.transferred_article_edges, 0);
    assert_eq!(outcome.transferred_comment_edges, 0);

    let entry = crate::repository::user::read_user(&state.graph, &user_id)
        .await
        .expect("read");
    assert_eq!(entry, None);
}

#[tokio::test]
async fn transfer_account_assets_is_idempotent_for_a_missing_user() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let outcome = transfer_account_assets(&state.graph, "missing")
        .await
        .expect("transfer");
    assert_eq!(outcome.transferred_article_edges, 0);
    assert_eq!(outcome.transferred_comment_edges, 0);
}
