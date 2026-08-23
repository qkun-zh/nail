use crate::request::download::{
    filename_from_content_disposition, origin_of, resolve_download_url,
};

#[test]
fn extracts_origin_from_absolute_urls() {
    assert_eq!(
        origin_of("https://api.example.com:8080/api/x"),
        Some("https://api.example.com:8080".to_string())
    );
    assert_eq!(origin_of("http://a/b"), Some("http://a".to_string()));
}

#[test]
fn rejects_relative_or_non_http_origins() {
    assert_eq!(origin_of("/api/x"), None);
    assert_eq!(origin_of("notaurl"), None);
    assert_eq!(origin_of("ftp://a/b"), None);
    assert_eq!(origin_of("https:///bad"), Some("https://bad".to_string()));
}

#[test]
fn resolves_root_relative_minted_urls_against_the_window_origin() {
    let minted = "/api/articles/a/versions/v/content?token=t";
    assert_eq!(
        resolve_download_url(minted, "https://app.example.com"),
        Some("https://app.example.com/api/articles/a/versions/v/content?token=t".to_string())
    );
}

#[test]
fn refuses_protocol_relative_urls() {
    assert_eq!(
        resolve_download_url("//evil.com/x", "https://app.example.com"),
        None
    );
}

#[test]
fn accepts_same_origin_absolute_urls() {
    let minted = "https://app.example.com/api/x";
    assert_eq!(
        resolve_download_url(minted, "https://app.example.com"),
        Some("https://app.example.com/api/x".to_string())
    );
}

#[test]
fn refuses_foreign_origins() {
    assert_eq!(
        resolve_download_url("https://evil.com/x", "https://app.example.com"),
        None
    );
}

#[test]
fn refuses_scheme_mismatch() {
    assert_eq!(
        resolve_download_url("http://app.example.com/x", "https://app.example.com"),
        None
    );
}

#[test]
fn refuses_garbage() {
    assert_eq!(
        resolve_download_url("garbage", "https://app.example.com"),
        None
    );
}

#[test]
fn parses_filename_from_content_disposition() {
    assert_eq!(
        filename_from_content_disposition(Some("attachment; filename=\"abc.pdf\"")),
        "abc.pdf"
    );
    assert_eq!(
        filename_from_content_disposition(Some("attachment; filename=\"article.pdf\"")),
        "article.pdf"
    );
}

#[test]
fn falls_back_to_article_pdf_for_missing_or_malformed_filenames() {
    assert_eq!(filename_from_content_disposition(None), "article.pdf");
    assert_eq!(
        filename_from_content_disposition(Some("attachment")),
        "article.pdf"
    );
    assert_eq!(
        filename_from_content_disposition(Some("attachment; filename=\"\"")),
        "article.pdf"
    );
    assert_eq!(
        filename_from_content_disposition(Some("attachment; filename=\"a/b.pdf\"")),
        "article.pdf"
    );
}
