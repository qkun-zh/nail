use super::context::{build_state, test_config};

use agdb::QueryBuilder;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::delete::{delete_article, delete_user, delete_version};
use crate::repository::schema::{
    CommentRow, EDGE_COMMENT_TO_COMMENT, EDGE_COMMENT_TO_VERSION, EDGE_USER_TO_COMMENT,
    ENTITY_TYPE_COMMENT, ENTITY_TYPE_USER, KEY_TYPE, alias_of,
};
use crate::repository::version::{VersionDraft, versions_of};

fn pdf_hash(seed: u8) -> String {
    (0..32).map(|_| format!("{seed:x}")).collect()
}

async fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::create_user(&state.graph, &nail_common::hash::email(email))
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
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: "Article".to_string(),
            summary: "summary".to_string(),
            tags: vec!["#rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: hash.to_string(),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    (article_id, version_id)
}

async fn insert_comment_node(
    state: &crate::infrastructure::state::AppState,
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
    comment_id
}

async fn insert_comment(
    state: &crate::infrastructure::state::AppState,
    version_id: &str,
    author_id: &str,
) -> String {
    let comment_id = insert_comment_node(state, author_id).await;
    let mut guard = state.graph.write().await;
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

async fn insert_reply(
    state: &crate::infrastructure::state::AppState,
    parent_comment_id: &str,
    author_id: &str,
) -> String {
    let comment_id = insert_comment_node(state, author_id).await;
    let mut guard = state.graph.write().await;
    guard
        .exec_mut(
            QueryBuilder::insert()
                .edges()
                .from(alias_of(ENTITY_TYPE_COMMENT, &comment_id))
                .to([alias_of(ENTITY_TYPE_COMMENT, parent_comment_id)])
                .values([[(KEY_TYPE, EDGE_COMMENT_TO_COMMENT).into()]])
                .query(),
        )
        .expect("parent comment edge");
    comment_id
}

async fn insert_comment_tree(
    state: &crate::infrastructure::state::AppState,
    version_id: &str,
    author_id: &str,
) {
    let top = insert_comment(state, version_id, author_id).await;
    let reply = insert_reply(state, &top, author_id).await;
    insert_reply(state, &reply, author_id).await;
}

fn count_by_type(guard: &agdb::DbAny, type_value: &str) -> usize {
    guard
        .exec(
            QueryBuilder::search()
                .elements()
                .where_()
                .key(KEY_TYPE)
                .value(type_value)
                .query(),
        )
        .expect("count by type")
        .elements
        .len()
}

async fn assert_no_comment_subtree_remains(state: &crate::infrastructure::state::AppState) {
    let guard = state.graph.read().await;
    assert_eq!(count_by_type(&guard, ENTITY_TYPE_COMMENT), 0);
    assert_eq!(count_by_type(&guard, EDGE_COMMENT_TO_COMMENT), 0);
    assert_eq!(count_by_type(&guard, EDGE_COMMENT_TO_VERSION), 0);
    assert_eq!(count_by_type(&guard, EDGE_USER_TO_COMMENT), 0);
}

#[tokio::test]
async fn delete_user_removes_the_user_node() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = create_user(&state, "alice@example.com").await;

    delete_user(&state.graph, &user_id).await.expect("delete");

    let entry = crate::repository::user::read_user(&state.graph, &user_id)
        .await
        .expect("read");
    assert_eq!(entry, None);
}

#[tokio::test]
async fn delete_user_is_idempotent_for_a_missing_user() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    delete_user(&state.graph, "missing").await.expect("delete");
}

#[tokio::test]
async fn delete_article_cascades_versions_and_comments_and_collects_hashes() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    insert_comment(&state, &version_id, &author_id).await;

    let outcome = delete_article(&state.graph, &article_id)
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
async fn delete_article_is_idempotent_for_a_missing_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let outcome = delete_article(&state.graph, "missing").await.expect("delete");
    assert!(outcome.removed_pdf_hashes.is_empty());
}

#[tokio::test]
async fn delete_version_removes_only_the_version_and_refreshes_latest() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, first_version) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    let second_version = uuid::Uuid::now_v7().to_string();
    crate::repository::version::create_version(
        &state.graph,
        &article_id,
        &VersionDraft {
            version_id: second_version.clone(),
            version_number: "2.0.0".to_string(),
            content_hash: pdf_hash(2),
            note: "note".to_string(),
        },
    )
    .await
    .expect("v2");

    let outcome = delete_version(&state.graph, &second_version)
        .await
        .expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(2)]);

    let (remaining, total) = versions_of(&state.graph, &article_id, 10, 0)
        .await
        .expect("versions");
    assert_eq!(total, 1);
    assert_eq!(remaining[0].id, first_version);
}

#[tokio::test]
async fn delete_version_is_idempotent_for_a_missing_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let outcome = delete_version(&state.graph, "missing").await.expect("delete");
    assert!(outcome.removed_pdf_hashes.is_empty());
}

#[tokio::test]
async fn delete_article_removes_a_nested_comment_subtree_and_collects_the_pdf_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    insert_comment_tree(&state, &version_id, &author_id).await;

    let outcome = delete_article(&state.graph, &article_id).await.expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(1)]);

    assert_no_comment_subtree_remains(&state).await;
}

#[tokio::test]
async fn delete_version_removes_a_nested_comment_subtree_and_collects_the_pdf_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    insert_comment_tree(&state, &version_id, &author_id).await;

    let outcome = delete_version(&state.graph, &version_id).await.expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(1)]);

    assert_no_comment_subtree_remains(&state).await;
}

#[tokio::test]
async fn delete_user_removes_a_nested_comment_subtree_and_collects_the_pdf_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    insert_comment_tree(&state, &version_id, &author_id).await;

    let outcome = delete_user(&state.graph, &author_id).await.expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(1)]);

    assert_no_comment_subtree_remains(&state).await;
    assert_eq!(
        crate::repository::user::read_user(&state.graph, &author_id)
            .await
            .expect("read user"),
        None
    );
}
