use super::support::{comment_doc, fresh_index, version_doc};
use crate::field::SearchField;
use crate::outcome::DocHit;
use crate::read::SearchRequest;

#[tokio::test]
async fn missing_query_returns_empty_outcome() {
    let (index, _path) = fresh_index("read_no_query").await;
    index
        .replace_article("a-1", vec![version_doc("a-1", "v-1", "alpha title")])
        .await
        .unwrap();
    let outcome = index
        .read(SearchRequest {
            query: None,
            fields: vec![SearchField::Title],
            limit: 10,
            ..SearchRequest::default()
        })
        .await
        .unwrap();
    assert!(outcome.hits.is_empty());
    index.close().await;
}

#[tokio::test]
async fn blank_query_returns_empty_outcome() {
    let (index, _path) = fresh_index("read_blank_query").await;
    let outcome = index
        .read(SearchRequest {
            query: Some("   ".to_string()),
            fields: vec![SearchField::Title],
            limit: 10,
            ..SearchRequest::default()
        })
        .await
        .unwrap();
    assert!(outcome.hits.is_empty());
    index.close().await;
}

#[tokio::test]
async fn no_fields_returns_empty_outcome() {
    let (index, _path) = fresh_index("read_no_fields").await;
    let outcome = index
        .read(SearchRequest {
            query: Some("alpha".to_string()),
            fields: Vec::new(),
            limit: 10,
            ..SearchRequest::default()
        })
        .await
        .unwrap();
    assert!(outcome.hits.is_empty());
    index.close().await;
}

#[tokio::test]
async fn title_search_returns_version_hit_with_markup() {
    let (index, _path) = fresh_index("read_title").await;
    index
        .replace_article(
            "a-1",
            vec![
                version_doc("a-1", "v-1", "alpha title"),
                comment_doc("a-1", "c-1", "unrelated comment"),
            ],
        )
        .await
        .unwrap();
    let outcome = index
        .read(SearchRequest {
            query: Some("alpha".to_string()),
            fields: vec![SearchField::Title],
            limit: 10,
            ..SearchRequest::default()
        })
        .await
        .unwrap();
    assert_eq!(outcome.hits.len(), 1, "only the version doc matches title");
    let DocHit::Version(version) = &outcome.hits[0] else {
        panic!("expected version hit");
    };
    assert_eq!(version.article_id, "a-1");
    assert_eq!(version.version_id, "v-1");
    assert!(
        version.title.contains("<mark>"),
        "title must carry highlight markup: {}",
        version.title
    );
    assert!(version.article_hits.is_empty());
    assert!(!version.version_number_hit);
    index.close().await;
}

#[tokio::test]
async fn summary_search_buckets_into_article_hits() {
    let (index, _path) = fresh_index("read_summary").await;
    let mut document = match version_doc("a-1", "v-1", "plain title") {
        crate::SearchDoc::Version(inner) => inner,
        crate::SearchDoc::Comment(_) => unreachable!(),
    };
    document.summary = "a needle hidden here".to_string();
    index
        .replace_article("a-1", vec![crate::SearchDoc::Version(document)])
        .await
        .unwrap();
    let outcome = index
        .read(SearchRequest {
            query: Some("needle".to_string()),
            fields: vec![SearchField::Summary],
            limit: 10,
            ..SearchRequest::default()
        })
        .await
        .unwrap();
    let DocHit::Version(version) = &outcome.hits[0] else {
        panic!("expected version hit");
    };
    assert_eq!(version.article_hits.len(), 1);
    assert_eq!(version.article_hits[0].field, SearchField::Summary);
    assert!(version.article_hits[0].snippet.contains("needle"));
    index.close().await;
}

#[tokio::test]
async fn note_and_version_number_bucket_into_version_level() {
    let (index, _path) = fresh_index("read_note").await;
    let mut document = match version_doc("a-1", "v-7", "plain title") {
        crate::SearchDoc::Version(inner) => inner,
        crate::SearchDoc::Comment(_) => unreachable!(),
    };
    document.note = "draft notes here".to_string();
    document.version_number = "7".to_string();
    index
        .replace_article("a-1", vec![crate::SearchDoc::Version(document)])
        .await
        .unwrap();
    let outcome = index
        .read(SearchRequest {
            query: Some("draft 7".to_string()),
            fields: vec![SearchField::Note, SearchField::VersionNumber],
            limit: 10,
            ..SearchRequest::default()
        })
        .await
        .unwrap();
    let DocHit::Version(version) = &outcome.hits[0] else {
        panic!("expected version hit");
    };
    assert_eq!(version.version_hits.len(), 1);
    assert_eq!(version.version_hits[0].field, SearchField::Note);
    assert!(version.version_number_hit, "version number 7 matched");
    index.close().await;
}

#[tokio::test]
async fn comment_field_discriminates_comment_documents() {
    let (index, _path) = fresh_index("read_comment").await;
    index
        .replace_article(
            "a-1",
            vec![
                version_doc("a-1", "v-1", "alpha title"),
                comment_doc("a-1", "c-1", "typo in section two"),
            ],
        )
        .await
        .unwrap();
    let outcome = index
        .read(SearchRequest {
            query: Some("typo".to_string()),
            fields: vec![SearchField::Comment],
            limit: 10,
            ..SearchRequest::default()
        })
        .await
        .unwrap();
    assert_eq!(outcome.hits.len(), 1);
    let DocHit::Comment(comment) = &outcome.hits[0] else {
        panic!("expected comment hit");
    };
    assert_eq!(comment.comment_id, "c-1");
    assert_eq!(comment.version_id, "v-a-1");
    assert!(comment.content.contains("<mark>"));
    index.close().await;
}

#[tokio::test]
async fn time_window_filters_by_timestamp() {
    let (index, _path) = fresh_index("read_time").await;
    let mut older = match version_doc("a-old", "v-old", "old title") {
        crate::SearchDoc::Version(inner) => inner,
        crate::SearchDoc::Comment(_) => unreachable!(),
    };
    older.ts = 100;
    let mut newer = match version_doc("a-new", "v-new", "new title") {
        crate::SearchDoc::Version(inner) => inner,
        crate::SearchDoc::Comment(_) => unreachable!(),
    };
    newer.ts = 200;
    index
        .replace_articles(vec![
            ("a-old".to_string(), vec![crate::SearchDoc::Version(older)]),
            ("a-new".to_string(), vec![crate::SearchDoc::Version(newer)]),
        ])
        .await
        .unwrap();

    let windowed = index
        .read(SearchRequest {
            query: Some("title".to_string()),
            fields: vec![SearchField::Title],
            from_seconds: Some(150),
            to_seconds: Some(300),
            limit: 10,
            ..SearchRequest::default()
        })
        .await
        .unwrap();
    assert_eq!(windowed.hits.len(), 1, "only the newer doc is in window");
    let DocHit::Version(version) = &windowed.hits[0] else {
        panic!("expected version hit");
    };
    assert_eq!(version.article_id, "a-new");

    let unbounded = index
        .read(SearchRequest {
            query: Some("title".to_string()),
            fields: vec![SearchField::Title],
            limit: 10,
            ..SearchRequest::default()
        })
        .await
        .unwrap();
    assert_eq!(unbounded.hits.len(), 2);
    index.close().await;
}
