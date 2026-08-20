use super::context::{TestCtx, unique_pdf};
use crate::logic::error::LogicError;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::role::{ROLE_MEMBER, hold_role};
use crate::repository::version::VersionDraft;

async fn member(context: &TestCtx, email: &str) -> String {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::hash(email.as_bytes()).expect("hash must succeed"),
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
        &nail_common::hash::hash("user-zero@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("lookup user zero")
    .expect("seeded user zero")
}

async fn plain(context: &TestCtx, email: &str) -> String {
    crate::repository::user::create_user(
        &context.state.database,
        &nail_common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user")
}

async fn article_fixture(context: &TestCtx, author_id: &str, title: &str) -> (String, String) {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &context.state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.to_string(),
            summary: "summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: nail_common::hash::pdf(&unique_pdf(title)),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    (article_id, version_id)
}

#[test]
fn validate_version_canonicalizes_valid_semver() {
    assert_eq!(
        crate::logic::version::validate_version("1.0.0").expect("valid"),
        "1.0.0"
    );
    assert_eq!(
        crate::logic::version::validate_version(" 1.2.3 ").expect("valid"),
        "1.2.3"
    );
    assert!(matches!(
        crate::logic::version::validate_version("not-semver"),
        Err(LogicError::BadRequest(_))
    ));
}

#[tokio::test]
async fn create_version_requires_a_strictly_greater_semver() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = article_fixture(&context, &actor, "Article").await;

    let error = crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "1.0.0",
        "note",
        context.upload(&unique_pdf("version-next")),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("new version must be strictly greater than the latest version")
    );
}

#[tokio::test]
async fn create_version_writes_a_new_version() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = article_fixture(&context, &actor, "Article").await;

    let version_id = crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "1.1.0",
        "next note",
        context.upload(&unique_pdf("version-1.1.0")),
    )
    .await
    .expect("create version");

    let entry = crate::repository::version::read_version(&context.state.database, &version_id)
        .await
        .expect("read")
        .expect("entry");
    assert_eq!(entry.version_number, "1.1.0");
    assert_eq!(entry.note, "next note");
}

#[tokio::test]
async fn read_version_cross_checks_the_parent_article() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, version_id) = article_fixture(&context, &actor, "Article").await;
    let (other_article, _) = article_fixture(&context, &actor, "Other").await;

    let data =
        crate::logic::version::read_version(&context.state, &actor, &version_id, Some(&article_id))
            .await
            .expect("read");
    assert_eq!(data.version, "1.0.0");

    let error = crate::logic::version::read_version(
        &context.state,
        &actor,
        &version_id,
        Some(&other_article),
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::not_found("version not found"));
}

#[tokio::test]
async fn read_version_and_read_versions_deny_a_user_without_the_grant() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, version_id) = article_fixture(&context, &actor, "Article").await;
    let outsider = plain(&context, "stranger@example.com").await;

    let error = crate::logic::version::read_version(&context.state, &outsider, &version_id, None)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));

    let error = crate::logic::version::read_versions(&context.state, &outsider, &article_id, 1, 10)
        .await
        .unwrap_err();
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn delete_version_hard_removes_the_version_as_admin() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (article_id, version_id) = article_fixture(&context, &actor, "Article").await;

    let data = crate::logic::version::delete_version(
        &context.state,
        &admin_id,
        &version_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect("delete");
    assert_eq!(data.version_id, version_id);
    assert!(
        crate::repository::version::read_version(&context.state.database, &version_id)
            .await
            .expect("read")
            .is_none()
    );
    let _ = article_id;
}

#[tokio::test]
async fn delete_version_hard_is_forbidden_for_a_member_owner() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, version_id) = article_fixture(&context, &actor, "Article").await;

    let error = crate::logic::version::delete_version(
        &context.state,
        &actor,
        &version_id,
        Some(nail_common::request::DeleteMode::Hard),
    )
    .await
    .expect_err("member cannot hard delete");
    assert!(matches!(error, LogicError::Forbidden(_)));
}

#[tokio::test]
async fn delete_version_rejects_transfer_mode() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, version_id) = article_fixture(&context, &actor, "Article").await;
    let error = crate::logic::version::delete_version(
        &context.state,
        &actor,
        &version_id,
        Some(nail_common::request::DeleteMode::Transfer),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("version delete only supports mode \"soft\" or \"hard\"")
    );
}

#[tokio::test]
async fn delete_version_soft_hides_the_version_as_admin() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (article_id, version_id) = article_fixture(&context, &actor, "Article").await;

    let data = crate::logic::version::delete_version(
        &context.state,
        &admin_id,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");
    assert_eq!(data.version_id, version_id);
    assert_eq!(
        crate::logic::version::read_version(&context.state, &actor, &version_id, None)
            .await
            .expect_err("soft-deleted version hidden from a member"),
        LogicError::not_found("version not found")
    );
    assert!(
        crate::logic::version::read_version(&context.state, &admin_id, &version_id, None)
            .await
            .expect("admin holds Undelete::Soft")
            .version
            .as_str()
            == "1.0.0",
        "soft-deleted version visible to the admin via Undelete::Soft"
    );
    let (versions, _) =
        crate::repository::version::versions_of(&context.state.database, &article_id, 10, 0)
            .await
            .expect("versions");
    assert_eq!(
        versions.len(),
        0,
        "soft-deleted version hidden from versions_of"
    );
}

#[tokio::test]
async fn delete_version_soft_hides_only_the_target_version() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (article_id, version_id) = article_fixture(&context, &actor, "Article").await;
    let second_id = {
        let draft = crate::repository::version::VersionDraft {
            version_id: uuid::Uuid::now_v7().to_string(),
            version_number: "2.0.0".to_string(),
            content_hash: nail_common::hash::pdf(&unique_pdf("second")),
            note: "note".to_string(),
        };
        crate::repository::version::create_version(&context.state.database, &article_id, &draft)
            .await
            .expect("second version");
        draft.version_id
    };

    crate::logic::version::delete_version(
        &context.state,
        &admin_id,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete first");

    assert!(
        crate::repository::version::read_version(&context.state.database, &second_id)
            .await
            .expect("read")
            .is_some(),
        "sibling version stays live"
    );
    let latest = crate::repository::article::read_article(&context.state.database, &article_id)
        .await
        .expect("read")
        .expect("article")
        .latest_version_id;
    assert_eq!(
        latest.as_str(),
        second_id.as_str(),
        "latest skips the soft-deleted version"
    );
}

#[tokio::test]
async fn delete_version_soft_is_allowed_for_the_member_owner() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, version_id) = article_fixture(&context, &actor, "Article").await;

    let data = crate::logic::version::delete_version(
        &context.state,
        &actor,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("member owner soft deletes via owner bypass");
    assert_eq!(data.version_id, version_id);
    assert_eq!(
        crate::logic::version::read_version(&context.state, &actor, &version_id, None)
            .await
            .expect_err("soft-deleted version hidden even from its owner"),
        LogicError::not_found("version not found")
    );
}

#[tokio::test]
async fn delete_version_soft_is_forbidden_for_a_stranger_member() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let stranger = member(&context, "bob@example.com").await;
    let (_, version_id) = article_fixture(&context, &actor, "Article").await;

    let error = crate::logic::version::delete_version(
        &context.state,
        &stranger,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect_err("stranger cannot soft delete a version");
    assert!(matches!(error, LogicError::Forbidden(_)));
    assert!(
        crate::repository::version::read_version(&context.state.database, &version_id)
            .await
            .expect("read")
            .is_some(),
        "version untouched"
    );
}

#[tokio::test]
async fn delete_version_soft_keeps_the_content_hash_held() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (_, version_id) = article_fixture(&context, &actor, "Article").await;

    crate::logic::version::delete_version(
        &context.state,
        &admin_id,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    let second_article = article_fixture(&context, &actor, "Other Article").await.0;
    let error = crate::logic::version::create_version(
        &context.state,
        &actor,
        &second_article,
        "1.0.1",
        "note",
        context.upload(&unique_pdf("Article")),
    )
    .await
    .unwrap_err();
    let message = error.to_string();
    assert!(
        message.starts_with("identical PDF already exists"),
        "soft-deleted version still holds its content hash: {message}"
    );
}

#[tokio::test]
async fn delete_version_soft_is_rejected_for_an_already_hidden_version() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (_, version_id) = article_fixture(&context, &actor, "Article").await;

    crate::logic::version::delete_version(
        &context.state,
        &admin_id,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("first soft delete");

    let error = crate::logic::version::delete_version(
        &context.state,
        &admin_id,
        &version_id,
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
async fn undelete_soft_version_revives_the_version_as_admin() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (_, version_id) = article_fixture(&context, &actor, "Restorable").await;

    crate::logic::version::delete_version(
        &context.state,
        &admin_id,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    let data = crate::logic::version::undelete_soft_version(&context.state, &admin_id, &version_id)
        .await
        .expect("undelete");
    assert_eq!(data.version_id, version_id);

    assert!(
        crate::repository::version::read_version(&context.state.database, &version_id)
            .await
            .expect("read")
            .is_some(),
        "version visible again after undelete"
    );
}

#[tokio::test]
async fn undelete_soft_version_is_forbidden_for_a_member() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (_, version_id) = article_fixture(&context, &actor, "Restore Denied").await;

    crate::logic::version::delete_version(
        &context.state,
        &admin_id,
        &version_id,
        Some(nail_common::request::DeleteMode::Soft),
    )
    .await
    .expect("soft delete");

    let error = crate::logic::version::undelete_soft_version(&context.state, &actor, &version_id)
        .await
        .expect_err("member undelete");
    assert_eq!(error, LogicError::forbidden("you are denied"));
}

#[tokio::test]
async fn create_version_rejects_an_invalid_version_number() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = article_fixture(&context, &actor, "Bad Version").await;

    let error = crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "not-a-version",
        "note",
        context.upload(&unique_pdf("bad-version")),
    )
    .await
    .unwrap_err();
    assert_eq!(error, LogicError::bad_request("invalid version number"));
}

#[tokio::test]
async fn create_version_rejects_a_duplicate_content_hash() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = article_fixture(&context, &actor, "Dup Content").await;
    let pdf = unique_pdf("shared");

    let _ = crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "2.0.0",
        "note",
        context.upload(&pdf),
    )
    .await
    .expect("first version");

    let error = crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "2.1.0",
        "note",
        context.upload(&pdf),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("identical PDF already exists (version 2.0.0)")
    );
}

#[tokio::test]
async fn reject_duplicate_content_hash_accepts_an_unknown_hash() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (_, _) = article_fixture(&context, &actor, "Unknown Hash").await;
    crate::logic::version::reject_duplicate_content_hash(
        &context.state,
        &nail_common::hash::pdf(&unique_pdf("unseen")),
    )
    .await
    .expect("unknown hash must pass");
}

#[tokio::test]
async fn reject_duplicate_content_hash_reports_the_owning_version() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let (article_id, _) = article_fixture(&context, &actor, "Owner Article").await;
    let pdf = unique_pdf("shared-hash");
    let hash = nail_common::hash::pdf(&pdf);
    crate::logic::version::create_version(
        &context.state,
        &actor,
        &article_id,
        "2.0.0",
        "note",
        context.upload(&pdf),
    )
    .await
    .expect("owning version");

    let error = crate::logic::version::reject_duplicate_content_hash(&context.state, &hash)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        LogicError::bad_request("identical PDF already exists (version 2.0.0)")
    );
}

#[tokio::test]
async fn undelete_soft_version_rejects_a_version_that_is_not_soft_deleted() {
    let context = TestCtx::new().await.expect("test context");
    let actor = member(&context, "alice@example.com").await;
    let admin_id = admin(&context).await;
    let (_, version_id) = article_fixture(&context, &actor, "Visible Version").await;

    let error =
        crate::logic::version::undelete_soft_version(&context.state, &admin_id, &version_id)
            .await
            .expect_err("undelete visible version");
    assert_eq!(error, LogicError::bad_request("not soft-deleted"));
}
