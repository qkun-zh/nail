use super::context::{build_state, test_config};

use crate::repository::article::{article_owner_id, create_article};
use crate::repository::transfer::{TargetTransferError, transfer_account_assets, transfer_article_ownership};

fn pdf_hash(seed: u8) -> String {
    (0..32).map(|_| format!("{seed:x}")).collect()
}

async fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::find_or_create_user(&state.graph, &nail_common::hash::email(email))
        .await
        .expect("user")
}

#[tokio::test]
async fn transfer_account_assets_removes_the_user_node() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = create_user(&state, "alice@example.com").await;

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

#[tokio::test]
async fn transfer_article_ownership_repoints_the_owner_edge_to_the_recycler() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let recycler_id = create_user(&state, "user-zero@example.com").await;
    let article_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.graph,
        &article_id,
        &author_id,
        "Article",
        "summary",
        &["#rust".to_string()],
        &uuid::Uuid::now_v7().to_string(),
        "1.0.0",
        &pdf_hash(1),
        "note",
    )
    .await
    .expect("create");

    assert_eq!(article_owner_id(&state.graph, &article_id).await.expect("owner"), Some(author_id.clone()));

    transfer_article_ownership(&state.graph, &article_id)
        .await
        .expect("transfer");

    assert_eq!(article_owner_id(&state.graph, &article_id).await.expect("owner"), Some(recycler_id));
}

#[tokio::test]
async fn transfer_article_ownership_reports_a_missing_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let error = transfer_article_ownership(&state.graph, "missing")
        .await
        .expect_err("missing");
    assert!(matches!(error, TargetTransferError::TargetNotFound));
}
