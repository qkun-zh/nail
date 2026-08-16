use crate::request::url::{build_path_with_query, encode_component};

#[test]
fn leaves_encode_uri_component_safe_characters_alone() {
    let safe = "ABCabc0123-_.!~*'()";
    assert_eq!(encode_component(safe), safe);
}

#[test]
fn encodes_spaces_and_special_characters() {
    assert_eq!(encode_component("a b"), "a%20b");
    assert_eq!(encode_component("a/b"), "a%2Fb");
    assert_eq!(encode_component("a?b=c&d"), "a%3Fb%3Dc%26d");
    assert_eq!(encode_component("+"), "%2B");
    assert_eq!(encode_component("#tag"), "%23tag");
}

#[test]
fn encodes_utf8_bytes_percent_wise() {
    assert_eq!(encode_component("中文"), "%E4%B8%AD%E6%96%87");
    assert_eq!(encode_component("é"), "%C3%A9");
}

#[test]
fn builds_encoded_path_segments() {
    assert_eq!(
        build_path_with_query(&["article", "read"], &[]),
        "/article/read"
    );
    assert_eq!(
        build_path_with_query(&["article", "a b", "read"], &[]),
        "/article/a%20b/read"
    );
    assert_eq!(
        build_path_with_query(&["version", "x/y", "comments"], &[]),
        "/version/x%2Fy/comments"
    );
}

#[test]
fn appends_encoded_query_parameters() {
    assert_eq!(
        build_path_with_query(&["article", "read"], &[("page", "2"), ("limit", "8")]),
        "/article/read?page=2&limit=8"
    );
    assert_eq!(
        build_path_with_query(&["session", "read"], &[("id", "true"), ("name", "true")]),
        "/session/read?id=true&name=true"
    );
    assert_eq!(
        build_path_with_query(
            &["article", "read"],
            &[("key_word", "hello world"), ("from", "2024-01-15T10:30:00")]
        ),
        "/article/read?key_word=hello%20world&from=2024-01-15T10%3A30%3A00"
    );
}

#[test]
fn builds_path_without_query_when_none_given() {
    assert_eq!(
        build_path_with_query(&["challenge", "create"], &[]),
        "/challenge/create"
    );
}
