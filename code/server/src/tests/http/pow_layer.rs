use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use serde_json::json;
use tower::ServiceExt;

use super::context::TestCtx;

#[tokio::test]
async fn challenges_endpoint_does_not_require_a_pow_header() {
    let context = TestCtx::new().await.expect("test context");
    let (status, body) = context.post("/challenges", json!({}), None).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(body["data"]["id"].as_str().is_some());
}

#[tokio::test]
async fn a_missing_pow_header_is_rejected() {
    let context = TestCtx::new().await.expect("test context");
    let request = Request::builder()
        .method("GET")
        .uri("/users")
        .body(Body::empty())
        .expect("build request");
    let response = context.app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(value["message"].as_str(), Some("missing x-pow header"));
}

#[tokio::test]
async fn a_malformed_pow_header_is_rejected() {
    let context = TestCtx::new().await.expect("test context");
    let request = Request::builder()
        .method("GET")
        .uri("/users")
        .header("x-pow", "not-json")
        .body(Body::empty())
        .expect("build request");
    let response = context.app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_proof_for_an_unissued_challenge_is_rejected() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.client_pow();
    let request = Request::builder()
        .method("GET")
        .uri("/users")
        .header("x-pow", serde_json::to_string(&pow).expect("serialize pow"))
        .body(Body::empty())
        .expect("build request");
    let response = context.app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(
        value["message"].as_str(),
        Some("challenge not issued, expired, or already used")
    );
}

#[tokio::test]
async fn an_issued_challenge_is_consumed_by_the_first_request() {
    let context = TestCtx::new().await.expect("test context");
    let pow = context.issued_pow();
    let header = serde_json::to_string(&pow).expect("serialize pow");
    let request = Request::builder()
        .method("GET")
        .uri("/users")
        .header("x-pow", &header)
        .body(Body::empty())
        .expect("build request");
    let response = context.app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let request = Request::builder()
        .method("GET")
        .uri("/users")
        .header("x-pow", &header)
        .body(Body::empty())
        .expect("build request");
    let response = context.app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(
        value["message"].as_str(),
        Some("challenge not issued, expired, or already used")
    );
}

#[tokio::test]
async fn a_tampered_solution_is_rejected() {
    let context = TestCtx::new().await.expect("test context");
    let mut pow = context.issued_pow();
    pow.solution = format!(
        "{}{}",
        if pow.solution.starts_with('0') {
            '1'
        } else {
            '0'
        },
        &pow.solution[1..]
    );
    let request = Request::builder()
        .method("GET")
        .uri("/users")
        .header("x-pow", serde_json::to_string(&pow).expect("serialize pow"))
        .body(Body::empty())
        .expect("build request");
    let response = context.app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(value["message"].as_str(), Some("PoW verification failed"));
}
