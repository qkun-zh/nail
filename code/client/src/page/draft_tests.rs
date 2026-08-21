use crate::page::draft::{build_draft_query, draft_url};

#[test]
fn skips_empty_field_values() {
    assert_eq!(build_draft_query(&[]), "");
    assert_eq!(build_draft_query(&[("body", "")]), "");
}

#[test]
fn encodes_keys_and_values() {
    assert_eq!(
        build_draft_query(&[("body", "hello world"), ("reply", "hi")]),
        "body=hello%20world&reply=hi"
    );
    assert_eq!(build_draft_query(&[("a", "x=y&z")]), "a=x%3Dy%26z");
}

#[test]
fn builds_a_draft_url_with_or_without_query() {
    assert_eq!(draft_url("/x", &[("body", "v")]), "/x?body=v");
    assert_eq!(draft_url("/x", &[]), "/x");
    assert_eq!(draft_url("/x", &[("body", "")]), "/x");
}
