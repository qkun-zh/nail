use super::context::{build_state, test_config};

use nail_common::request::DeleteMode;

use crate::infrastructure::state::AppState;
use crate::logic::comment::{
    create_comment, create_reply, delete_comment, read_comment, read_comment_children,
    read_comments, update_comment,
};
use crate::logic::error::LogicError;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::role::{ROLE_MEMBER, hold_role};
use crate::repository::version::VersionDraft;

async fn create_user(state: &AppState, email: &str) -> String {
    crate::repository::user::create_user(&state.graph, &nail_common::hash::email(email))
        .await
        .expect("user")
}

async fn member(state: &AppState, email: &str) -> String {
    let user_id = create_user(state, email).await;
    hold_role(&state.graph, &user_id, ROLE_MEMBER)
        .await
        .expect("member");
    user_id
}

async fn admin(state: &AppState) -> String {
    crate::repository::user::read_user_by_email_address_hash(
        &state.graph,
        &nail_common::hash::email("user-zero@example.com"),
    )
    .await
    .expect("lookup user zero")
    .expect("seeded user zero")
}

async fn create_version_fixture(state: &AppState, author_id: &str) -> String {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: format!("Article {article_id}"),
            summary: "summary".to_string(),
            tags: vec!["rust".to_string()],
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
async fn create_comment_requires_the_comment_create_permission() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;

    let error = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect_err("no permission");
    assert!(matches!(error, LogicError::Forbidden(_)));
}

#[tokio::test]
async fn create_comment_creates_a_top_level_comment_for_a_member() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;

    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");
    assert!(!comment_id.is_empty());
}

#[tokio::test]
async fn create_comment_reports_a_missing_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;

    let error = create_comment(&state, &author_id, "missing-version", "hello")
        .await
        .expect_err("missing version");
    assert!(matches!(error, LogicError::NotFound(_)));
}

#[tokio::test]
async fn create_reply_reports_a_thread_too_deep() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;

    let mut parent = create_comment(&state, &author_id, &version_id, "top")
        .await
        .expect("top");
    for _ in 0..64 {
        parent = create_reply(&state, &author_id, &parent, "reply")
            .await
            .expect("reply");
    }

    let error = create_reply(&state, &author_id, &parent, "too deep")
        .await
        .expect_err("too deep");
    assert!(matches!(error, LogicError::BadRequest(_)));
}

#[tokio::test]
async fn read_comments_returns_top_level_comments_with_child_counts() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let top = create_comment(&state, &author_id, &version_id, "top")
        .await
        .expect("top");
    let reply = create_reply(&state, &author_id, &top, "reply")
        .await
        .expect("reply");

    let data = read_comments(&state, &version_id, 1, 8)
        .await
        .expect("read");
    let comments = &data.comments;
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].id, top);
    assert!(comments[0].parent_id.is_none());
    assert_eq!(comments[0].child_count, 1);
    assert!(!comments[0].user_name.is_empty());

    let children = read_comment_children(&state, &author_id, &top, 1, 8)
        .await
        .expect("children");
    let child_list = &children.comments;
    assert_eq!(child_list.len(), 1);
    assert_eq!(child_list[0].id, reply);
    assert_eq!(child_list[0].parent_id.as_deref(), Some(top.as_str()));
    assert_eq!(child_list[0].child_count, 0);

    let single = read_comment(&state, &author_id, &top)
        .await
        .expect("single");
    assert_eq!(single.id, top);
    assert_eq!(single.content, "top");
    assert_eq!(single.child_count, 1);
}

#[tokio::test]
async fn read_comments_reports_a_missing_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let _ = author_id;

    let error = read_comments(&state, "missing-version", 1, 8)
        .await
        .expect_err("missing version");
    assert!(matches!(error, LogicError::NotFound(_)));
}

#[tokio::test]
async fn read_comments_rejects_a_non_uuidv7_comment_id() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    crate::repository::comment::create_top_level_comment(
        &state.graph,
        "not-a-uuid",
        &author_id,
        &version_id,
        "corrupt",
    )
    .await
    .expect("corrupt comment");

    let error = read_comments(&state, &version_id, 1, 8)
        .await
        .expect_err("invalid comment id");
    assert!(matches!(error, LogicError::BadRequest(message) if message == "invalid comment id"));
}

#[tokio::test]
async fn update_comment_allows_the_comment_author_and_rejects_a_non_owner() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let stranger = member(&state, "bob@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    let error = update_comment(&state, &stranger, &comment_id, "stolen")
        .await
        .expect_err("non owner");
    assert!(matches!(error, LogicError::Forbidden(_)));

    let error = update_comment(&state, &author_id, &comment_id, "   ")
        .await
        .expect_err("empty content");
    assert!(matches!(error, LogicError::BadRequest(_)));

    update_comment(&state, &author_id, &comment_id, "edited")
        .await
        .expect("update");
}

#[tokio::test]
async fn delete_comment_requires_a_mode() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    let error = delete_comment(&state, &author_id, &comment_id, None)
        .await
        .expect_err("missing mode");
    assert!(matches!(error, LogicError::BadRequest(_)));
}

#[tokio::test]
async fn delete_comment_transfer_repoints_the_owner() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    delete_comment(&state, &author_id, &comment_id, Some(DeleteMode::Transfer))
        .await
        .expect("transfer");

    let owner = crate::repository::comment::owner_of_comment(&state.graph, &comment_id)
        .await
        .expect("owner");
    assert!(owner.is_some());
    assert_ne!(owner.as_deref(), Some(author_id.as_str()));
}

#[tokio::test]
async fn delete_comment_hard_removes_the_subtree_as_admin() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let admin_id = admin(&state).await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let top = create_comment(&state, &author_id, &version_id, "top")
        .await
        .expect("top");
    let reply = create_reply(&state, &author_id, &top, "reply")
        .await
        .expect("reply");

    delete_comment(&state, &admin_id, &top, Some(DeleteMode::Hard))
        .await
        .expect("hard delete");

    assert_eq!(
        crate::repository::comment::owner_of_comment(&state.graph, &top)
            .await
            .expect("owner"),
        None
    );
    assert_eq!(
        crate::repository::comment::owner_of_comment(&state.graph, &reply)
            .await
            .expect("owner"),
        None
    );
}

#[tokio::test]
async fn delete_comment_hard_is_forbidden_for_a_member_owner() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    let error = delete_comment(&state, &author_id, &comment_id, Some(DeleteMode::Hard))
        .await
        .expect_err("member cannot hard delete");
    assert!(matches!(error, LogicError::Forbidden(_)));
}

#[tokio::test]
async fn delete_comment_soft_hides_the_comment_but_keeps_replies_visible() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let top = create_comment(&state, &author_id, &version_id, "top")
        .await
        .expect("top");
    let reply = create_reply(&state, &author_id, &top, "reply")
        .await
        .expect("reply");

    delete_comment(&state, &author_id, &top, Some(DeleteMode::Soft))
        .await
        .expect("soft delete");

    let error = read_comment(&state, &author_id, &top)
        .await
        .expect_err("soft-deleted comment");
    assert_eq!(error, LogicError::not_found("comment not found"));
    let page = read_comments(&state, &version_id, 1, 50)
        .await
        .expect("comments");
    assert!(
        page.comments.is_empty(),
        "soft-deleted top-level comment hidden from the version page"
    );
    assert!(
        read_comment(&state, &author_id, &reply)
            .await
            .expect("read reply")
            .id
            == reply,
        "reply stays readable after its parent is soft-deleted"
    );
}

#[tokio::test]
async fn delete_comment_soft_is_forbidden_for_a_stranger() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let stranger = member(&state, "bob@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    let error = delete_comment(&state, &stranger, &comment_id, Some(DeleteMode::Soft))
        .await
        .expect_err("stranger soft delete forbidden");
    assert!(matches!(error, LogicError::Forbidden(_)));
    assert!(
        read_comment(&state, &author_id, &comment_id)
            .await
            .expect("read")
            .id
            == comment_id,
        "comment untouched"
    );
}

#[tokio::test]
async fn delete_comment_soft_keeps_the_owner_edge() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com").await;
    let version_id = create_version_fixture(&state, &author_id).await;
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    delete_comment(&state, &author_id, &comment_id, Some(DeleteMode::Soft))
        .await
        .expect("soft delete");

    assert_eq!(
        crate::repository::comment::owner_of_comment(&state.graph, &comment_id)
            .await
            .expect("owner")
            .as_deref(),
        Some(author_id.as_str()),
        "soft delete keeps the author edge"
    );
}
