use super::context::{TestCtx, unique_pdf, valid_pdf};
use crate::logic::error::LogicError;
use crate::repository::role::{ROLE_MEMBER, hold_role};

async fn member(context: &TestCtx, email: &str) -> String {
    let user_id = crate::repository::user::create_user(
        &context.state.graph,
        &nail_common::hash::email(email),
    )
    .await
    .expect("user");
    hold_role(&context.state.graph, &user_id, ROLE_MEMBER)
        .await
        .expect("member role");
    user_id
}

async fn admin(context: &TestCtx) -> String {
    crate::repository::user::read_user_by_email_address_hash(
        &context.state.graph,
        &nail_common::hash::email("user-zero@example.com"),
    )
    .await
    .expect("lookup user zero")
    .expect("seeded user zero")
}

async fn plain(context: &TestCtx, email: &str) -> String {
    crate::repository::user::create_user(&context.state.graph, &nail_common::hash::email(email))
        .await
        .expect("user")
}

#[tokio::test]
async fn create_article_writes_the_article_and_version() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let upload = context.upload(&valid_pdf());

    let (article_id, version_id) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "My Article",
            summary: "A summary.",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload,
        },
    )
    .await
    .expect("create article");

    assert!(!article_id.is_empty());
    assert!(!version_id.is_empty());
    assert!(
        crate::repository::article::read_article(&context.state.graph, &article_id)
            .await
            .expect("read")
            .is_some()
    );
}

#[tokio::test]
async fn create_article_requires_article_create_permission() {
    let context = TestCtx::new().await.expect("test context");
    let actor = crate::repository::user::create_user(
        &context.state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    let error = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Title",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn create_article_rejects_an_empty_title() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let error = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("text cannot be empty"));
}

#[tokio::test]
async fn create_article_rejects_a_duplicate_title() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let _ = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Duplicated",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&unique_pdf("first")),
        },
    )
    .await
    .expect("first");

    let error = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Duplicated",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&unique_pdf("second")),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("title already exists"));
}

#[tokio::test]
async fn create_article_rejects_a_duplicate_content_hash() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let pdf = valid_pdf();
    let _ = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "First",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&pdf),
        },
    )
    .await
    .expect("first");

    let error = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Second",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&pdf),
        },
    )
    .await
    .unwrap_err();
    let message = error.to_string();
    assert!(
        message.starts_with("identical PDF already exists"),
        "{message}"
    );
}

#[tokio::test]
async fn read_article_returns_detail() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Titled",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    let data = crate::logic::article::read_article(&context.state, &actor, &article_id)
        .await
        .expect("read");
    assert_eq!(data.title, "Titled");
    assert_eq!(data.author_id, actor);
}

#[tokio::test]
async fn read_article_missing_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let reader = admin(&context).await;
    let error = crate::logic::article::read_article(&context.state, &reader, "missing")
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::not_found("article not found"));
}

#[tokio::test]
async fn read_article_denies_a_user_without_the_grant() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Restricted",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");
    let outsider = plain(&context, "stranger@example.com").await;

    let error = crate::logic::article::read_article(&context.state, &outsider, &article_id)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn delete_article_soft_hides_the_article_and_its_versions() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Softly Deleted",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    let data = crate::logic::article::delete_article(
        &context.state,
        &actor,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");
    assert_eq!(data.article_id, article_id);

    let error = crate::logic::article::read_article(&context.state, &actor, &article_id)
        .await
        .expect_err("deleted article");
    assert_eq!(error, LogicError::not_found("article not found"));
    let (versions, _) =
        crate::repository::version::versions_of(&context.state.graph, &article_id, 10, 0)
            .await
            .expect("versions");
    assert_eq!(
        versions.len(),
        0,
        "versions hidden after article soft delete"
    );
}

#[tokio::test]
async fn delete_article_soft_is_forbidden_for_a_stranger() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let stranger = member(&context, "bob@example.com").await;
    let (article_id, _) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Not Yours",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    let error = crate::logic::article::delete_article(
        &context.state,
        &stranger,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect_err("stranger cannot soft delete");
    assert!(matches!(error, LogicError::Forbidden(_)));
    assert!(
        crate::repository::article::read_article(&context.state.graph, &article_id)
            .await
            .expect("read")
            .is_some(),
        "article untouched"
    );
}

#[tokio::test]
async fn delete_article_soft_keeps_the_title_and_content_hash_held() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let pdf = valid_pdf();
    let (article_id, _) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Held Title",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&pdf),
        },
    )
    .await
    .expect("create");

    crate::logic::article::delete_article(
        &context.state,
        &actor,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    let error = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Held Title",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&unique_pdf("reused")),
        },
    )
    .await
    .unwrap_err();
    let message = error.to_string();
    assert!(
        message.starts_with("title already exists"),
        "deleted node still holds its title: {message}"
    );
}

#[tokio::test]
async fn delete_article_soft_is_rejected_for_an_already_hidden_article() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Double Soft",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    crate::logic::article::delete_article(
        &context.state,
        &actor,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("first soft delete");

    let error = crate::logic::article::delete_article(
        &context.state,
        &actor,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect_err("second soft delete");
    assert_eq!(
        error,
        LogicError::bad_request("already soft-deleted"),
        "repeated soft delete is rejected at the logic layer"
    );
}

#[tokio::test]
async fn restore_article_revives_the_article_and_its_versions() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (article_id, _) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Restorable",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    crate::logic::article::delete_article(
        &context.state,
        &actor,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    let data = crate::logic::article::restore_article(&context.state, &admin_id, &article_id)
        .await
        .expect("restore");
    assert_eq!(data.article_id, article_id);

    crate::logic::article::read_article(&context.state, &admin_id, &article_id)
        .await
        .expect("article visible again");
    let (versions, _) =
        crate::repository::version::versions_of(&context.state.graph, &article_id, 10, 0)
            .await
            .expect("versions");
    assert_eq!(
        versions.len(),
        1,
        "versions revived after the article restore"
    );
}

#[tokio::test]
async fn restore_article_is_forbidden_for_a_member() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Restore Denied",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    crate::logic::article::delete_article(
        &context.state,
        &actor,
        &article_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    let error = crate::logic::article::restore_article(&context.state, &actor, &article_id)
        .await
        .expect_err("member restore");
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn restore_article_is_rejected_when_the_article_is_visible() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (article_id, _) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Already Visible",
            summary: "Summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .expect("create");

    let error = crate::logic::article::restore_article(&context.state, &admin_id, &article_id)
        .await
        .expect_err("restore of visible article");
    assert_eq!(
        error,
        LogicError::bad_request("not soft-deleted"),
        "restore of a visible article is rejected"
    );
}

#[tokio::test]
async fn create_article_rejects_an_empty_tag_set() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let error = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "No Tags",
            summary: "Summary",
            tags: "",
            version: "1.0.0",
            note: "note",
            upload: context.upload(&valid_pdf()),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("at least one tag is required")
    );
}

#[tokio::test]
async fn update_article_of_a_missing_article_is_not_found() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let error = crate::logic::article::update_article(
        &context.state,
        &actor,
        "missing-article",
        "New Title",
        "New Summary",
        "rust",
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::not_found("article not found"));
}
