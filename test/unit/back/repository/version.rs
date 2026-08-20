use super::context::{build_state, test_config};

use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::version::{
    CreateVersionError, VersionDraft, content_hash_owner, create_version, parent_article_of,
    read_version, update_version, versions_of,
};

fn pdf_hash(seed: u8) -> String {
    format!("{seed:x}").repeat(32)
}

fn draft(number: &str, hash: &str) -> VersionDraft {
    VersionDraft {
        version_id: uuid::Uuid::now_v7().to_string(),
        version_number: number.to_string(),
        content_hash: hash.to_string(),
        note: "note".to_string(),
    }
}

async fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::create_user(&state.database, &nail_common::hash::email(email))
        .await
        .expect("user")
}

async fn create_article_fixture(
    state: &crate::infrastructure::state::AppState,
    author_id: &str,
    title: &str,
    hash: &str,
) -> (String, String) {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.to_string(),
            summary: "a summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: hash.to_string(),
                note: "initial note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    (article_id, version_id)
}

#[tokio::test]
async fn create_version_requires_a_strictly_greater_semver() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;

    let error = create_version(&state.database, &article_id, &draft("1.0.0", &pdf_hash(2)))
        .await
        .expect_err("not greater");
    assert!(matches!(error, CreateVersionError::NotGreater));

    create_version(&state.database, &article_id, &draft("1.1.0", &pdf_hash(2)))
        .await
        .expect("greater version");
}

#[tokio::test]
async fn create_version_rejects_a_duplicate_content_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;

    let error = create_version(&state.database, &article_id, &draft("2.0.0", &pdf_hash(1)))
        .await
        .expect_err("duplicate content hash");
    assert!(matches!(error, CreateVersionError::ContentHashTaken));
}

#[tokio::test]
async fn create_version_rejects_an_invalid_number() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;

    let error = create_version(
        &state.database,
        &article_id,
        &draft("not-semver", &pdf_hash(2)),
    )
    .await
    .expect_err("invalid number");
    assert!(matches!(error, CreateVersionError::InvalidNumber));
}

#[tokio::test]
async fn create_version_updates_latest_version_id() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;
    let newer = uuid::Uuid::now_v7().to_string();

    create_version(
        &state.database,
        &article_id,
        &VersionDraft {
            version_id: newer.clone(),
            version_number: "2.0.0".to_string(),
            content_hash: pdf_hash(2),
            note: "note".to_string(),
        },
    )
    .await
    .expect("create version");

    let (items, has_next) = versions_of(&state.database, &article_id, 10, 0)
        .await
        .expect("versions");
    assert_eq!(items.len(), 2);
    assert!(!has_next);
    assert!(items.iter().any(|item| item.version_number == "1.0.0"));
    assert!(items.iter().any(|item| item.version_number == "2.0.0"));
}

#[tokio::test]
async fn versions_of_is_paginated_in_default_order_and_reports_has_next() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;
    create_version(&state.database, &article_id, &draft("2.0.0", &pdf_hash(2)))
        .await
        .expect("v2");

    let (page, has_next) = versions_of(&state.database, &article_id, 1, 0)
        .await
        .expect("versions");
    assert_eq!(page.len(), 1);
    assert!(has_next, "more versions exist beyond the first page");
}

#[tokio::test]
async fn update_version_changes_the_note() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (_, version_id) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;

    update_version(&state.database, &version_id, "updated note")
        .await
        .expect("update");
    let entry = read_version(&state.database, &version_id)
        .await
        .expect("read")
        .expect("version");
    assert_eq!(entry.note, "updated note");
}

#[tokio::test]
async fn content_hash_owner_returns_the_version_and_article_title() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (_, version_id) = create_article_fixture(&state, &author_id, "Titled", &pdf_hash(9)).await;

    let found = content_hash_owner(&state.database, &pdf_hash(9))
        .await
        .expect("find")
        .expect("found");
    assert_eq!(found.version_id, version_id);
    assert_eq!(found.article_title, "Titled");
}

#[tokio::test]
async fn parent_article_of_returns_the_parent() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) =
        create_article_fixture(&state, &author_id, "Titled", &pdf_hash(9)).await;

    assert_eq!(
        parent_article_of(&state.database, &version_id)
            .await
            .expect("parent"),
        Some(article_id)
    );
    assert_eq!(
        parent_article_of(&state.database, "missing")
            .await
            .expect("parent"),
        None
    );
}
