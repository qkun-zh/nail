use axum::http::Uri;

use crate::infrastructure::server::{redact_token_query, redacted_uri};

fn uri(path: &str) -> Uri {
    Uri::builder().path_and_query(path).build().unwrap()
}

#[test]
fn path_without_query_stays_plain() {
    assert_eq!(redacted_uri(&uri("/api/tags/read")), "/api/tags/read");
}

#[test]
fn query_without_token_is_kept_verbatim() {
    assert_eq!(
        redacted_uri(&uri("/api/articles/search?query=x&page=1&limit=8")),
        "/api/articles/search?query=x&page=1&limit=8"
    );
}

#[test]
fn token_param_is_redacted() {
    assert_eq!(
        redacted_uri(&uri("/api/articles/9/versions/3/content?token=abc123")),
        "/api/articles/9/versions/3/content?token=<REDACTED>"
    );
}

#[test]
fn token_is_redacted_among_other_params_keeping_order() {
    assert_eq!(
        redact_token_query("a=1&token=abc&b=2"),
        "a=1&token=<REDACTED>&b=2"
    );
}

#[test]
fn bare_token_key_is_redacted() {
    assert_eq!(redact_token_query("a=1&token"), "a=1&token=<REDACTED>");
}

#[test]
fn similar_keys_are_not_touched() {
    assert_eq!(
        redact_token_query("download_token=x&tokens=y"),
        "download_token=x&tokens=y"
    );
}
