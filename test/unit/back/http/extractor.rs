use axum::extract::FromRequestParts;
use axum::http::Request;
use axum::http::StatusCode;

use super::context::TestCtx;
use crate::interface::envelope::ApiError;
use crate::interface::extractor::AppPaged;
use crate::interface::principal::read_session_token;

async fn extract(uri: &str, context: &TestCtx) -> Result<(u64, u64), ApiError> {
    let (mut parts, ()) = Request::builder()
        .uri(uri)
        .body(())
        .expect("request")
        .into_parts();
    AppPaged::from_request_parts(&mut parts, &context.state)
        .await
        .map(|paged| paged.0)
}

#[tokio::test]
async fn app_paged_defaults_to_page_one_and_the_search_page_size() {
    let context = TestCtx::new().await.expect("test context");
    let (page, limit) = extract("/read", &context).await.expect("extract");
    assert_eq!(
        (page, limit),
        (1, context.state.configurator.search_page_size())
    );
}

#[tokio::test]
async fn app_paged_reads_and_returns_explicit_values() {
    let context = TestCtx::new().await.expect("test context");
    let (page, limit) = extract("/read?page=3&limit=10", &context)
        .await
        .expect("extract");
    assert_eq!((page, limit), (3, 10));
}

#[tokio::test]
async fn app_paged_clamps_limit_to_the_max_page_size() {
    let context = TestCtx::new().await.expect("test context");
    let (page, limit) = extract("/read?limit=500", &context).await.expect("extract");
    assert_eq!((page, limit), (1, 200));
}

#[tokio::test]
async fn app_paged_clamps_a_zero_limit_to_one() {
    let context = TestCtx::new().await.expect("test context");
    let (page, limit) = extract("/read?limit=0", &context).await.expect("extract");
    assert_eq!((page, limit), (1, 1));
}

#[tokio::test]
async fn app_paged_clamps_page_zero_to_one() {
    let context = TestCtx::new().await.expect("test context");
    let (page, limit) = extract("/read?page=0", &context).await.expect("extract");
    assert_eq!(
        (page, limit),
        (1, context.state.configurator.search_page_size())
    );
}

#[tokio::test]
async fn app_paged_rejects_a_page_beyond_max_search_pages() {
    let context = TestCtx::new().await.expect("test context");
    let error = extract("/read?page=1025", &context)
        .await
        .expect_err("must reject");
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "page exceeds max search pages");
}

#[tokio::test]
async fn app_paged_rejects_a_non_numeric_page() {
    let context = TestCtx::new().await.expect("test context");
    let error = extract("/read?page=abc", &context)
        .await
        .expect_err("must reject");
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "invalid query parameters");
}

#[tokio::test]
async fn app_paged_rejects_a_non_numeric_limit() {
    let context = TestCtx::new().await.expect("test context");
    let error = extract("/read?limit=abc", &context)
        .await
        .expect_err("must reject");
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "invalid query parameters");
}

#[test]
fn read_session_token_returns_the_header_value() {
    let (parts, ()) = Request::builder()
        .uri("/token/create")
        .header("session-token", "tok-123")
        .body(())
        .expect("request")
        .into_parts();
    assert_eq!(read_session_token(&parts).as_deref(), Some("tok-123"));
}

#[test]
fn read_session_token_returns_none_when_the_header_is_absent() {
    let (parts, ()) = Request::builder()
        .uri("/token/create")
        .body(())
        .expect("request")
        .into_parts();
    assert_eq!(read_session_token(&parts), None);
}

#[test]
fn read_session_token_returns_none_for_a_non_utf8_header() {
    let (mut parts, ()) = Request::builder()
        .uri("/token/create")
        .body(())
        .expect("request")
        .into_parts();
    parts.headers.insert(
        "session-token",
        axum::http::HeaderValue::from_bytes(b"\xff").expect("header"),
    );
    assert_eq!(read_session_token(&parts), None);
}
