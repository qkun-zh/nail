use std::fs;

use super::support::{comment_doc, fresh_index, version_doc};
use crate::index::SearchIndex;
use crate::{Error, IndexDoc};

#[tokio::test]
async fn fresh_open_is_not_a_recreate_and_reopen_keeps_data() {
    let (index, path) = fresh_index("reopen").await;
    assert!(!index.was_recreated());
    index
        .replace_article("a-1", vec![version_doc("a-1", "v-1", "alpha title")])
        .await
        .unwrap();
    assert_eq!(index.stats().await.live, 1);
    index.close().await;

    let reopened = SearchIndex::open_or_create(path.to_str().unwrap())
        .await
        .unwrap();
    assert!(!reopened.was_recreated(), "healthy reopen must not wipe");
    assert_eq!(reopened.stats().await.live, 1, "committed doc survives");
    reopened.close().await;
}

#[tokio::test]
async fn replace_article_twice_leaves_exactly_one_version() {
    let (index, _path) = fresh_index("replace_twice").await;
    index
        .replace_article("a-1", vec![version_doc("a-1", "v-1", "first title")])
        .await
        .unwrap();
    index
        .replace_article(
            "a-1",
            vec![
                version_doc("a-1", "v-1", "second title"),
                comment_doc("a-1", "c-1", "a comment"),
            ],
        )
        .await
        .unwrap();
    let stats = index.stats().await;
    assert_eq!(stats.live, 2, "old set fully replaced by new set");
    assert_eq!(stats.deleted, 1, "replaced doc becomes a tombstone");
    index.close().await;
}

#[tokio::test]
async fn empty_replacement_removes_every_document_of_the_article() {
    let (index, _path) = fresh_index("empty_replace").await;
    index
        .replace_article(
            "a-1",
            vec![
                version_doc("a-1", "v-1", "alpha title"),
                comment_doc("a-1", "c-1", "comment body"),
            ],
        )
        .await
        .unwrap();
    index.replace_article("a-1", Vec::new()).await.unwrap();
    let stats = index.stats().await;
    assert_eq!(stats.live, 0);
    assert_eq!(stats.deleted, 2);
    index.close().await;
}

#[tokio::test]
async fn batch_replace_swaps_multiple_articles_in_one_commit() {
    let (index, _path) = fresh_index("batch").await;
    index
        .replace_article("a-1", vec![version_doc("a-1", "v-1", "one title")])
        .await
        .unwrap();
    index
        .replace_article("a-2", vec![version_doc("a-2", "v-2", "two title")])
        .await
        .unwrap();
    let replaced = index
        .replace_articles(vec![
            (
                "a-1".to_string(),
                vec![version_doc("a-1", "v-9", "one rewritten")],
            ),
            ("a-2".to_string(), Vec::new()),
            (
                "a-3".to_string(),
                vec![version_doc("a-3", "v-3", "three title")],
            ),
        ])
        .await
        .unwrap();
    assert_eq!(replaced, 3);
    let stats = index.stats().await;
    assert_eq!(stats.live, 2, "a-1 rewritten, a-2 removed, a-3 added");
    index.close().await;
}

#[tokio::test]
async fn rebuild_wipes_tombstones_and_indexes_only_the_given_set() {
    let (index, _path) = fresh_index("rebuild").await;
    for article in ["a-1", "a-2"] {
        index
            .replace_article(article, vec![version_doc(article, article, "seed title")])
            .await
            .unwrap();
    }
    index.replace_article("a-1", Vec::new()).await.unwrap();
    let before = index.stats().await;
    assert!(before.deleted >= 1);

    let indexed = index
        .rebuild(vec![(
            "a-9".to_string(),
            vec![
                version_doc("a-9", "v-9a", "fresh alpha"),
                comment_doc("a-9", "c-9", "fresh comment"),
            ],
        )])
        .await
        .unwrap();
    assert_eq!(indexed, 2);
    let after = index.stats().await;
    assert_eq!(
        (after.indexed, after.live, after.deleted),
        (2, 2, 0),
        "clear_index must drop tombstones"
    );
    index.close().await;
}

#[tokio::test]
async fn corrupt_directory_is_healed_and_flagged() {
    let (index, path) = fresh_index("corrupt").await;
    index
        .replace_article("a-1", vec![version_doc("a-1", "v-1", "alpha title")])
        .await
        .unwrap();
    index.close().await;
    fs::write(path.join("meta.json"), "{not json").unwrap();

    let healed = SearchIndex::open_or_create(path.to_str().unwrap())
        .await
        .unwrap();
    assert!(healed.was_recreated(), "corrupt dir must be rebuilt");
    assert_eq!(healed.stats().await.live, 0);
    healed
        .replace_article("a-2", vec![version_doc("a-2", "v-2", "beta title")])
        .await
        .unwrap();
    assert_eq!(healed.stats().await.live, 1, "healed index is usable");
    healed.close().await;
}

#[tokio::test]
async fn stale_schema_marker_forces_recreate() {
    let (index, path) = fresh_index("stale_marker").await;
    index
        .replace_article("a-1", vec![version_doc("a-1", "v-1", "alpha title")])
        .await
        .unwrap();
    index.close().await;
    fs::write(path.join("nail_schema_version"), "5").unwrap();

    let recreated = SearchIndex::open_or_create(path.to_str().unwrap())
        .await
        .unwrap();
    assert!(recreated.was_recreated());
    assert_eq!(recreated.stats().await.live, 0);
    recreated.close().await;
}

#[tokio::test]
async fn mismatched_article_id_in_documents_is_rejected() {
    let (index, _path) = fresh_index("mismatch").await;
    let result = index
        .replace_article("a-1", vec![version_doc("OTHER", "v-1", "alpha title")])
        .await;
    assert!(matches!(result, Err(Error::Engine(_))));
    assert_eq!(index.stats().await.live, 0, "nothing written on rejection");
    index.close().await;
}

#[tokio::test]
async fn no_op_replacement_is_accepted_and_changes_nothing() {
    let (index, _path) = fresh_index("no_op").await;
    index
        .replace_article("missing-article", Vec::new())
        .await
        .unwrap();
    let stats = index.stats().await;
    assert_eq!((stats.indexed, stats.live, stats.deleted), (0, 0, 0));
    index.close().await;
}

#[tokio::test]
async fn mixed_batch_with_one_bad_key_writes_nothing() {
    let (index, _path) = fresh_index("batch_mismatch").await;
    let result = index
        .replace_articles(vec![
            (
                "a-1".to_string(),
                vec![version_doc("a-1", "v-1", "ok title")],
            ),
            (
                "a-2".to_string(),
                vec![IndexDoc::Comment(comment_doc_inner("WRONG"))],
            ),
        ])
        .await;
    assert!(matches!(result, Err(Error::Engine(_))));
    assert_eq!(index.stats().await.indexed, 0);
    index.close().await;
}

fn comment_doc_inner(article_id: &str) -> crate::doc::CommentDoc {
    crate::doc::CommentDoc {
        comment_id: "c-x".to_string(),
        version_id: "v-x".to_string(),
        article_id: article_id.to_string(),
        author_name: "bob".to_string(),
        author_id: "u-2".to_string(),
        role: String::new(),
        content: "text".to_string(),
        ts: 0,
    }
}
