use super::context::{build_state, test_config};

use crate::repository::article::create_article;
use crate::repository::search::{
    SearchQuery, open_or_create_index, search_articles, sync_article, sync_articles_of_user,
};

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
        &article_id,
        author_id,
        title,
        "a searchable summary",
        &["#rust".to_string()],
        &uuid::Uuid::now_v7().to_string(),
        "1.0.0",
        &pdf_hash(1),
        "note",
    )
    .await
    .expect("create");
    article_id
}

fn empty_query(limit: u64) -> SearchQuery {
    SearchQuery {
        q: None,
        fields: vec!["title".to_string(), "summary".to_string()],
        from: None,
        to: None,
        sort: Vec::new(),
        offset: 0,
        limit,
    }
}

#[tokio::test]
async fn sync_and_search_round_trips_an_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::find_or_create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = open_or_create_index(directory.to_str().expect("path"))
        .await
        .expect("index");
    let article_id = create_article_fixture(&state, &author_id, "A Unique Title").await;

    sync_article(&index, &state.graph, &article_id)
        .await
        .expect("sync");

    let outcome = search_articles(&index, empty_query(10)).await.expect("search");
    assert_eq!(outcome.total, 1);
    assert_eq!(outcome.docs.len(), 1);
    assert_eq!(outcome.docs[0].id, article_id);
    assert_eq!(outcome.docs[0].title, "A Unique Title");

    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn keyword_search_returns_highlighted_hits() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::find_or_create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = open_or_create_index(directory.to_str().expect("path"))
        .await
        .expect("index");
    let article_id = create_article_fixture(&state, &author_id, "Rust Programming").await;
    sync_article(&index, &state.graph, &article_id).await.expect("sync");

    let query = SearchQuery {
        q: Some("rust".to_string()),
        fields: vec!["title".to_string()],
        from: None,
        to: None,
        sort: Vec::new(),
        offset: 0,
        limit: 10,
    };
    let outcome = search_articles(&index, query).await.expect("search");
    assert_eq!(outcome.total, 1);
    assert!(
        outcome.docs[0]
            .hits
            .iter()
            .any(|(field, snippet)| field == "title" && snippet.contains("<mark>"))
    );

    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn sync_articles_of_user_refreshes_the_author_name() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::find_or_create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = open_or_create_index(directory.to_str().expect("path"))
        .await
        .expect("index");
    let article_id = create_article_fixture(&state, &author_id, "Article").await;
    sync_article(&index, &state.graph, &article_id).await.expect("sync");

    crate::repository::user::update_user_name(&state.graph, &author_id, "renamed-author")
        .await
        .expect("rename");

    let synced = sync_articles_of_user(&index, &state.graph, &author_id)
        .await
        .expect("sync user");
    assert_eq!(synced, 1);

    let outcome = search_articles(&index, empty_query(10)).await.expect("search");
    assert_eq!(outcome.docs[0].author, "renamed-author");

    let _ = std::fs::remove_dir_all(&directory);
}
