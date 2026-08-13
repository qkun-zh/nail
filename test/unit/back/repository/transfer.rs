use super::context::{build_state, test_config};

use crate::repository::article::{ArticleDraft, create_article, owner_of};
use crate::repository::transfer::{
    TransferTargetError, transfer_account_assets, transfer_article,
};
use crate::repository::version::VersionDraft;

fn pdf_hash(seed: u8) -> String {
    (0..32).map(|_| format!("{seed:x}")).collect()
}

async fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::create_user(&state.graph, &nail_common::hash::email(email))
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
    assert!(outcome.transferred_article_ids.is_empty());
    assert!(outcome.transferred_comment_ids.is_empty());

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
    assert!(outcome.transferred_article_ids.is_empty());
    assert!(outcome.transferred_comment_ids.is_empty());
}

#[tokio::test]
async fn transfer_article_repoints_the_owner_edge_to_the_recycler() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let recycler_id = create_user(&state, "user-zero@example.com").await;
    let article_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.clone(),
            title: "Article".to_string(),
            summary: "summary".to_string(),
            tags: vec!["#rust".to_string()],
            first_version: VersionDraft {
                version_id: uuid::Uuid::now_v7().to_string(),
                version_number: "1.0.0".to_string(),
                content_hash: pdf_hash(1),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create");

    assert_eq!(owner_of(&state.graph, &article_id).await.expect("owner"), Some(author_id.clone()));

    transfer_article(&state.graph, &article_id).await.expect("transfer");

    assert_eq!(owner_of(&state.graph, &article_id).await.expect("owner"), Some(recycler_id));
}

#[tokio::test]
async fn transfer_article_reports_a_missing_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let error = transfer_article(&state.graph, "missing")
        .await
        .expect_err("missing");
    assert!(matches!(error, TransferTargetError::TargetMissing));
}
