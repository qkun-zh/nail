use axum::body::Body;
use axum::extract::FromRequest;
use axum::http::Request;

use super::context::{TestCtx, test_config, valid_pdf};
use crate::interface::envelope::ApiError;
use crate::interface::extractor::AppMultipart;
use crate::interface::multipart::{MultipartField, collect_fields};

fn raw_multipart(
    boundary: &str,
    fields: &[(&str, &[u8])],
    file_field: &str,
    file_name: &str,
    file_bytes: &[u8],
) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n")
                .as_bytes(),
        );
        body.extend_from_slice(value);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{file_field}\"; filename=\"{file_name}\"\r\nContent-Type: application/pdf\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(file_bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

async fn extract_fields(
    context: &TestCtx,
    boundary: &str,
    body: Vec<u8>,
    pdf_fields: &[&str],
    text_fields: &[&str],
) -> Result<std::collections::HashMap<String, MultipartField>, ApiError> {
    let (parts, body) = Request::builder()
        .method("POST")
        .uri("/create")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .expect("request")
        .into_parts();
    let multipart = AppMultipart::from_request(Request::from_parts(parts, body), &context.state)
        .await
        .expect("multipart extract");
    collect_fields(&context.state, multipart.0, pdf_fields, text_fields).await
}

async fn rejected_message(
    context: &TestCtx,
    boundary: &str,
    body: Vec<u8>,
    pdf_fields: &[&str],
    text_fields: &[&str],
) -> String {
    match extract_fields(context, boundary, body, pdf_fields, text_fields).await {
        Ok(_) => panic!("collect_fields must reject"),
        Err(error) => error.message,
    }
}

#[tokio::test]
async fn collect_fields_returns_accepted_fields_and_skips_unknown_ones() {
    let context = TestCtx::new().await.expect("test context");
    let body = raw_multipart(
        "b1",
        &[
            ("title", b"hello".as_slice()),
            ("unexpected_field", b"ignored".as_slice()),
            ("version", b"1.0.0".as_slice()),
        ],
        "file",
        "a.pdf",
        &valid_pdf(),
    );
    let fields = extract_fields(&context, "b1", body, &["file"], &["title", "version"])
        .await
        .expect("collect");
    assert_eq!(fields.len(), 3);
    match &fields["title"] {
        MultipartField::Text(value) => assert_eq!(value, "hello"),
        MultipartField::Pdf(_) => panic!("title must be text"),
    }
    match &fields["version"] {
        MultipartField::Text(value) => assert_eq!(value, "1.0.0"),
        MultipartField::Pdf(_) => panic!("version must be text"),
    }
    match &fields["file"] {
        MultipartField::Pdf(upload) => {
            assert_eq!(upload.hash, common::hash::pdf(&valid_pdf()));
        }
        MultipartField::Text(_) => panic!("file must be pdf"),
    }
}

#[tokio::test]
async fn collect_fields_last_duplicate_wins() {
    let context = TestCtx::new().await.expect("test context");
    let body = raw_multipart(
        "b2",
        &[
            ("version", b"1.0.0".as_slice()),
            ("version", b"1.1.0".as_slice()),
        ],
        "file",
        "a.pdf",
        &valid_pdf(),
    );
    let fields = extract_fields(&context, "b2", body, &["file"], &["version"])
        .await
        .expect("collect");
    match &fields["version"] {
        MultipartField::Text(value) => assert_eq!(value, "1.1.0"),
        MultipartField::Pdf(_) => panic!("version must be text"),
    }
}

#[tokio::test]
async fn collect_fields_rejects_non_utf8_text() {
    let context = TestCtx::new().await.expect("test context");
    let body = raw_multipart(
        "b3",
        &[("title", [0xff, 0xfe, b'x'].as_slice())],
        "file",
        "a.pdf",
        &valid_pdf(),
    );
    let message = rejected_message(&context, "b3", body, &["file"], &["title"]).await;
    assert_eq!(message, "text field must be UTF-8");
}

#[tokio::test]
async fn collect_fields_rejects_an_oversized_text_field() {
    let mut config = test_config();
    config.server.max_text_field_bytes = 8;
    let context = TestCtx::with_config(config).await.expect("test context");
    let body = raw_multipart(
        "b4",
        &[("title", b"too long value".as_slice())],
        "file",
        "a.pdf",
        &valid_pdf(),
    );
    let message = rejected_message(&context, "b4", body, &["file"], &["title"]).await;
    assert_eq!(message, "text field too large");
}
