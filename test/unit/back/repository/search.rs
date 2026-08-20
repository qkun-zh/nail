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
        &state.database,
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
        &state.database,
        &nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");
    let article_id = create_article_fixture(&state, &author_id, "A Unique Title").await;

    index
        .sync(&state.database, &article_id)
        .await
        .expect("sync");

    let outcome = index
        .read(
            &state.database,
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
async fn sync_all_and_sync_user_skip_soft_deleted_articles() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.database,
        &nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let index = state.searcher.clone();

    let first = create_fixture_with_hash(&state, &author_id, "Soft Del First", &pdf_hash(4)).await;
    let second =
        create_fixture_with_hash(&state, &author_id, "Soft Del Second", &pdf_hash(5)).await;
    index
        .sync(&state.database, &first)
        .await
        .expect("sync first");
    index
        .sync(&state.database, &second)
        .await
        .expect("sync second");

    crate::repository::delete::soft_delete_article(&state.database, &first)
        .await
        .expect("soft delete");

    let synced_all = index.sync_all(&state.database).await.expect("sync all");
    assert_eq!(synced_all, 1, "deleted article excluded from sync_all");

    let synced_user = index
        .sync_user(&state.database, &author_id)
        .await
        .expect("sync user");
    assert_eq!(synced_user, 1, "deleted article excluded from sync_user");

    let rebuilt = version_articles(
        &index
            .read(
                &state.database,
                query_request("soft", vec![nail_common::search::SearchRange::Title]),
            )
            .await
            .expect("read"),
    )
    .len();
    assert_eq!(rebuilt, 1, "only the live article remains indexed");

    index.close().await;
}

#[tokio::test]
async fn keyword_read_returns_highlighted_hits() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.database,
        &nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");
    let article_id = create_article_fixture(&state, &author_id, "Rust Programming").await;
    index
        .sync(&state.database, &article_id)
        .await
        .expect("sync");

    let outcome = index
        .read(
            &state.database,
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
        &state.database,
        &nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");
    let article_id = create_article_fixture(&state, &author_id, "Article").await;
    index
        .sync(&state.database, &article_id)
        .await
        .expect("sync");

    crate::repository::user::update_user_name(&state.database, &author_id, "renamed-author")
        .await
        .expect("rename");

    let synced = index
        .sync_user(&state.database, &author_id)
        .await
        .expect("sync user");
    assert_eq!(synced, 1);

    let outcome = index
        .read(
            &state.database,
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
        &state.database,
        &nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let commenter_id = crate::repository::user::create_user(
        &state.database,
        &nail_common::hash::hash("bob@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    crate::repository::user::update_user_name(&state.database, &commenter_id, "old-name")
        .await
        .expect("name");

    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");

    let version_id = uuid::Uuid::now_v7().to_string();
    let article_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.database,
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
        &state.database,
        &uuid::Uuid::now_v7().to_string(),
        &commenter_id,
        &version_id,
        "hello from bob",
    )
    .await
    .expect("comment");
    index
        .sync(&state.database, &article_id)
        .await
        .expect("sync");

    crate::repository::user::update_user_name(&state.database, &commenter_id, "new-name")
        .await
        .expect("rename");

    let synced = index
        .sync_user(&state.database, &commenter_id)
        .await
        .expect("sync user");
    assert_eq!(synced, 1, "commenter's article must be re-synced");

    let outcome = index
        .read(
            &state.database,
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
        &state.database,
        &nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");
    let article_id = create_article_fixture(&state, &author_id, "Article").await;
    index
        .sync(&state.database, &article_id)
        .await
        .expect("sync");

    crate::repository::delete::delete_article(&state.database, &article_id)
        .await
        .expect("delete");
    index
        .sync(&state.database, &article_id)
        .await
        .expect("sync after delete");

    let outcome = index
        .read(
            &state.database,
            query_request("article", vec![nail_common::search::SearchRange::Title]),
        )
        .await
        .expect("read");
    assert!(version_articles(&outcome).is_empty());

    index.close().await;
    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn sync_excludes_a_soft_deleted_version_doc_and_its_comments() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.database,
        &nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");

    let version_id = uuid::Uuid::now_v7().to_string();
    let article_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.clone(),
            title: "Soft Version Title".to_string(),
            summary: "a searchable summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: pdf_hash(6),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create");
    let comment_id = uuid::Uuid::now_v7().to_string();
    crate::repository::comment::create_top_level_comment(
        &state.database,
        &comment_id,
        &author_id,
        &version_id,
        "public comment on the soft-deleted version",
    )
    .await
    .expect("comment");
    index
        .sync(&state.database, &article_id)
        .await
        .expect("sync");

    crate::repository::delete::soft_delete_version(&state.database, &version_id)
        .await
        .expect("soft delete version");
    index
        .sync(&state.database, &article_id)
        .await
        .expect("resync after soft delete");

    let outcome = index
        .read(
            &state.database,
            query_request("soft", vec![nail_common::search::SearchRange::Title]),
        )
        .await
        .expect("read");
    let versions = version_articles(&outcome);
    assert!(
        versions.is_empty(),
        "soft-deleted version doc removed from search"
    );
    let comments = index
        .read(
            &state.database,
            SearchRequest {
                query: Some("public comment".to_string()),
                ranges: vec![nail_common::search::SearchRange::Comment],
                ..empty_request(10)
            },
        )
        .await
        .expect("read")
        .docs
        .into_iter()
        .filter_map(|doc| match doc {
            SearchDocOutcome::Comment(comment) => Some(comment.comment_id),
            SearchDocOutcome::Version(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        comments,
        Vec::<String>::new(),
        "comments hidden with their version"
    );

    index.close().await;
    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn sync_excludes_a_soft_deleted_comment_doc() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.database,
        &nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");

    let version_id = uuid::Uuid::now_v7().to_string();
    let article_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.clone(),
            title: "Soft Comment Title".to_string(),
            summary: "a searchable summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: pdf_hash(7),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create");
    let deleted_comment = uuid::Uuid::now_v7().to_string();
    crate::repository::comment::create_top_level_comment(
        &state.database,
        &deleted_comment,
        &author_id,
        &version_id,
        "doomed comment text",
    )
    .await
    .expect("comment");
    let live_comment = uuid::Uuid::now_v7().to_string();
    crate::repository::comment::create_top_level_comment(
        &state.database,
        &live_comment,
        &author_id,
        &version_id,
        "live comment text",
    )
    .await
    .expect("comment");
    index
        .sync(&state.database, &article_id)
        .await
        .expect("sync");

    crate::repository::delete::soft_delete_comment(&state.database, &deleted_comment)
        .await
        .expect("soft delete comment");
    index
        .sync(&state.database, &article_id)
        .await
        .expect("resync after soft delete");

    let comments = index
        .read(
            &state.database,
            SearchRequest {
                query: Some("comment text".to_string()),
                ranges: vec![nail_common::search::SearchRange::Comment],
                ..empty_request(10)
            },
        )
        .await
        .expect("read")
        .docs
        .into_iter()
        .filter_map(|doc| match doc {
            SearchDocOutcome::Comment(comment) => Some(comment.comment_id),
            SearchDocOutcome::Version(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        comments,
        vec![live_comment.clone()],
        "only the live comment remains indexed"
    );

    index.close().await;
    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn sync_drops_all_docs_of_a_soft_deleted_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.database,
        &nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let directory = std::env::temp_dir().join(format!("nail_search_{}", uuid::Uuid::now_v7()));
    let index = SearchIndex::open_or_create(directory.to_str().expect("path"))
        .await
        .expect("index");

    let version_id = uuid::Uuid::now_v7().to_string();
    let article_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.clone(),
            title: "Soft Article Title".to_string(),
            summary: "a searchable summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: pdf_hash(8),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create");
    index
        .sync(&state.database, &article_id)
        .await
        .expect("sync");

    crate::repository::delete::soft_delete_article(&state.database, &article_id)
        .await
        .expect("soft delete article");
    index
        .sync(&state.database, &article_id)
        .await
        .expect("resync after soft delete");

    let outcome = index
        .read(
            &state.database,
            query_request(
                "soft article",
                vec![nail_common::search::SearchRange::Title],
            ),
        )
        .await
        .expect("read");
    let versions = version_articles(&outcome);
    assert!(
        versions.is_empty(),
        "soft-deleted article keeps no docs (subtree hidden)"
    );

    index.close().await;
    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn sync_all_and_incremental_sync_agree_on_document_count() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.database,
        &nail_common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .await
    .expect("user");
    let index = state.searcher.clone();

    let first = create_fixture_with_hash(&state, &author_id, "First Article", &pdf_hash(2)).await;
    let second = create_fixture_with_hash(&state, &author_id, "Second Article", &pdf_hash(3)).await;
    index
        .sync(&state.database, &first)
        .await
        .expect("sync first");
    index
        .sync(&state.database, &second)
        .await
        .expect("sync second");
    let incremental_versions = version_articles(
        &index
            .read(
                &state.database,
                query_request("article", vec![nail_common::search::SearchRange::Title]),
            )
            .await
            .expect("read"),
    )
    .len();
    assert_eq!(incremental_versions, 2);

    let rebuilt = index.sync_all(&state.database).await.expect("sync all");
    assert_eq!(rebuilt, 2, "one version document per article");
    let after_rebuild_versions = version_articles(
        &index
            .read(
                &state.database,
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

    crate::repository::delete::delete_article(&state.database, &first)
        .await
        .expect("delete");
    index
        .sync(&state.database, &first)
        .await
        .expect("sync after delete");
    let after_delete_versions = version_articles(
        &index
            .read(
                &state.database,
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
        marker_version, "5",
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
        &state.database,
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
