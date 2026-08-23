use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::schema::{SCHEMA_VERSION, fields, meta, read_marker, write_marker};

fn scratch_dir(label: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("searcher_test_{}_{}", std::process::id(), label));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn field_json(index: usize) -> serde_json::Value {
    serde_json::to_value(&fields()[index]).unwrap()
}

fn flag(value: &serde_json::Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

#[test]
fn v6_has_thirteen_fields_in_fixed_order() {
    let names: Vec<String> = fields().iter().map(|field| field.field.clone()).collect();
    assert_eq!(
        names,
        vec![
            "version_id",
            "article_id",
            "comment_id",
            "version_number",
            "title",
            "summary",
            "author_name",
            "author_id",
            "role",
            "note",
            "tags",
            "content",
            "ts",
        ]
    );
}

#[test]
fn v6_drops_doc_type() {
    for field in fields() {
        assert_ne!(field.field, "doc_type");
    }
}

#[test]
fn v6_title_is_text_without_facet() {
    let title = field_json(4);
    assert_eq!(title["field"], json!("title"));
    assert_eq!(title["field_type"], json!("Text"));
    assert!(!flag(&title, "facet"));
    assert_eq!(title["boost"], json!(3.0));
}

#[test]
fn v6_author_name_is_text_without_facet() {
    let author_name = field_json(6);
    assert_eq!(author_name["field"], json!("author_name"));
    assert_eq!(author_name["field_type"], json!("Text"));
    assert!(!flag(&author_name, "facet"));
}

#[test]
fn v6_key_fields_keep_facets_and_types() {
    let expected = [
        (0usize, "String32", true),
        (1, "String32", true),
        (2, "String32", true),
        (7, "String32", true),
        (8, "String16", true),
        (12, "Timestamp", true),
    ];
    for (index, field_type, facet) in expected {
        let value = field_json(index);
        assert_eq!(value["field_type"], json!(field_type), "field {index}");
        assert_eq!(value["facet"], json!(facet), "field {index}");
    }
}

#[test]
fn v6_summary_is_longest_field_source() {
    let summary = field_json(5);
    assert_eq!(summary["longest"], json!(true));
}

#[test]
fn meta_matches_expected_engine_settings() {
    let value = serde_json::to_value(meta()).unwrap();
    assert_eq!(value["name"], json!("nail_articles"));
    assert_eq!(value["lexical_similarity"], json!("Bm25f"));
    assert_eq!(value["tokenizer"], json!("UnicodeAlphanumericFolded"));
    assert_eq!(value["document_compression"], json!("Snappy"));
    assert_eq!(value["access_type"], json!("Mmap"));
}

#[test]
fn marker_roundtrips_version() {
    let directory = scratch_dir("marker_roundtrip");
    assert_eq!(read_marker(&directory), None);
    write_marker(&directory).unwrap();
    assert_eq!(read_marker(&directory), Some(SCHEMA_VERSION.to_string()));
    let _ = fs::remove_dir_all(&directory);
}
