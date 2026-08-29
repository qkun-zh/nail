use axum::http::StatusCode;

use super::context::TestCtx;

#[tokio::test]
async fn read_config_over_http_serves_the_runtime_limits_without_a_session() {
    let context = TestCtx::new().await.expect("test context");

    let (status, body) = context.get("/config", None).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    let data = &body["data"];
    assert_eq!(data["max_tags_per_article"].as_u64(), Some(8));
    assert_eq!(data["max_comment_body_chars"].as_u64(), Some(1024));
    assert_eq!(data["max_version_note_chars"].as_u64(), Some(1024));
    assert_eq!(data["max_title_chars"].as_u64(), Some(200));
    assert_eq!(data["max_summary_chars"].as_u64(), Some(2000));
    assert_eq!(data["max_pdf_size_bytes"].as_u64(), Some(32 * 1024 * 1024));
    assert_eq!(data["max_text_field_bytes"].as_u64(), Some(1024 * 1024));
    assert_eq!(data["download_token_ttl_seconds"].as_u64(), Some(60));
    assert_eq!(data["search_page_size"].as_u64(), Some(8));
    assert_eq!(data["max_search_pages"].as_u64(), Some(1024));
    assert_eq!(data["tag_page_size"].as_u64(), Some(8));
}

#[test]
fn generated_route_constants_match_their_literal_paths() {
    use crate::interface::router::{
        ROUTE_ARTICLES_ID_VERSIONS_VID_CONTENT, ROUTE_CHALLENGES, ROUTE_COMMENTS_ID,
        ROUTE_ROLES_ID, ROUTE_USERS_ID, ROUTE_VERSIONS_ID_COMMENTS,
    };
    assert_eq!(ROUTE_CHALLENGES, "/challenges");
    assert_eq!(ROUTE_USERS_ID, "/users/{id}");
    assert_eq!(
        ROUTE_ARTICLES_ID_VERSIONS_VID_CONTENT,
        "/articles/{id}/versions/{version_id}/content"
    );
    assert_eq!(ROUTE_VERSIONS_ID_COMMENTS, "/versions/{id}/comments");
    assert_eq!(ROUTE_ROLES_ID, "/roles/{id}");
    assert_eq!(ROUTE_COMMENTS_ID, "/comments/{id}");
}
