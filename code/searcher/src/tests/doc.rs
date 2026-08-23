use serde_json::json;

use crate::doc::{CommentDoc, SearchDoc, VersionDoc};

fn sample_version() -> VersionDoc {
    VersionDoc {
        version_id: "v-1".to_string(),
        article_id: "a-1".to_string(),
        version_number: "3".to_string(),
        title: "Hello world".to_string(),
        summary: "A summary".to_string(),
        author_name: "Alice".to_string(),
        author_id: "u-1".to_string(),
        role: "author,reviewer".to_string(),
        note: "first draft".to_string(),
        tags: vec!["rust".to_string(), "search".to_string()],
        ts: 1_700_000_000,
    }
}

fn sample_comment() -> CommentDoc {
    CommentDoc {
        comment_id: "c-1".to_string(),
        version_id: "v-1".to_string(),
        article_id: "a-1".to_string(),
        author_name: "Bob".to_string(),
        author_id: "u-2".to_string(),
        role: "reviewer".to_string(),
        content: "typo in section 2".to_string(),
        ts: 1_700_000_100,
    }
}

#[test]
fn version_doc_converts_with_all_fields() {
    let value =
        serde_json::to_value(SearchDoc::Version(sample_version()).to_document().unwrap()).unwrap();
    assert_eq!(value["version_id"], json!("v-1"));
    assert_eq!(value["article_id"], json!("a-1"));
    assert_eq!(value["version_number"], json!("3"));
    assert_eq!(value["title"], json!("Hello world"));
    assert_eq!(value["summary"], json!("A summary"));
    assert_eq!(value["author_name"], json!("Alice"));
    assert_eq!(value["author_id"], json!("u-1"));
    assert_eq!(value["role"], json!("author,reviewer"));
    assert_eq!(value["note"], json!("first draft"));
    assert_eq!(value["tags"], json!(["rust", "search"]));
    assert_eq!(value["ts"], json!(1_700_000_000));
}

#[test]
fn version_doc_has_no_discriminator_or_comment_keys() {
    let value =
        serde_json::to_value(SearchDoc::Version(sample_version()).to_document().unwrap()).unwrap();
    assert!(value.get("doc_type").is_none());
    assert!(value.get("comment_id").is_none());
}

#[test]
fn comment_doc_converts_with_all_fields() {
    let value =
        serde_json::to_value(SearchDoc::Comment(sample_comment()).to_document().unwrap()).unwrap();
    assert_eq!(value["comment_id"], json!("c-1"));
    assert_eq!(value["version_id"], json!("v-1"));
    assert_eq!(value["article_id"], json!("a-1"));
    assert_eq!(value["author_name"], json!("Bob"));
    assert_eq!(value["author_id"], json!("u-2"));
    assert_eq!(value["role"], json!("reviewer"));
    assert_eq!(value["content"], json!("typo in section 2"));
    assert_eq!(value["ts"], json!(1_700_000_100));
}

#[test]
fn comment_doc_has_no_version_only_keys() {
    let value =
        serde_json::to_value(SearchDoc::Comment(sample_comment()).to_document().unwrap()).unwrap();
    assert!(value.get("doc_type").is_none());
    assert!(value.get("title").is_none());
    assert!(value.get("summary").is_none());
    assert!(value.get("note").is_none());
    assert!(value.get("tags").is_none());
    assert!(value.get("version_number").is_none());
}

#[test]
fn index_doc_exposes_article_id() {
    let version = SearchDoc::Version(sample_version());
    let comment = SearchDoc::Comment(sample_comment());
    assert_eq!(version.article_id(), "a-1");
    assert_eq!(comment.article_id(), "a-1");
}
