use super::context::{build_state, test_config};

use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::delete::{
    delete_article, delete_user, delete_version, soft_delete_article, soft_delete_comment,
    soft_delete_version,
};
use crate::repository::schema::{
    CommentRow, EDGE_COMMENT_ATTACH_VERSION, EDGE_COMMENT_REPLY_COMMENT, EDGE_USER_AUTHOR_COMMENT,
    ENTITY_TYPE_COMMENT, ENTITY_TYPE_USER, KEY_SOFT_DELETED, KEY_TYPE, alias_of,
};
use crate::repository::version::{VersionDraft, versions_of};
use agdb::QueryBuilder;

fn pdf_hash(seed: u8) -> String {
    format!("{seed:x}").repeat(32)
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
            tags: vec!["rust".to_string()],
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
                .values([[(KEY_TYPE, EDGE_USER_AUTHOR_COMMENT).into()]])
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
                .values([[(KEY_TYPE, EDGE_COMMENT_ATTACH_VERSION).into()]])
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
                .values([[(KEY_TYPE, EDGE_COMMENT_REPLY_COMMENT).into()]])
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
    assert_eq!(count_by_type(&guard, EDGE_COMMENT_REPLY_COMMENT), 0);
    assert_eq!(count_by_type(&guard, EDGE_COMMENT_ATTACH_VERSION), 0);
    assert_eq!(count_by_type(&guard, EDGE_USER_AUTHOR_COMMENT), 0);
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

    assert!(
        crate::repository::article::read_article(&state.graph, &article_id)
            .await
            .expect("read")
            .is_none()
    );
    assert!(
        crate::repository::version::read_version(&state.graph, &version_id)
            .await
            .expect("read")
            .is_none()
    );
}

#[tokio::test]
async fn delete_article_is_idempotent_for_a_missing_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let outcome = delete_article(&state.graph, "missing")
        .await
        .expect("delete");
    assert!(outcome.removed_pdf_hashes.is_empty());
}

#[tokio::test]
async fn delete_version_removes_only_the_version_and_refreshes_latest() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, first_version) =
        create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
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

    let (remaining, _) = versions_of(&state.graph, &article_id, 10, 0)
        .await
        .expect("versions");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, first_version);
}

#[tokio::test]
async fn delete_version_is_idempotent_for_a_missing_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let outcome = delete_version(&state.graph, "missing")
        .await
        .expect("delete");
    assert!(outcome.removed_pdf_hashes.is_empty());
}

#[tokio::test]
async fn delete_article_removes_a_nested_comment_subtree_and_collects_the_pdf_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    insert_comment_tree(&state, &version_id, &author_id).await;

    let outcome = delete_article(&state.graph, &article_id)
        .await
        .expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(1)]);

    assert_no_comment_subtree_remains(&state).await;
}

#[tokio::test]
async fn delete_version_removes_a_nested_comment_subtree_and_collects_the_pdf_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    insert_comment_tree(&state, &version_id, &author_id).await;

    let outcome = delete_version(&state.graph, &version_id)
        .await
        .expect("delete");
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

async fn has_soft_deleted_flag(
    state: &crate::infrastructure::state::AppState,
    kind: &str,
    id: &str,
) -> bool {
    let guard = state.graph.read().await;
    let Some(node) =
        crate::repository::graph::resolve_node_id_sync(&guard, kind, id).expect("resolve node")
    else {
        return false;
    };
    let result = guard
        .exec(
            QueryBuilder::search()
                .elements()
                .where_()
                .ids([node])
                .and()
                .keys(KEY_SOFT_DELETED)
                .query(),
        )
        .expect("flag search");
    !result.elements.is_empty()
}

#[tokio::test]
async fn soft_delete_article_cascades_the_flag_over_the_subtree() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    insert_comment(&state, &version_id, &author_id).await;

    soft_delete_article(&state.graph, &article_id)
        .await
        .expect("soft delete");

    assert!(has_soft_deleted_flag(&state, "article", &article_id).await);
    assert!(has_soft_deleted_flag(&state, "version", &version_id).await);
    let (remaining, _) = versions_of(&state.graph, &article_id, 10, 0)
        .await
        .expect("versions");
    assert_eq!(remaining.len(), 0, "versions hidden by article flag");
    assert!(
        crate::repository::version::read_version(&state.graph, &version_id)
            .await
            .expect("read version")
            .is_none(),
        "version content hidden under a soft-deleted article"
    );
}

#[tokio::test]
async fn soft_delete_version_cascades_the_flag_over_versions_and_comments() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, first_version) =
        create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
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
    insert_comment(&state, &second_version, &author_id).await;

    soft_delete_version(&state.graph, &second_version)
        .await
        .expect("soft delete");

    assert!(has_soft_deleted_flag(&state, "version", &second_version).await);
    assert!(!has_soft_deleted_flag(&state, "version", &first_version).await);
    assert!(!has_soft_deleted_flag(&state, "article", &article_id).await);
    let (page, _) = crate::repository::comment::read_comments_page_by_version(
        &state.graph,
        &second_version,
        10,
        0,
    )
    .await
    .expect("comment page");
    assert!(
        page.is_empty(),
        "comments hidden under a soft-deleted version"
    );
}

#[tokio::test]
async fn soft_delete_comment_cascades_the_flag_over_the_reply_subtree() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    let top = insert_comment(&state, &version_id, &author_id).await;
    let reply = insert_reply(&state, &top, &author_id).await;

    soft_delete_comment(&state.graph, &top)
        .await
        .expect("soft delete");

    assert!(has_soft_deleted_flag(&state, "comment", &top).await);
    assert!(has_soft_deleted_flag(&state, "comment", &reply).await);
    assert!(
        crate::repository::comment::read_comment_item(&state.graph, &reply)
            .await
            .expect("read reply")
            .is_none(),
        "reply node carries the cascade flag"
    );
}

#[tokio::test]
async fn soft_delete_is_idempotent_for_a_missing_node() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    soft_delete_article(&state.graph, "missing")
        .await
        .expect("soft delete");
    soft_delete_version(&state.graph, "missing")
        .await
        .expect("soft delete");
    soft_delete_comment(&state.graph, "missing")
        .await
        .expect("soft delete");
}

#[tokio::test]
async fn clearing_the_soft_deleted_flag_revives_the_node() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    let top = insert_comment(&state, &version_id, &author_id).await;

    soft_delete_article(&state.graph, &article_id)
        .await
        .expect("soft delete article");
    soft_delete_version(&state.graph, &version_id)
        .await
        .expect("soft delete version");
    soft_delete_comment(&state.graph, &top)
        .await
        .expect("soft delete comment");

    crate::repository::delete::clear_soft_deleted_flag(&state.graph, &article_id)
        .await
        .expect("clear article");
    crate::repository::delete::clear_soft_deleted_flag(&state.graph, &version_id)
        .await
        .expect("clear version");
    crate::repository::delete::clear_soft_deleted_flag(&state.graph, &top)
        .await
        .expect("clear comment");

    assert!(
        crate::repository::article::read_article(&state.graph, &article_id)
            .await
            .expect("read article")
            .is_some(),
        "article revived"
    );
    assert!(
        crate::repository::version::read_version(&state.graph, &version_id)
            .await
            .expect("read version")
            .is_some(),
        "version revived"
    );
    assert!(
        crate::repository::comment::read_comment_item(&state.graph, &top)
            .await
            .expect("read comment")
            .is_some(),
        "comment revived"
    );
    assert!(!has_soft_deleted_flag(&state, "article", &article_id).await);
    assert!(!has_soft_deleted_flag(&state, "version", &version_id).await);
    assert!(!has_soft_deleted_flag(&state, "comment", &top).await);
}

#[tokio::test]
async fn soft_deleted_article_read_returns_none_and_versions_hidden() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;

    soft_delete_article(&state.graph, &article_id)
        .await
        .expect("soft delete");

    assert!(
        crate::repository::article::read_article(&state.graph, &article_id)
            .await
            .expect("read article")
            .is_none(),
        "deleted article is not found"
    );
    let (remaining, _) = versions_of(&state.graph, &article_id, 10, 0)
        .await
        .expect("versions");
    assert_eq!(
        remaining.len(),
        0,
        "versions hidden after article soft delete"
    );
}

#[tokio::test]
async fn soft_deleted_latest_version_falls_back_to_the_live_latest() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, first_version) =
        create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
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

    soft_delete_version(&state.graph, &second_version)
        .await
        .expect("soft delete v2");

    let view = crate::repository::article::read_article(&state.graph, &article_id)
        .await
        .expect("read article")
        .expect("article");
    assert_eq!(
        view.latest_version_id, first_version,
        "falls back to live latest"
    );
    assert_eq!(view.latest_version, "1.0.0");
}

#[tokio::test]
async fn soft_deleted_only_version_leaves_no_dangling_latest() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;

    soft_delete_version(&state.graph, &version_id)
        .await
        .expect("soft delete");

    let view = crate::repository::article::read_article(&state.graph, &article_id)
        .await
        .expect("read article")
        .expect("article");
    assert!(view.latest_version_id.is_empty(), "no dangling latest id");
    assert!(view.latest_version.is_empty(), "no dangling latest number");
}

#[tokio::test]
async fn versions_of_excludes_soft_deleted_versions_and_reports_has_next() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, first_version) =
        create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
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
    let third_version = uuid::Uuid::now_v7().to_string();
    crate::repository::version::create_version(
        &state.graph,
        &article_id,
        &VersionDraft {
            version_id: third_version.clone(),
            version_number: "3.0.0".to_string(),
            content_hash: pdf_hash(3),
            note: "note".to_string(),
        },
    )
    .await
    .expect("v3");

    soft_delete_version(&state.graph, &second_version)
        .await
        .expect("soft delete v2");

    let (page, has_next) = versions_of(&state.graph, &article_id, 1, 0)
        .await
        .expect("page one");
    assert_eq!(page.len(), 1, "page one has one live version");
    assert!(has_next, "second live version exists");
    assert!(
        page.iter().all(|item| item.id != second_version),
        "deleted version excluded"
    );

    let (page_two, has_next) = versions_of(&state.graph, &article_id, 1, 1)
        .await
        .expect("page two");
    assert_eq!(page_two.len(), 1);
    assert!(!has_next, "no further live versions");
    let mut seen: Vec<&str> = page
        .iter()
        .chain(&page_two)
        .map(|item| item.id.as_str())
        .collect();
    seen.sort_unstable();
    let mut live: Vec<&str> = vec![first_version.as_str(), third_version.as_str()];
    live.sort_unstable();
    assert_eq!(seen, live, "pages tile exactly the live versions");
}

#[tokio::test]
async fn read_version_returns_none_for_a_soft_deleted_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;

    soft_delete_version(&state.graph, &version_id)
        .await
        .expect("soft delete");

    assert!(
        crate::repository::version::read_version(&state.graph, &version_id)
            .await
            .expect("read version")
            .is_none(),
        "deleted version is not found"
    );
}

#[tokio::test]
async fn comment_page_hides_soft_deleted_comments_and_their_replies() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    let top = insert_comment(&state, &version_id, &author_id).await;
    let _reply = insert_reply(&state, &top, &author_id).await;

    soft_delete_comment(&state.graph, &top)
        .await
        .expect("soft delete top");

    let (page, has_next) =
        crate::repository::comment::read_comments_page_by_version(&state.graph, &version_id, 10, 0)
            .await
            .expect("comment page");
    assert!(page.is_empty(), "deleted top-level comment hidden");
    assert!(!has_next);
    let children_error =
        crate::repository::comment::read_comment_children_page(&state.graph, &top, 10, 0)
            .await
            .expect_err("children page hidden with deleted parent");
    assert!(
        crate::repository::graph::is_not_found(&children_error),
        "deleted parent surfaces as not found, got {children_error:?}"
    );
    assert!(
        crate::repository::comment::read_comment_item(&state.graph, &top)
            .await
            .expect("read top")
            .is_none(),
        "deleted comment item hidden"
    );
}

#[tokio::test]
async fn comment_page_tiles_around_soft_deleted_comments() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    let first = insert_comment(&state, &version_id, &author_id).await;
    let second = insert_comment(&state, &version_id, &author_id).await;
    let third = insert_comment(&state, &version_id, &author_id).await;

    soft_delete_comment(&state.graph, &second)
        .await
        .expect("soft delete second");

    let (page_one, has_next) =
        crate::repository::comment::read_comments_page_by_version(&state.graph, &version_id, 1, 0)
            .await
            .expect("page one");
    assert_eq!(page_one.len(), 1);
    assert!(has_next, "one of two live comments remains");
    let (page_two, has_next) =
        crate::repository::comment::read_comments_page_by_version(&state.graph, &version_id, 1, 1)
            .await
            .expect("page two");
    assert_eq!(page_two.len(), 1);
    assert!(!has_next);
    let mut seen: Vec<String> = page_one
        .into_iter()
        .chain(page_two)
        .map(|item| item.id)
        .collect();
    seen.sort_unstable();
    let mut live: Vec<String> = vec![first.clone(), third.clone()];
    live.sort_unstable();
    assert_eq!(seen, live, "pages tile exactly the live comments");
}
