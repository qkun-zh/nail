use super::context::{build_state, test_config};

use crate::repository::article::{
    CreateArticleError, UpdateArticleError, create_article, list_articles_page, read_article,
    update_article,
};
use crate::repository::version::{
    CreateVersionError, create_version, find_version_by_hash, read_article_versions, read_version,
};

async fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::find_or_create_user(&state.graph, &nail_common::hash::email(email))
        .await
        .expect("user")
}

fn pdf_hash(seed: u8) -> String {
    let mut hash = String::with_capacity(32);
    for _ in 0..32 {
        hash.push_str(&format!("{seed:x}"));
    }
    hash
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
        &state.graph,
        &article_id,
        author_id,
        title,
        "a summary",
        &["#rust".to_string()],
        &version_id,
        "1.0.0",
        hash,
        "initial note",
    )
    .await
    .expect("create article");
    (article_id, version_id)
}

#[tokio::test]
async fn create_article_writes_nodes_and_edges_and_reads_back() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();

    create_article(
        &state.graph,
        &article_id,
        &author_id,
        "My Article",
        "A longer summary.",
        &["#rust".to_string(), "#db".to_string()],
        &version_id,
        "1.0.0",
        &pdf_hash(1),
        "first",
    )
    .await
    .expect("create");

    let detail = read_article(&state.graph, &article_id)
        .await
        .expect("read")
        .expect("article");
    assert_eq!(detail.title, "My Article");
    assert_eq!(detail.author_id, author_id);
    assert_eq!(detail.tags.len(), 2);
    assert!(detail.tags.iter().any(|tag| tag.name == "#rust"));

    let version = read_version(&state.graph, &version_id)
        .await
        .expect("version")
        .expect("version");
    assert_eq!(version.version_number, "1.0.0");
    assert_eq!(version.content_hash, pdf_hash(1));
}

#[tokio::test]
async fn create_article_rejects_a_duplicate_title() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    create_article_fixture(&state, &author_id, "Duplicated", &pdf_hash(1)).await;

    let error = create_article(
        &state.graph,
        &uuid::Uuid::now_v7().to_string(),
        &author_id,
        "Duplicated",
        "another summary",
        &["#go".to_string()],
        &uuid::Uuid::now_v7().to_string(),
        "1.0.0",
        &pdf_hash(2),
        "note",
    )
    .await
    .expect_err("duplicate title");
    assert!(matches!(error, CreateArticleError::TitleAlreadyExists));
}

#[tokio::test]
async fn create_article_rejects_a_duplicate_content_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    create_article_fixture(&state, &author_id, "First", &pdf_hash(3)).await;

    let error = create_article(
        &state.graph,
        &uuid::Uuid::now_v7().to_string(),
        &author_id,
        "Second",
        "another summary",
        &["#go".to_string()],
        &uuid::Uuid::now_v7().to_string(),
        "1.0.0",
        &pdf_hash(3),
        "note",
    )
    .await
    .expect_err("duplicate content hash");
    assert!(matches!(error, CreateArticleError::ContentHashExists));
}

#[tokio::test]
async fn create_version_requires_a_strictly_greater_semver() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;

    let error = create_version(
        &state.graph,
        &article_id,
        &uuid::Uuid::now_v7().to_string(),
        "1.0.0",
        &pdf_hash(2),
        "note",
    )
    .await
    .expect_err("not greater");
    assert!(matches!(error, CreateVersionError::VersionNotGreater));

    create_version(
        &state.graph,
        &article_id,
        &uuid::Uuid::now_v7().to_string(),
        "1.1.0",
        &pdf_hash(2),
        "note",
    )
    .await
    .expect("greater version");
}

#[tokio::test]
async fn create_version_rejects_a_duplicate_content_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;

    let error = create_version(
        &state.graph,
        &article_id,
        &uuid::Uuid::now_v7().to_string(),
        "2.0.0",
        &pdf_hash(1),
        "note",
    )
    .await
    .expect_err("duplicate content hash");
    assert!(matches!(error, CreateVersionError::ContentHashExists));
}

#[tokio::test]
async fn create_version_updates_latest_version_id() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;
    let newer = uuid::Uuid::now_v7().to_string();

    create_version(&state.graph, &article_id, &newer, "2.0.0", &pdf_hash(2), "note")
        .await
        .expect("create version");

    let (items, total) = read_article_versions(&state.graph, &article_id, 10, 0)
        .await
        .expect("versions");
    assert_eq!(total, 2);
    assert_eq!(items[0].id, newer);
    assert_eq!(items[0].version_number, "2.0.0");
}

#[tokio::test]
async fn read_article_versions_is_newest_first_and_paginated() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;
    create_version(&state.graph, &article_id, &uuid::Uuid::now_v7().to_string(), "2.0.0", &pdf_hash(2), "n")
        .await
        .expect("v2");

    let (page, total) = read_article_versions(&state.graph, &article_id, 1, 0)
        .await
        .expect("versions");
    assert_eq!(total, 2);
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].version_number, "2.0.0");
}

#[tokio::test]
async fn update_article_changes_fields_and_reconciles_tags() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;

    update_article(
        &state.graph,
        &article_id,
        "Renamed",
        "new summary",
        &["#go".to_string()],
    )
    .await
    .expect("update");

    let detail = read_article(&state.graph, &article_id)
        .await
        .expect("read")
        .expect("article");
    assert_eq!(detail.title, "Renamed");
    assert_eq!(detail.summary, "new summary");
    assert_eq!(detail.tags.len(), 1);
    assert_eq!(detail.tags[0].name, "#go");
}

#[tokio::test]
async fn update_article_rejects_a_duplicate_title() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "First", &pdf_hash(1)).await;
    create_article_fixture(&state, &author_id, "Second", &pdf_hash(2)).await;

    let error = update_article(&state.graph, &article_id, "Second", "summary", &["#go".to_string()])
        .await
        .expect_err("duplicate title");
    assert!(matches!(error, UpdateArticleError::TitleAlreadyExists));
}

#[tokio::test]
async fn update_article_returns_not_found_for_a_missing_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let error = update_article(&state.graph, "missing", "Title", "summary", &["#go".to_string()])
        .await
        .expect_err("missing");
    assert!(matches!(error, UpdateArticleError::NotFound));
}

#[tokio::test]
async fn find_version_by_hash_returns_the_version_and_article_title() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (_, version_id) = create_article_fixture(&state, &author_id, "Titled", &pdf_hash(9)).await;

    let found = find_version_by_hash(&state.graph, &pdf_hash(9))
        .await
        .expect("find")
        .expect("found");
    assert_eq!(found.0, version_id);
    assert_eq!(found.1, "Titled");
}

#[tokio::test]
async fn list_articles_page_returns_id_desc_with_enrichment() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    create_article_fixture(&state, &author_id, "First", &pdf_hash(1)).await;
    create_article_fixture(&state, &author_id, "Second", &pdf_hash(2)).await;

    let (items, total) = list_articles_page(&state.graph, 10, 0).await.expect("list");
    assert_eq!(total, 2);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].title, "Second");
    assert_eq!(items[0].author_id, author_id);
    assert_eq!(items[0].latest_version, "1.0.0");
    assert!(!items[0].latest_version_id.is_empty());
    assert_eq!(items[0].tags.len(), 1);
    assert_eq!(items[1].title, "First");
}
