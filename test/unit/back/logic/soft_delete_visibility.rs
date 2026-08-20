use super::context::{TestCtx, unique_pdf};
use crate::logic::error::LogicError;
use crate::repository::role::{ROLE_MEMBER, hold_role};

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

async fn admin(context: &TestCtx) -> String {
    crate::repository::user::read_user_by_email_address_hash(
        &context.state.database,
        &nail_common::hash::email("user-zero@example.com"),
    )
    .await
    .expect("lookup user zero")
    .expect("seeded user zero")
}

async fn article_fixture(context: &TestCtx, actor_id: &str, title: &str) -> (String, String) {
    context.seed_tags(TEST_TAGS).await;
    crate::logic::article::create_article(
        &context.state,
        actor_id,
        crate::logic::article::ArticleCreateInput {
            title,
            summary: "a summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&unique_pdf("visibility")),
        },
    )
    .await
    .expect("create article")
}

#[tokio::test]
async fn soft_deleted_article_needs_undelete_to_be_read() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (article_id, _) = article_fixture(&context, &owner, "Hidden Article").await;

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
            .expect_err("member denied"),
        LogicError::not_found("article not found")
    );
    let view = crate::logic::article::read_article(&context.state, &admin_id, &article_id)
        .await
        .expect("admin holds Article::Undelete::Soft");
    assert_eq!(view.id, article_id);
}

#[tokio::test]
async fn soft_deleted_version_needs_undelete_to_be_read() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (_, version_id) = article_fixture(&context, &owner, "Hidden Version").await;

    crate::logic::version::delete_version(
        &context.state,
        &owner,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    assert_eq!(
        crate::logic::version::read_version(&context.state, &owner, &version_id, None)
            .await
            .expect_err("member denied"),
        LogicError::not_found("version not found")
    );
    let view = crate::logic::version::read_version(&context.state, &admin_id, &version_id, None)
        .await
        .expect("admin holds Version::Undelete::Soft");
    assert_eq!(view.id, version_id);
}

#[tokio::test]
async fn soft_deleted_comment_needs_undelete_to_be_read() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (_, version_id) = article_fixture(&context, &owner, "Hidden Comment").await;
    let comment_id = crate::logic::comment::create_comment(
        &context.state,
        &owner,
        &version_id,
        "hidden comment",
    )
    .await
    .expect("comment");

    crate::logic::comment::delete_comment(
        &context.state,
        &owner,
        &comment_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    assert_eq!(
        crate::logic::comment::read_comment(&context.state, &owner, &comment_id)
            .await
            .expect_err("member denied"),
        LogicError::not_found("comment not found")
    );
    let view = crate::logic::comment::read_comment(&context.state, &admin_id, &comment_id)
        .await
        .expect("admin holds Comment::Undelete::Soft");
    assert_eq!(view.id, comment_id);
}

#[tokio::test]
async fn soft_deleted_version_download_needs_undelete() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (article_id, version_id) = article_fixture(&context, &owner, "Hidden Download").await;

    crate::logic::version::delete_version(
        &context.state,
        &owner,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    assert_eq!(
        crate::logic::download::mint_download_token(
            &context.state,
            &owner,
            &article_id,
            &version_id,
        )
        .await
        .expect_err("member denied"),
        LogicError::not_found("version not found")
    );
    crate::logic::download::mint_download_token(
        &context.state,
        &admin_id,
        &article_id,
        &version_id,
    )
    .await
    .expect("admin holds Version::Undelete::Soft");
}

#[tokio::test]
async fn comments_of_a_soft_deleted_version_are_gated_by_undelete() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (_, version_id) = article_fixture(&context, &owner, "Hidden Thread").await;
    crate::logic::comment::create_comment(&context.state, &owner, &version_id, "thread comment")
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
        crate::logic::comment::read_comments(&context.state, &owner, &version_id, 1, 50)
            .await
            .expect_err("member denied"),
        LogicError::not_found("version not found")
    );
    let page = crate::logic::comment::read_comments(&context.state, &admin_id, &version_id, 1, 50)
        .await
        .expect("admin passes the visibility gate");
    assert_eq!(page.items.len(), 0);
    assert!(!page.has_next);
}

#[tokio::test]
async fn undelete_soft_restores_visibility_for_members() {
    let context = TestCtx::new().await.expect("test context");
    let owner = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (article_id, version_id) = article_fixture(&context, &owner, "Restored Article").await;

    crate::logic::article::delete_article(
        &context.state,
        &owner,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");
    crate::logic::article::undelete_soft_article(&context.state, &admin_id, &article_id)
        .await
        .expect("undelete restores the whole subtree");

    crate::logic::article::read_article(&context.state, &owner, &article_id)
        .await
        .expect("owner reads again");
    crate::logic::version::read_version(&context.state, &owner, &version_id, None)
        .await
        .expect("owner reads the version again");
}
