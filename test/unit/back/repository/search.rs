use super::context::{build_state, test_config};

use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::search::{SearchDocOutcome, SearchIndex, SearchRequest};
use crate::repository::version::VersionDraft;

fn pdf_hash(seed: u8) -> String {
    format!("{seed:x}").repeat(32)
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
            tags: vec!["rust".to_string()],
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

fn query_request(query: &str, ranges: Vec<nail_common::search::SearchRange>) -> SearchRequest {
    SearchRequest {
        query: Some(query.to_string()),
        ranges,
        sort: Vec::new(),
        from_seconds: None,
        to_seconds: None,
        offset: 0,
        limit: 10,
    }
}

fn version_articles(
    outcome: &crate::repository::search::SearchOutcome,
) -> Vec<&crate::repository::search::SearchVersionOutcome> {
    outcome
        .docs
        .iter()
        .filter_map(|doc| match doc {
            SearchDocOutcome::Version(version) => Some(version),
            SearchDocOutcome::Comment(_) => None,
        })
        .collect()
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

    let outcome = index
        .read(
            &state.graph,
            query_request("unique", vec![nail_common::search::SearchRange::Title]),
        )
        .await
        .expect("read");
    let versions = version_articles(&outcome);
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].article_id, article_id);
    assert!(versions[0].title.contains("Unique"));

    index.close().await;
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
        .read(
            &state.graph,
            SearchRequest {
                query: Some("rust".to_string()),
                ranges: vec![nail_common::search::SearchRange::Title],
                ..empty_request(10)
            },
        )
        .await
        .expect("read");
    let versions = version_articles(&outcome);
    assert_eq!(versions.len(), 1);
    assert!(
        versions[0].title.contains("<mark>"),
        "title should be highlighted: {}",
        versions[0].title
    );

    index.close().await;
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

    let synced = index
        .sync_user(&state.graph, &author_id)
        .await
        .expect("sync user");
    assert_eq!(synced, 1);

    let outcome = index
        .read(
            &state.graph,
            query_request("article", vec![nail_common::search::SearchRange::Title]),
        )
        .await
        .expect("read");
    assert_eq!(version_articles(&outcome)[0].author_name, "renamed-author");

    index.close().await;
    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn sync_user_refreshes_the_author_name_of_their_comments() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    let commenter_id = crate::repository::user::create_user(
        &state.graph,
        &nail_common::hash::email("bob@example.com"),
    )
    .await
    .expect("user");
    crate::repository::user::update_user_name(&state.graph, &commenter_id, "old-name")
        .await
        .expect("name");

    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");

    let version_id = uuid::Uuid::now_v7().to_string();
    let article_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.clone(),
            title: "Article".to_string(),
            summary: "a searchable summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: pdf_hash(1),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create");
    crate::repository::comment::create_top_level_comment(
        &state.graph,
        &uuid::Uuid::now_v7().to_string(),
        &commenter_id,
        &version_id,
        "hello from bob",
    )
    .await
    .expect("comment");
    index.sync(&state.graph, &article_id).await.expect("sync");

    crate::repository::user::update_user_name(&state.graph, &commenter_id, "new-name")
        .await
        .expect("rename");

    let synced = index
        .sync_user(&state.graph, &commenter_id)
        .await
        .expect("sync user");
    assert_eq!(synced, 1, "commenter's article must be re-synced");

    let outcome = index
        .read(
            &state.graph,
            SearchRequest {
                query: Some("hello".to_string()),
                ranges: vec![nail_common::search::SearchRange::Comment],
                ..empty_request(10)
            },
        )
        .await
        .expect("read");
    let comments: Vec<_> = outcome
        .docs
        .iter()
        .filter_map(|doc| match doc {
            SearchDocOutcome::Comment(comment) => Some(comment),
            SearchDocOutcome::Version(_) => None,
        })
        .collect();
    assert_eq!(comments.len(), 1);
    assert_eq!(
        comments[0].author_name, "new-name",
        "renaming must refresh the comment author name in the index"
    );

    index.close().await;
    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn sync_removes_documents_for_a_deleted_article() {
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
    index
        .sync(&state.graph, &article_id)
        .await
        .expect("sync after delete");

    let outcome = index
        .read(
            &state.graph,
            query_request("article", vec![nail_common::search::SearchRange::Title]),
        )
        .await
        .expect("read");
    assert!(version_articles(&outcome).is_empty());

    index.close().await;
    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn sync_all_and_incremental_sync_agree_on_document_count() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    let index = state.search.clone();

    let first = create_fixture_with_hash(&state, &author_id, "First Article", &pdf_hash(2)).await;
    let second = create_fixture_with_hash(&state, &author_id, "Second Article", &pdf_hash(3)).await;
    index.sync(&state.graph, &first).await.expect("sync first");
    index
        .sync(&state.graph, &second)
        .await
        .expect("sync second");
    let incremental_versions = version_articles(
        &index
            .read(
                &state.graph,
                query_request("article", vec![nail_common::search::SearchRange::Title]),
            )
            .await
            .expect("read"),
    )
    .len();
    assert_eq!(incremental_versions, 2);

    let rebuilt = index.sync_all(&state.graph).await.expect("sync all");
    assert_eq!(rebuilt, 2, "one version document per article");
    let after_rebuild_versions = version_articles(
        &index
            .read(
                &state.graph,
                query_request("article", vec![nail_common::search::SearchRange::Title]),
            )
            .await
            .expect("read"),
    )
    .len();
    assert_eq!(
        after_rebuild_versions, 2,
        "full rebuild must agree with incremental sync"
    );

    crate::repository::delete::delete_article(&state.graph, &first)
        .await
        .expect("delete");
    index
        .sync(&state.graph, &first)
        .await
        .expect("sync after delete");
    let after_delete_versions = version_articles(
        &index
            .read(
                &state.graph,
                query_request("article", vec![nail_common::search::SearchRange::Title]),
            )
            .await
            .expect("read"),
    )
    .len();
    assert_eq!(
        after_delete_versions, 1,
        "incremental delete must agree with the seekstorm count"
    );
    index.close().await;
}

#[tokio::test]
async fn opening_a_stale_schema_recreates_the_index() {
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let marker = directory.join("nail_schema_version");

    let first = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("create");
    assert!(!first.was_recreated(), "fresh create is not a migration");
    first.close().await;

    let marker_version = std::fs::read_to_string(&marker).expect("marker written");
    assert_eq!(
        marker_version, "2",
        "schema marker records the current version"
    );

    std::fs::write(&marker, "1").expect("write stale marker");
    let migrated = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("reopen");
    assert!(
        migrated.was_recreated(),
        "stale marker must trigger a rebuild"
    );
    migrated.close().await;

    let _ = std::fs::remove_dir_all(&directory);
}

async fn create_fixture_with_hash(
    state: &crate::infrastructure::state::AppState,
    author_id: &str,
    title: &str,
    hash: &str,
) -> String {
    let article_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.to_string(),
            summary: "a searchable summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: uuid::Uuid::now_v7().to_string(),
                version_number: "1.0.0".to_string(),
                content_hash: hash.to_string(),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create");
    article_id
}

#[tokio::test]
async fn probe_live_data_58() {
    use crate::repository::search::SearchRequest;
    use nail_common::search::SearchRange;
    let db = crate::repository::graph::open("/tmp/opencode/probe/agdb").expect("open db");
    let index =
        crate::repository::search::SearchIndex::open_or_create("/tmp/opencode/probe/search")
            .await
            .expect("open index");
    let outcome = index
        .read(
            &db,
            SearchRequest {
                query: Some("58".to_string()),
                ranges: vec![
                    SearchRange::Title,
                    SearchRange::Summary,
                    SearchRange::AuthorName,
                    SearchRange::Comment,
                    SearchRange::Note,
                    SearchRange::Tag,
                    SearchRange::VersionNumber,
                ],
                sort: Vec::new(),
                from_seconds: None,
                to_seconds: None,
                offset: 0,
                limit: 10,
            },
        )
        .await
        .expect("search 58");
    println!("docs = {}", outcome.docs.len());
    for doc in &outcome.docs {
        match doc {
            crate::repository::search::SearchDocOutcome::Version(v) => {
                println!(
                    "VERSION article={} title={:?} author={:?} version_number={:?}",
                    v.article_id, v.title, v.author_name, v.version_number
                );
            }
            crate::repository::search::SearchDocOutcome::Comment(c) => {
                println!(
                    "COMMENT article={} title={:?} author={:?} content={:?}",
                    c.article_id, c.article_title, c.article_author_name, c.content
                );
            }
        }
    }
    let _ = db;
    let _ = index;

    let mut config = super::context::test_config();
    config.server.db_path = "/tmp/opencode/probe/agdb".to_string();
    config.server.search_index_path = "/tmp/opencode/probe/search".to_string();
    let graph = crate::repository::graph::open(&config.server.db_path).expect("open db 2");
    let search =
        crate::repository::search::SearchIndex::open_or_create(&config.server.search_index_path)
            .await
            .expect("open index 2");
    let caches = crate::repository::cache::TokenCaches::new(
        std::time::Duration::from_secs(8000),
        std::time::Duration::from_secs(8000),
        std::time::Duration::from_mins(5),
        std::time::Duration::from_mins(1),
        100,
    );
    let state = crate::infrastructure::state::AppState {
        graph,
        search,
        caches,
        config: std::sync::Arc::new(config),
        email: crate::infrastructure::email::RateLimitedSender::new(
            std::sync::Arc::new(super::context::RecordingSender::default()),
            0,
        ),
    };
    let params = nail_common::request::ArticleSearchParams {
        q: Some("58".to_string()),
        ranges: Some("title,summary,author_name,comment,note,tag,version_number".to_string()),
        sort: None,
        from: None,
        to: None,
        page: None,
        limit: None,
    };
    let page = crate::logic::search::search_articles(&state, &params)
        .await
        .expect("logic search");
    println!("logic total = {}", page.total);
    for item in &page.article_list {
        println!(
            "LOGIC article={} title={:?} author={:?} time={:?} hits={:?}",
            item.article_id, item.title, item.author_name, item.time, item.article_hits
        );
    }
}
