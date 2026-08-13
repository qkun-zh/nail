
use axum::Json;
use axum::extract::State;
use common::response::ResponseEnvelope;

use crate::other::AppState;

pub async fn read_config(
    State(state): State<AppState>,
) -> Json<ResponseEnvelope<serde_json::Value>> {
    let s = &state.config.server;
    Json(ResponseEnvelope::ok(
        200,
        serde_json::json!({
            "max_tags_per_article": s.max_tags_per_article,
            "max_comment_body_chars": s.max_comment_body_chars,
            "max_version_note_chars": s.max_version_note_chars,
            "max_title_chars": s.max_title_chars,
            "max_summary_chars": s.max_summary_chars,
            "max_pdf_size_bytes": s.max_pdf_size_bytes,
            "search_page_size": s.search_page_size,
            "max_search_pages": s.max_search_pages,
            "max_page": s.max_page,
        }),
        "ok",
    ))
}
