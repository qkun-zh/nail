use super::context::{build_state, test_config};

use common::request::DeleteMode;

use crate::infrastructure::state::AppState;
use crate::logic::comment::{
    create_comment, create_reply, delete_comment, read_comment, read_comment_children,
    read_comments, update_comment,
};
use crate::logic::error::LogicError;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::role::{ROLE_MEMBER, hold_role};
use crate::repository::version::VersionDraft;

fn create_user(state: &AppState, email: &str) -> String {
    crate::repository::user::create_user(
        &state.database,
        &common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .expect("user")
}

fn member(state: &AppState, email: &str) -> String {
    let user_id = create_user(state, email);
    hold_role(&state.database, &user_id, ROLE_MEMBER).expect("member");
    user_id
}

fn admin(state: &AppState) -> String {
    crate::repository::user::read_user_by_email_address_hash(
        &state.database,
        &common::hash::hash("user-zero@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("lookup user zero")
    .expect("seeded user zero")
}

fn create_version_fixture(state: &AppState, author_id: &str) -> String {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.database,
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
    .expect("create article");
    version_id
}

#[tokio::test]
async fn create_comment_requires_the_comment_create_permission() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);

    let error = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect_err("no permission");
    assert!(matches!(error, LogicError::Forbidden(_)));
}

#[tokio::test]
async fn create_comment_creates_a_top_level_comment_for_a_member() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);

    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");
    assert!(!comment_id.is_empty());
}

#[tokio::test]
async fn create_comment_reports_a_missing_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");

    let error = create_comment(&state, &author_id, "missing-version", "hello")
        .await
        .expect_err("missing version");
    assert!(matches!(error, LogicError::NotFound(_)));
}

#[tokio::test]
async fn create_reply_reports_a_thread_too_deep() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);

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
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);
    let top = create_comment(&state, &author_id, &version_id, "top")
        .await
        .expect("top");
    let reply = create_reply(&state, &author_id, &top, "reply")
        .await
        .expect("reply");

    let data = read_comments(&state, &author_id, &version_id, 1, 8).expect("read");
    let comments = &data.items;
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].id, top);
    assert!(comments[0].parent_id.is_none());
    assert_eq!(comments[0].child_count, 1);
    assert!(!comments[0].user_name.is_empty());

    let children = read_comment_children(&state, &author_id, &top, 1, 8).expect("children");
    let child_list = &children.items;
    assert_eq!(child_list.len(), 1);
    assert_eq!(child_list[0].id, reply);
    assert_eq!(child_list[0].parent_id.as_deref(), Some(top.as_str()));
    assert_eq!(child_list[0].child_count, 0);

    let single = read_comment(&state, &author_id, &top).expect("single");
    assert_eq!(single.id, top);
    assert_eq!(single.content, "top");
    assert_eq!(single.child_count, 1);
}

#[tokio::test]
async fn read_comments_reports_a_missing_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");

    let error =
        read_comments(&state, &author_id, "missing-version", 1, 8).expect_err("missing version");
    assert!(matches!(error, LogicError::NotFound(_)));
}

#[tokio::test]
async fn read_comment_functions_deny_a_user_without_the_grant() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);
    let comment_id = create_comment(&state, &author_id, &version_id, "top")
        .await
        .expect("top");
    let outsider = create_user(&state, "stranger@example.com");

    let error = read_comments(&state, &outsider, &version_id, 1, 8).expect_err("denied read");
    assert_eq!(error, LogicError::forbidden("you are denied"));

    let error = read_comment(&state, &outsider, &comment_id).expect_err("denied read");
    assert_eq!(error, LogicError::forbidden("you are denied"));

    let error =
        read_comment_children(&state, &outsider, &comment_id, 1, 8).expect_err("denied read");
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn read_comments_rejects_a_non_uuidv7_comment_id() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);
    crate::repository::comment::create_top_level_comment(
        &state.database,
        "not-a-uuid",
        &author_id,
        &version_id,
        "corrupt",
    )
    .expect("corrupt comment");

    let error =
        read_comments(&state, &author_id, &version_id, 1, 8).expect_err("invalid comment id");
    assert!(matches!(error, LogicError::BadRequest(message) if message == "invalid comment id"));
}

#[tokio::test]
async fn update_comment_allows_the_comment_author_and_rejects_a_non_owner() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let stranger = member(&state, "bob@example.com");
    let version_id = create_version_fixture(&state, &author_id);
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
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);
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
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    delete_comment(&state, &author_id, &comment_id, Some(DeleteMode::Transfer))
        .await
        .expect("transfer");

    let owner =
        crate::repository::comment::owner_of_comment(&state.database, &comment_id).expect("owner");
    assert!(owner.is_some());
    assert_ne!(owner.as_deref(), Some(author_id.as_str()));
}

#[tokio::test]
async fn delete_comment_hard_removes_the_subtree_as_admin() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let admin_id = admin(&state);
    let version_id = create_version_fixture(&state, &author_id);
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
        crate::repository::comment::owner_of_comment(&state.database, &top).expect("owner"),
        None
    );
    assert_eq!(
        crate::repository::comment::owner_of_comment(&state.database, &reply).expect("owner"),
        None
    );
}

#[tokio::test]
async fn delete_comment_hard_is_forbidden_for_a_member_owner() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    let error = delete_comment(&state, &author_id, &comment_id, Some(DeleteMode::Hard))
        .await
        .expect_err("member cannot hard delete");
    assert!(matches!(error, LogicError::Forbidden(_)));
}

#[tokio::test]
async fn delete_comment_soft_hides_the_comment_and_its_replies() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);
    let top = create_comment(&state, &author_id, &version_id, "top")
        .await
        .expect("top");
    let reply = create_reply(&state, &author_id, &top, "reply")
        .await
        .expect("reply");

    delete_comment(&state, &author_id, &top, Some(DeleteMode::Soft))
        .await
        .expect("soft delete");

    let error = read_comment(&state, &author_id, &top).expect_err("soft-deleted comment");
    assert_eq!(error, LogicError::not_found("comment not found"));
    let page = read_comments(&state, &author_id, &version_id, 1, 50).expect("comments");
    assert!(
        page.items.is_empty(),
        "soft-deleted top-level comment hidden from the version page"
    );
    assert!(
        read_comment(&state, &author_id, &reply).expect_err("reply hidden with its parent")
            == LogicError::not_found("comment not found"),
        "reply is hidden once its parent is soft-deleted"
    );
}

#[tokio::test]
async fn delete_comment_soft_is_forbidden_for_a_stranger() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let stranger = member(&state, "bob@example.com");
    let version_id = create_version_fixture(&state, &author_id);
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    let error = delete_comment(&state, &stranger, &comment_id, Some(DeleteMode::Soft))
        .await
        .expect_err("stranger soft delete forbidden");
    assert!(matches!(error, LogicError::Forbidden(_)));
    assert!(
        read_comment(&state, &author_id, &comment_id)
            .expect("read")
            .id
            == comment_id,
        "comment untouched"
    );
}

#[tokio::test]
async fn delete_comment_soft_keeps_the_owner_edge() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    delete_comment(&state, &author_id, &comment_id, Some(DeleteMode::Soft))
        .await
        .expect("soft delete");

    assert_eq!(
        crate::repository::comment::owner_of_comment(&state.database, &comment_id)
            .expect("owner")
            .as_deref(),
        Some(author_id.as_str()),
        "soft delete keeps the author edge"
    );
}

#[tokio::test]
async fn delete_comment_soft_is_rejected_for_an_already_hidden_comment() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    delete_comment(&state, &author_id, &comment_id, Some(DeleteMode::Soft))
        .await
        .expect("first soft delete");

    let error = delete_comment(&state, &author_id, &comment_id, Some(DeleteMode::Soft))
        .await
        .expect_err("second soft delete");
    assert_eq!(
        error,
        LogicError::bad_request("already soft-deleted"),
        "repeated soft delete is rejected at the logic layer"
    );
}

#[tokio::test]
async fn undelete_soft_comment_revives_the_comment_as_admin() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let admin_id = crate::repository::user::read_user_by_email_address_hash(
        &state.database,
        &common::hash::hash("user-zero@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("lookup user zero")
    .expect("seeded user zero");
    let version_id = create_version_fixture(&state, &author_id);
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    delete_comment(&state, &author_id, &comment_id, Some(DeleteMode::Soft))
        .await
        .expect("soft delete");

    let data = crate::logic::comment::undelete_soft_comment(&state, &admin_id, &comment_id)
        .await
        .expect("undelete");
    assert_eq!(data.comment_id, comment_id);

    read_comment(&state, &author_id, &comment_id).expect("comment visible again");
}

#[tokio::test]
async fn undelete_soft_comment_is_forbidden_for_a_member() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let version_id = create_version_fixture(&state, &author_id);
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    delete_comment(&state, &author_id, &comment_id, Some(DeleteMode::Soft))
        .await
        .expect("soft delete");

    let error = crate::logic::comment::undelete_soft_comment(&state, &author_id, &comment_id)
        .await
        .expect_err("member undelete");
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn update_comment_reports_a_missing_comment() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let error = update_comment(&state, &author_id, "missing-comment", "edited")
        .await
        .expect_err("missing comment");
    assert_eq!(error, LogicError::not_found("comment not found"));
}

#[tokio::test]
async fn undelete_soft_comment_rejects_a_comment_that_is_not_soft_deleted() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = member(&state, "alice@example.com");
    let admin_id = crate::repository::user::read_user_by_email_address_hash(
        &state.database,
        &common::hash::hash("user-zero@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("lookup user zero")
    .expect("seeded user zero");
    let version_id = create_version_fixture(&state, &author_id);
    let comment_id = create_comment(&state, &author_id, &version_id, "hello")
        .await
        .expect("create");

    let error = crate::logic::comment::undelete_soft_comment(&state, &admin_id, &comment_id)
        .await
        .expect_err("not soft deleted");
    assert_eq!(error, LogicError::bad_request("not soft-deleted"));
}
