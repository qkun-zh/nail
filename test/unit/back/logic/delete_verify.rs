use nail_common::request::UserDeleteQuery;

use super::context::{TestCtx, unique_pdf, valid_pdf};
use crate::logic::error::LogicError;
use crate::repository::role::{ROLE_ADMIN, ROLE_MEMBER, hold_role};

const TEST_TAGS: &[&str] = &["rust", "backend", "frontend", "devops"];

async fn member(context: &TestCtx, email: &str) -> String {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::email(email),
    )
    .await
    .expect("user");
    hold_role(&context.state.database, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
    user_id
}

async fn admin(context: &TestCtx, email: &str) -> String {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::email(email),
    )
    .await
    .expect("user");
    hold_role(&context.state.database, &user_id, ROLE_ADMIN)
        .await
        .expect("admin role");
    user_id
}

async fn create_seeded_article(
    context: &TestCtx,
    actor_id: &str,
    title: &str,
    version: &str,
    note: &str,
) -> (String, String) {
    context.seed_tags(TEST_TAGS).await;
    crate::logic::article::create_article(
        &context.state,
        actor_id,
        crate::logic::article::ArticleCreateInput {
            title,
            summary: "summary",
            tags: "rust",
            version,
            note,
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create article")
}

#[tokio::test]
async fn hard_delete_article_removes_versions_comments_and_search_docs() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context, "admin@example.com").await;
    let (article_id, version_id) =
        create_seeded_article(&context, &owner, "Hard Full Teardown", "1.0.0", "note").await;
    crate::logic::comment::create_comment(&context.state, &owner, &version_id, "comment marker")
        .await
        .expect("comment");

    crate::logic::article::delete_article(
        &context.state,
        &admin_id,
        &article_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect("hard delete");

    assert!(
        crate::repository::article::read_article(&context.state.database, &article_id)
            .await
            .expect("read")
            .is_none(),
        "article node must be gone"
    );
    assert!(
        crate::repository::version::versions_of(&context.state.database, &article_id, 10, 0)
            .await
            .expect("versions")
            .0
            .is_empty(),
        "no versions may survive a hard article delete"
    );
    let page = crate::logic::search::search_articles(
        &context.state,
        &owner,
        &nail_common::request::ArticleSearchParams {
            q: Some("teardown".to_string()),
            ranges: Some("title".to_string()),
            from: None,
            to: None,
            limit: None,
            page: None,
        },
    )
    .await
    .expect("search");
    assert!(page.items.is_empty(), "search docs must be gone");
}

#[tokio::test]
async fn hard_delete_version_removes_only_that_version_and_its_comments() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context, "admin@example.com").await;
    let (article_id, first_version_id) =
        create_seeded_article(&context, &owner, "Surgical Version", "1.0.0", "first").await;
    let second_version_id = crate::logic::version::create_version(
        &context.state,
        &owner,
        &article_id,
        "2.0.0",
        "second",
        context.upload(&unique_pdf("sv2")),
    )
    .await
    .expect("create v2");
    crate::logic::comment::create_comment(&context.state, &owner, &first_version_id, "doomed")
        .await
        .expect("comment on v1");
    crate::logic::comment::create_comment(&context.state, &owner, &second_version_id, "survivor")
        .await
        .expect("comment on v2");

    crate::logic::version::delete_version(
        &context.state,
        &admin_id,
        &first_version_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect("hard delete v1");

    assert!(
        crate::repository::version::read_version(&context.state.database, &first_version_id)
            .await
            .expect("read v1")
            .is_none(),
        "v1 must be gone"
    );
    assert!(
        crate::repository::version::read_version(&context.state.database, &second_version_id)
            .await
            .expect("read v2")
            .is_some(),
        "v2 must survive"
    );
    let comments =
        crate::logic::comment::read_comments(&context.state, &owner, &second_version_id, 1, 10)
            .await
            .expect("comments of v2");
    assert_eq!(comments.items.len(), 1, "v2 comment must survive");
}

#[tokio::test]
async fn hard_delete_comment_removes_the_subtree_but_keeps_siblings() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context, "admin@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &owner, "Comment Tree", "1.0.0", "note").await;
    let doomed =
        crate::logic::comment::create_comment(&context.state, &owner, &version_id, "doomed top")
            .await
            .expect("doomed top");
    crate::logic::comment::create_reply(&context.state, &owner, &doomed, "doomed reply")
        .await
        .expect("doomed reply");
    let sibling = crate::logic::comment::create_comment(
        &context.state,
        &owner,
        &version_id,
        "sibling comment",
    )
    .await
    .expect("sibling");

    crate::logic::comment::delete_comment(
        &context.state,
        &admin_id,
        &doomed,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect("hard delete doomed top");

    let comments = crate::logic::comment::read_comments(&context.state, &owner, &version_id, 1, 10)
        .await
        .expect("comments");
    assert_eq!(
        comments.items.len(),
        1,
        "only the sibling must remain at top level"
    );
    assert_eq!(comments.items[0].id, sibling);
}

#[tokio::test]
async fn soft_deleted_article_can_still_be_hard_deleted() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context, "admin@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &owner, "Soft Then Hard", "1.0.0", "note").await;

    crate::logic::article::delete_article(
        &context.state,
        &owner,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");
    crate::logic::article::delete_article(
        &context.state,
        &admin_id,
        &article_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect("hard delete after soft");

    assert!(
        crate::repository::article::read_article(&context.state.database, &article_id)
            .await
            .expect("read")
            .is_none(),
        "soft flag must not block a later hard delete"
    );
}

#[tokio::test]
async fn soft_deleted_version_can_still_be_hard_deleted() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context, "admin@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &owner, "Version Soft Then Hard", "1.0.0", "note").await;

    crate::logic::version::delete_version(
        &context.state,
        &owner,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");
    crate::logic::version::delete_version(
        &context.state,
        &admin_id,
        &version_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect("hard delete after soft");

    assert!(
        crate::repository::version::read_version(&context.state.database, &version_id)
            .await
            .expect("read")
            .is_none(),
        "soft flag must not block a later hard delete"
    );
}

#[tokio::test]
async fn transfer_article_repoints_ownership_but_keeps_content_readable() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (article_id, version_id) =
        create_seeded_article(&context, &owner, "Transferred Title", "1.0.0", "note").await;

    crate::logic::article::delete_article(
        &context.state,
        &owner,
        &article_id,
        Some(nail_common::request::DeleteMode::Transfer),
    )
    .await
    .expect("transfer");

    assert!(
        crate::repository::article::read_article(&context.state.database, &article_id)
            .await
            .expect("read")
            .is_some(),
        "transferred article must remain readable"
    );
    let versions = crate::logic::version::read_versions(&context.state, &owner, &article_id, 1, 10)
        .await
        .expect("versions");
    assert_eq!(versions.items.len(), 1, "version must survive transfer");
    let _ = version_id;
}

#[tokio::test]
async fn transfer_article_updates_the_search_author_name() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &owner, "Transferred Search", "1.0.0", "note").await;

    crate::logic::article::delete_article(
        &context.state,
        &owner,
        &article_id,
        Some(nail_common::request::DeleteMode::Transfer),
    )
    .await
    .expect("transfer");

    let recycler_id = crate::repository::user::read_user_by_email_address_hash(
        &context.state.database,
        &nail_common::hash::email("user-zero@example.com"),
    )
    .await
    .expect("lookup recycler")
    .expect("seeded recycler");
    let recycler_name = crate::repository::user::read_user(&context.state.database, &recycler_id)
        .await
        .expect("read recycler")
        .expect("recycler exists")
        .name;

    let page = crate::logic::search::search_articles(
        &context.state,
        &owner,
        &nail_common::request::ArticleSearchParams {
            q: Some("transferred".to_string()),
            ranges: Some("title".to_string()),
            from: None,
            to: None,
            limit: None,
            page: None,
        },
    )
    .await
    .expect("search");
    assert_eq!(page.items.len(), 1);
    assert_eq!(
        page.items[0].author_name, recycler_name,
        "search must reflect the recycler as the new author"
    );
}

#[tokio::test]
async fn transfer_comment_repoints_ownership_but_keeps_it_visible() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &owner, "Transfer Comment", "1.0.0", "note").await;
    let comment_id =
        crate::logic::comment::create_comment(&context.state, &owner, &version_id, "transfer me")
            .await
            .expect("comment");

    crate::logic::comment::delete_comment(
        &context.state,
        &owner,
        &comment_id,
        Some(nail_common::request::DeleteMode::Transfer),
    )
    .await
    .expect("transfer comment");

    let comments = crate::logic::comment::read_comments(&context.state, &owner, &version_id, 1, 10)
        .await
        .expect("comments");
    assert_eq!(
        comments.items.len(),
        1,
        "transferred comment must stay visible"
    );
}

#[tokio::test]
async fn delete_missing_article_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context, "admin@example.com").await;

    let error = crate::logic::article::delete_article(
        &context.state,
        &admin_id,
        "missing-article-id",
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect_err("missing article");
    assert!(matches!(error, LogicError::NotFound(_)));
    let _ = owner;
}

#[tokio::test]
async fn delete_missing_version_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let admin_id = admin(&context, "admin@example.com").await;

    let error = crate::logic::version::delete_version(
        &context.state,
        &admin_id,
        "missing-version-id",
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect_err("missing version");
    assert!(matches!(error, LogicError::NotFound(_)));
}

#[tokio::test]
async fn delete_missing_comment_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let admin_id = admin(&context, "admin@example.com").await;

    let error = crate::logic::comment::delete_comment(
        &context.state,
        &admin_id,
        "missing-comment-id",
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect_err("missing comment");
    assert!(matches!(error, LogicError::NotFound(_)));
}

#[tokio::test]
async fn delete_article_without_a_mode_is_a_bad_request() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (article_id, _) = create_seeded_article(&context, &owner, "No Mode", "1.0.0", "note").await;

    let error = crate::logic::article::delete_article(&context.state, &owner, &article_id, None)
        .await
        .expect_err("no mode");
    assert!(matches!(error, LogicError::BadRequest(_)));
}

#[tokio::test]
async fn delete_version_without_a_mode_is_a_bad_request() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &owner, "No Mode V", "1.0.0", "note").await;

    let error = crate::logic::version::delete_version(&context.state, &owner, &version_id, None)
        .await
        .expect_err("no mode");
    assert!(matches!(error, LogicError::BadRequest(_)));
}

#[tokio::test]
async fn delete_comment_without_a_mode_is_a_bad_request() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &owner, "No Mode C", "1.0.0", "note").await;
    let comment_id =
        crate::logic::comment::create_comment(&context.state, &owner, &version_id, "comment")
            .await
            .expect("comment");

    let error = crate::logic::comment::delete_comment(&context.state, &owner, &comment_id, None)
        .await
        .expect_err("no mode");
    assert!(matches!(error, LogicError::BadRequest(_)));
}

#[tokio::test]
async fn member_owner_cannot_hard_delete_own_article() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &owner, "Owner No Hard", "1.0.0", "note").await;

    let error = crate::logic::article::delete_article(
        &context.state,
        &owner,
        &article_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect_err("member must not hard delete");
    assert!(matches!(error, LogicError::Forbidden(_)));
    assert!(
        crate::repository::article::read_article(&context.state.database, &article_id)
            .await
            .expect("read")
            .is_some(),
        "article must survive the forbidden attempt"
    );
}

#[tokio::test]
async fn member_owner_cannot_hard_delete_own_version() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &owner, "Owner No Hard V", "1.0.0", "note").await;

    let error = crate::logic::version::delete_version(
        &context.state,
        &owner,
        &version_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect_err("member must not hard delete");
    assert!(matches!(error, LogicError::Forbidden(_)));
}

#[tokio::test]
async fn member_owner_cannot_hard_delete_own_comment() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &owner, "Owner No Hard C", "1.0.0", "note").await;
    let comment_id =
        crate::logic::comment::create_comment(&context.state, &owner, &version_id, "comment")
            .await
            .expect("comment");

    let error = crate::logic::comment::delete_comment(
        &context.state,
        &owner,
        &comment_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect_err("member must not hard delete");
    assert!(matches!(error, LogicError::Forbidden(_)));
}

#[tokio::test]
async fn admin_can_hard_delete_a_members_article() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context, "admin@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &owner, "Admin Hard", "1.0.0", "note").await;

    crate::logic::article::delete_article(
        &context.state,
        &admin_id,
        &article_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect("admin hard delete");
    assert!(
        crate::repository::article::read_article(&context.state.database, &article_id)
            .await
            .expect("read")
            .is_none()
    );
}

#[tokio::test]
async fn stranger_member_cannot_soft_delete_others_article() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let stranger = member(&context, "mallory@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &owner, "Stranger Soft", "1.0.0", "note").await;

    let error = crate::logic::article::delete_article(
        &context.state,
        &stranger,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect_err("stranger must not soft delete");
    assert!(matches!(error, LogicError::Forbidden(_)));
}

#[tokio::test]
async fn hard_delete_user_removes_content_and_search_docs() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context, "admin@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &owner, "User Hard", "1.0.0", "note").await;

    crate::logic::user::delete_user(
        &context.state,
        &admin_id,
        &owner,
        UserDeleteQuery {
            mode: Some(nail_common::request::DeleteMode::Hard),
            pow: serde_json::to_string(&context.issued_pow("alice@example.com")).unwrap(),
        },
    )
    .await
    .expect("hard delete user");

    assert!(
        crate::repository::user::read_user(&context.state.database, &owner)
            .await
            .expect("read user")
            .is_none(),
        "user node must be gone"
    );
    assert!(
        crate::repository::article::read_article(&context.state.database, &article_id)
            .await
            .expect("read article")
            .is_none(),
        "user hard delete must cascade to content"
    );
    let page = crate::logic::search::search_articles(
        &context.state,
        &admin_id,
        &nail_common::request::ArticleSearchParams {
            q: Some("user".to_string()),
            ranges: Some("title".to_string()),
            from: None,
            to: None,
            limit: None,
            page: None,
        },
    )
    .await
    .expect("search");
    assert!(page.items.is_empty(), "search must be cleaned up");
}

#[tokio::test]
async fn soft_delete_keeps_article_identity_while_hiding_it() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (article_id, _) =
        create_seeded_article(&context, &owner, "Soft Identity", "1.0.0", "note").await;

    crate::logic::article::delete_article(
        &context.state,
        &owner,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    assert_eq!(
        crate::logic::article::read_article(&context.state, &owner, &article_id)
            .await
            .expect_err("soft-deleted article hidden"),
        LogicError::not_found("article not found")
    );
    assert!(
        crate::repository::article::read_article(&context.state.database, &article_id)
            .await
            .expect("read")
            .is_some(),
        "the node must survive for identity/occupancy"
    );
    let guard = context.state.database.read().await;
    let holder = crate::repository::graph::resolve_node_id(
        &guard,
        crate::repository::schema::ENTITY_TYPE_ARTICLE,
        &article_id,
    )
    .expect("resolve node");
    assert!(
        holder.is_some(),
        "the node must survive for identity/occupancy"
    );
}

#[tokio::test]
async fn soft_deleted_article_hides_its_whole_subtree_and_rejects_writes() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (article_id, version_id) =
        create_seeded_article(&context, &owner, "Subtree Hidden", "1.0.0", "note").await;
    let top = crate::logic::comment::create_comment(
        &context.state,
        &owner,
        &version_id,
        "subtree top comment",
    )
    .await
    .expect("top comment");
    crate::logic::comment::create_reply(&context.state, &owner, &top, "subtree reply")
        .await
        .expect("reply");

    crate::logic::article::delete_article(
        &context.state,
        &owner,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    let (versions, _) =
        crate::repository::version::versions_of(&context.state.database, &article_id, 10, 0)
            .await
            .expect("versions");
    assert!(versions.is_empty(), "version list hidden");
    assert_eq!(
        crate::logic::version::read_version(&context.state, &owner, &version_id, None)
            .await
            .expect_err("version detail hidden"),
        LogicError::not_found("version not found")
    );
    let comments = crate::logic::comment::read_comments(&context.state, &owner, &version_id, 1, 50)
        .await
        .expect_err("comment page hidden");
    assert_eq!(comments, LogicError::not_found("version not found"));
    assert_eq!(
        crate::logic::comment::read_comment(&context.state, &owner, &top)
            .await
            .expect_err("comment hidden"),
        LogicError::not_found("comment not found")
    );

    assert_eq!(
        crate::logic::version::create_version(
            &context.state,
            &owner,
            &article_id,
            "2.0.0",
            "n",
            context.upload(&unique_pdf("subtree-hidden")),
        )
        .await
        .expect_err("create version on hidden article"),
        LogicError::not_found("article not found")
    );
    assert_eq!(
        crate::logic::comment::create_comment(&context.state, &owner, &version_id, "late")
            .await
            .expect_err("create comment on hidden version"),
        LogicError::not_found("comment target not found (the version may have been removed)")
    );
    assert_eq!(
        crate::logic::comment::create_reply(&context.state, &owner, &top, "late reply")
            .await
            .expect_err("create reply on hidden thread"),
        LogicError::not_found("reply target not found (the parent comment may have been removed)")
    );

    let page = crate::logic::search::search_articles(
        &context.state,
        &owner,
        &nail_common::request::ArticleSearchParams {
            q: Some("subtree".to_string()),
            ranges: Some("title".to_string()),
            from: None,
            to: None,
            limit: None,
            page: None,
        },
    )
    .await
    .expect("search");
    assert!(page.items.is_empty(), "article gone from search");
}

#[tokio::test]
async fn soft_deleted_article_restore_brings_back_the_whole_subtree() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (article_id, version_id) =
        create_seeded_article(&context, &owner, "Restore All", "1.0.0", "note").await;
    crate::logic::comment::create_comment(&context.state, &owner, &version_id, "restored comment")
        .await
        .expect("comment");

    crate::logic::article::delete_article(
        &context.state,
        &owner,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    crate::repository::delete::clear_soft_deleted_flag(&context.state.database, &article_id)
        .await
        .expect("restore");

    assert!(
        crate::repository::article::read_article(&context.state.database, &article_id)
            .await
            .expect("read article")
            .is_some(),
        "article readable again"
    );
    let (versions, _) =
        crate::repository::version::versions_of(&context.state.database, &article_id, 10, 0)
            .await
            .expect("versions");
    assert_eq!(versions.len(), 1, "version list back");
    let comments = crate::logic::comment::read_comments(&context.state, &owner, &version_id, 1, 50)
        .await
        .expect("comments back");
    assert_eq!(comments.items.len(), 1, "comments back");
}

#[tokio::test]
async fn soft_deleted_version_hides_its_comments_and_download() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (article_id, version_id) =
        create_seeded_article(&context, &owner, "Version Hide", "1.0.0", "note").await;
    let top = crate::logic::comment::create_comment(
        &context.state,
        &owner,
        &version_id,
        "version comment",
    )
    .await
    .expect("comment");

    crate::logic::version::delete_version(
        &context.state,
        &owner,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete version");

    assert_eq!(
        crate::logic::version::read_version(&context.state, &owner, &version_id, None)
            .await
            .expect_err("version detail hidden"),
        LogicError::not_found("version not found")
    );
    assert_eq!(
        crate::logic::comment::read_comments(&context.state, &owner, &version_id, 1, 50)
            .await
            .expect_err("comments hidden"),
        LogicError::not_found("version not found")
    );
    assert_eq!(
        crate::logic::comment::read_comment(&context.state, &owner, &top)
            .await
            .expect_err("comment hidden"),
        LogicError::not_found("comment not found")
    );

    crate::repository::delete::clear_soft_deleted_flag(&context.state.database, &version_id)
        .await
        .expect("restore version");
    assert!(
        crate::repository::version::read_version(&context.state.database, &version_id)
            .await
            .expect("read version")
            .is_some(),
        "version back after restore"
    );
    assert_eq!(
        crate::logic::comment::read_comment(&context.state, &owner, &top)
            .await
            .expect("comment back")
            .id,
        top,
        "comment back after version restore"
    );
    let (versions, _) =
        crate::repository::version::versions_of(&context.state.database, &article_id, 10, 0)
            .await
            .expect("versions");
    assert_eq!(versions.len(), 1, "version listed again");
}

#[tokio::test]
async fn soft_deleted_comment_hides_its_reply_subtree() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let (_, version_id) =
        create_seeded_article(&context, &owner, "Reply Hide", "1.0.0", "note").await;
    let top =
        crate::logic::comment::create_comment(&context.state, &owner, &version_id, "top level")
            .await
            .expect("top");
    let reply = crate::logic::comment::create_reply(&context.state, &owner, &top, "first reply")
        .await
        .expect("reply");
    crate::logic::comment::create_reply(&context.state, &owner, &reply, "nested reply")
        .await
        .expect("nested");

    crate::logic::comment::delete_comment(
        &context.state,
        &owner,
        &top,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete top");

    assert_eq!(
        crate::logic::comment::read_comment(&context.state, &owner, &top)
            .await
            .expect_err("top hidden"),
        LogicError::not_found("comment not found")
    );
    assert_eq!(
        crate::logic::comment::read_comment(&context.state, &owner, &reply)
            .await
            .expect_err("reply hidden with parent"),
        LogicError::not_found("comment not found")
    );

    crate::repository::delete::clear_soft_deleted_flag(&context.state.database, &top)
        .await
        .expect("restore top");
    assert_eq!(
        crate::logic::comment::read_comment(&context.state, &owner, &top)
            .await
            .expect("top back")
            .id,
        top,
        "top back after restore"
    );
    assert_eq!(
        crate::logic::comment::read_comment(&context.state, &owner, &reply)
            .await
            .expect("reply back")
            .id,
        reply,
        "reply back after restore"
    );
}
