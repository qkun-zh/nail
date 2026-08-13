use super::context::{build_state, test_config};

use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::search::{SearchIndex, SearchRequest};
use crate::repository::version::VersionDraft;

fn pdf_hash(seed: u8) -> String {
    (0..32).map(|_| format!("{seed:x}")).collect()
}

async fn create_article_fixture(
    state: &crate::infrastructure::state::AppState,
    author_id: &str,
    title: &str,
) -> String {
    let article_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.to_string(),
            summary: "a searchable summary".to_string(),
            tags: vec!["#rust".to_string()],
            first_version: VersionDraft {
                version_id: uuid::Uuid::now_v7().to_string(),
                version_number: "1.0.0".to_string(),
                content_hash: pdf_hash(1),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create");
    article_id
}

fn empty_request(limit: u64) -> SearchRequest {
    SearchRequest {
        query: None,
        ranges: vec![nail_common::search::SearchRange::Title],
        sort: Vec::new(),
        from_seconds: None,
        to_seconds: None,
        offset: 0,
        limit,
    }
}

#[tokio::test]
async fn sync_and_read_round_trips_an_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");
    let article_id = create_article_fixture(&state, &author_id, "A Unique Title").await;

    index.sync(&state.graph, &article_id).await.expect("sync");

    let outcome = index.read(empty_request(10)).await.expect("read");
    assert_eq!(outcome.total, 1);
    assert_eq!(outcome.articles.len(), 1);
    assert_eq!(outcome.articles[0].id, article_id);
    assert_eq!(outcome.articles[0].title, "A Unique Title");

    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn keyword_read_returns_highlighted_hits() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");
    let article_id = create_article_fixture(&state, &author_id, "Rust Programming").await;
    index.sync(&state.graph, &article_id).await.expect("sync");

    let outcome = index
        .read(SearchRequest {
            query: Some("rust".to_string()),
            ranges: vec![nail_common::search::SearchRange::Title],
            ..empty_request(10)
        })
        .await
        .expect("read");
    assert_eq!(outcome.total, 1);
    assert!(
        outcome.articles[0]
            .hits
            .iter()
            .any(|hit| hit.range == nail_common::search::SearchRange::Title
                && hit.snippet.contains("<mark>"))
    );

    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn sync_user_refreshes_the_author_name() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");
    let article_id = create_article_fixture(&state, &author_id, "Article").await;
    index.sync(&state.graph, &article_id).await.expect("sync");

    crate::repository::user::update_user_name(&state.graph, &author_id, "renamed-author")
        .await
        .expect("rename");

    let synced = index.sync_user(&state.graph, &author_id).await.expect("sync user");
    assert_eq!(synced, 1);

    let outcome = index.read(empty_request(10)).await.expect("read");
    assert_eq!(outcome.articles[0].author, "renamed-author");

    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn sync_removes_a_document_for_a_deleted_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");
    let article_id = create_article_fixture(&state, &author_id, "Article").await;
    index.sync(&state.graph, &article_id).await.expect("sync");

    crate::repository::delete::delete_article(&state.graph, &article_id)
        .await
        .expect("delete");
    index.sync(&state.graph, &article_id).await.expect("sync after delete");

    let outcome = index.read(empty_request(10)).await.expect("read");
    assert_eq!(outcome.total, 0);

    let _ = std::fs::remove_dir_all(&directory);
}
