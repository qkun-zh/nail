use super::context::{build_state, test_config};

use agdb::QueryBuilder;
use crate::repository::article::{create_article, read_article_versions};
use crate::repository::delete::{hard_delete_article, hard_delete_user, hard_delete_version};
use crate::repository::schema::{
    CommentRow, EDGE_COMMENT_TO_VERSION, EDGE_USER_TO_COMMENT, ENTITY_TYPE_COMMENT,
    ENTITY_TYPE_USER, KEY_TYPE, alias_of,
};

fn pdf_hash(seed: u8) -> String {
    (0..32).map(|_| format!("{seed:x}")).collect()
}

async fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::find_or_create_user(&state.graph, &nail_common::hash::email(email))
        .await
        .expect("user")
}

async fn create_article_fixture(
    state: &crate::infrastructure::state::AppState,
    author_id: &str,
    hash: &str,
) -> (String, String) {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.graph,
        &article_id,
        author_id,
        "Article",
        "summary",
        &["#rust".to_string()],
        &version_id,
        "1.0.0",
        hash,
        "note",
    )
    .await
    .expect("create article");
    (article_id, version_id)
}

async fn insert_comment(
    state: &crate::infrastructure::state::AppState,
    version_id: &str,
    author_id: &str,
) -> String {
    let comment_id = uuid::Uuid::now_v7().to_string();
    let mut guard = state.graph.write().await;
    guard
        .exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([alias_of(ENTITY_TYPE_COMMENT, &comment_id)])
                .values(CommentRow {
                    db_id: None,
                    entity_type: ENTITY_TYPE_COMMENT.to_string(),
                    id: comment_id.clone(),
                    content: "a comment".to_string(),
                })
                .query(),
        )
        .expect("comment node");
    guard
        .exec_mut(
            QueryBuilder::insert()
                .edges()
                .from(alias_of(ENTITY_TYPE_USER, author_id))
                .to([alias_of(ENTITY_TYPE_COMMENT, &comment_id)])
                .values([[(KEY_TYPE, EDGE_USER_TO_COMMENT).into()]])
                .query(),
        )
        .expect("user comment edge");
    guard
        .exec_mut(
            QueryBuilder::insert()
                .edges()
                .from(alias_of(ENTITY_TYPE_COMMENT, &comment_id))
                .to([alias_of("version", version_id)])
                .values([[(KEY_TYPE, EDGE_COMMENT_TO_VERSION).into()]])
                .query(),
        )
        .expect("version comment edge");
    comment_id
}

#[tokio::test]
async fn hard_delete_user_removes_the_user_node() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = create_user(&state, "alice@example.com").await;

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

#[tokio::test]
async fn hard_delete_article_cascades_versions_and_comments_and_collects_hashes() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    insert_comment(&state, &version_id, &author_id).await;

    let outcome = hard_delete_article(&state.graph, &article_id)
        .await
        .expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(1)]);

    assert!(crate::repository::article::read_article(&state.graph, &article_id)
        .await
        .expect("read")
        .is_none());
    assert!(crate::repository::version::read_version(&state.graph, &version_id)
        .await
        .expect("read")
        .is_none());
}

#[tokio::test]
async fn hard_delete_version_removes_only_the_version_and_refreshes_latest() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, first_version) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    let second_version = uuid::Uuid::now_v7().to_string();
    crate::repository::version::create_version(
        &state.graph,
        &article_id,
        &second_version,
        "2.0.0",
        &pdf_hash(2),
        "note",
    )
    .await
    .expect("v2");

    let outcome = hard_delete_version(&state.graph, &second_version)
        .await
        .expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(2)]);

    let (remaining, total) = read_article_versions(&state.graph, &article_id, 10, 0)
        .await
        .expect("versions");
    assert_eq!(total, 1);
    assert_eq!(remaining[0].id, first_version);
}

#[tokio::test]
async fn hard_delete_version_is_idempotent_for_a_missing_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let outcome = hard_delete_version(&state.graph, "missing").await.expect("delete");
    assert!(outcome.removed_pdf_hashes.is_empty());
}
