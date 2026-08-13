
use uuid::Uuid;

use axum::http::StatusCode;

use crate::api::{sanitize_attachment_filename, serve_pdf_file};

#[test]
fn sanitize_attachment_filename_strips_quotes_backslashes_and_non_ascii() {
    assert_eq!(
        sanitize_attachment_filename("019f1234-5678.pdf"),
        "019f1234-5678.pdf"
    );
    assert_eq!(sanitize_attachment_filename("a\"b\\c.pdf"), "abc.pdf");
    assert_eq!(sanitize_attachment_filename("\u{4e2d}\u{6587}.pdf"), ".pdf");
    assert_eq!(sanitize_attachment_filename("\"\\\\\""), "article.pdf");
}

async fn temporary_pdf_file(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("nail_api_test_{}", Uuid::now_v7()));
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let path = dir.join(name);
    tokio::fs::write(&path, b"%PDF-1.4 fake content\n")
        .await
        .unwrap();
    path
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn serve_pdf_file_streams_with_attachment_headers() {
    let path = temporary_pdf_file("019f1234-5678.pdf").await;
    let response = serve_pdf_file(path.to_str().unwrap())
        .await
        .expect("serve_pdf_file");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap(),
        "application/pdf"
    );
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::CONTENT_DISPOSITION)
            .unwrap(),
        "attachment; filename=\"019f1234-5678.pdf\""
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(bytes.as_ref(), b"%PDF-1.4 fake content\n");
    let _ = tokio::fs::remove_dir_all(path.parent().unwrap()).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn serve_pdf_file_sanitizes_filename_in_disposition() {
    let path = temporary_pdf_file("a\"b\\c.pdf").await;
    let response = serve_pdf_file(path.to_str().unwrap())
        .await
        .expect("serve_pdf_file");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::CONTENT_DISPOSITION)
            .unwrap(),
        "attachment; filename=\"abc.pdf\""
    );
    let _ = tokio::fs::remove_dir_all(path.parent().unwrap()).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn serve_pdf_file_missing_file_returns_with_404() {
    let missing = std::env::temp_dir().join(format!("nail_api_missing_{}", Uuid::now_v7()));
    let err = serve_pdf_file(missing.to_str().unwrap()).await.unwrap_err();
    assert_eq!(err.0, StatusCode::NOT_FOUND);
}
