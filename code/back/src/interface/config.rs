use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use nail_common::response::RuntimeLimits;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::json_response;

pub async fn read_config(State(state): State<AppState>) -> impl IntoResponse {
    let server = &state.config.server;
    json_response::<RuntimeLimits>(
        StatusCode::OK,
        RuntimeLimits {
            max_tags_per_article: server.max_tags_per_article as u64,
            max_comment_body_chars: server.max_comment_body_chars,
            max_version_note_chars: server.max_version_note_chars,
            max_title_chars: server.max_title_chars,
            max_summary_chars: server.max_summary_chars,
            max_pdf_size_bytes: server.max_pdf_size_bytes,
            max_text_field_bytes: server.max_text_field_bytes,
            download_token_ttl_seconds: server.download_token_ttl_seconds,
            search_page_size: server.search_page_size,
            max_search_pages: server.max_search_pages,
        },
        "ok",
    )
}
