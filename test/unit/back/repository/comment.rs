use super::context::{build_state, test_config};

use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::comment::{
    CommentTreeItem, CreateCommentError, create_reply_comment, create_top_level_comment,
    owner_of_comment, read_comments_page_by_version, update_comment_content, version_of_comment,
};
use crate::repository::version::VersionDraft;

const MAX_DEPTH: usize = 64;

async fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::create_user(&state.graph, &nail_common::hash::email(email))
        .await
        .expect("user")
}

async fn create_version_fixture(
    state: &crate::infrastructure::state::AppState,
    author_id: &str,
) -> String {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: format!("Article {article_id}"),
            summary: "summary".to_string(),
            tags: vec!["#rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: format!("{:032x}", article_id.len()),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    version_id
}

#[tokio::test]
async fn create_top_level_comment_writes_nodes_and_edges() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let comment_id = uuid::Uuid::now_v7().to_string();

    create_top_level_comment(&state.graph, &comment_id, &author_id, &version_id, "hello")
        .await
        .expect("create");

    assert_eq!(
        owner_of_comment(&state.graph, &comment_id)
            .await
            .expect("owner"),
        Some(author_id.clone())
    );
    assert_eq!(
        version_of_comment(&state.graph, &comment_id)
            .await
            .expect("version"),
        Some(version_id)
    );
}

#[tokio::test]
async fn create_top_level_comment_rejects_a_missing_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;

    let error = create_top_level_comment(
        &state.graph,
        &uuid::Uuid::now_v7().to_string(),
        &author_id,
        "missing-version",
        "hello",
    )
    .await
    .expect_err("missing version");
    assert!(matches!(error, CreateCommentError::TargetNotFound));
}

#[tokio::test]
async fn create_reply_links_to_the_parent_and_is_not_top_level() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let top_id = uuid::Uuid::now_v7().to_string();
    let reply_id = uuid::Uuid::now_v7().to_string();

    create_top_level_comment(&state.graph, &top_id, &author_id, &version_id, "top")
        .await
        .expect("top");
    create_reply_comment(
        &state.graph,
        &reply_id,
        &author_id,
        &top_id,
        "reply",
        MAX_DEPTH,
    )
    .await
    .expect("reply");

    assert_eq!(
        version_of_comment(&state.graph, &reply_id)
            .await
            .expect("version"),
        Some(version_id.clone())
    );

    let (items, total) = read_comments_page_by_version(&state.graph, &version_id, MAX_DEPTH, 10, 0)
        .await
        .expect("read");
    assert_eq!(total, 1);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, top_id);
    assert_eq!(items[0].parent_id, None);
    assert_eq!(items[1].id, reply_id);
    assert_eq!(items[1].parent_id.as_deref(), Some(top_id.as_str()));
}

#[tokio::test]
async fn create_reply_rejects_a_missing_parent() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;

    let error = create_reply_comment(
        &state.graph,
        &uuid::Uuid::now_v7().to_string(),
        &author_id,
        "missing-parent",
        "reply",
        MAX_DEPTH,
    )
    .await
    .expect_err("missing parent");
    assert!(matches!(error, CreateCommentError::TargetNotFound));
}

#[tokio::test]
async fn create_reply_rejects_a_thread_deeper_than_the_cap() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;

    let mut parent = uuid::Uuid::now_v7().to_string();
    create_top_level_comment(&state.graph, &parent, &author_id, &version_id, "top")
        .await
        .expect("top");

    for _ in 0..MAX_DEPTH {
        let next = uuid::Uuid::now_v7().to_string();
        create_reply_comment(&state.graph, &next, &author_id, &parent, "reply", MAX_DEPTH)
            .await
            .expect("reply under cap");
        parent = next;
    }

    let error = create_reply_comment(
        &state.graph,
        &uuid::Uuid::now_v7().to_string(),
        &author_id,
        &parent,
        "too deep",
        MAX_DEPTH,
    )
    .await
    .expect_err("too deep");
    assert!(matches!(error, CreateCommentError::CommentTreeTooDeep));
}

#[tokio::test]
async fn read_comments_pages_top_level_comments_newest_first() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;

    for id in ["top-1", "top-2", "top-3"] {
        create_top_level_comment(&state.graph, id, &author_id, &version_id, "content")
            .await
            .expect("create");
    }

    let (items, total) = read_comments_page_by_version(&state.graph, &version_id, MAX_DEPTH, 2, 0)
        .await
        .expect("read");
    assert_eq!(total, 3);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, "top-3");
    assert_eq!(items[1].id, "top-2");
}

#[tokio::test]
async fn update_comment_content_applies_the_new_text_and_reports_missing() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let comment_id = uuid::Uuid::now_v7().to_string();
    create_top_level_comment(&state.graph, &comment_id, &author_id, &version_id, "before")
        .await
        .expect("create");

    assert!(
        update_comment_content(&state.graph, &comment_id, "after")
            .await
            .expect("update")
    );
    assert!(
        !update_comment_content(&state.graph, "missing", "after")
            .await
            .expect("missing")
    );

    let (items, _) = read_comments_page_by_version(&state.graph, &version_id, MAX_DEPTH, 10, 0)
        .await
        .expect("read");
    assert_eq!(items[0].content, "after");
}

fn _assert_item_shape(_: &CommentTreeItem) {}
