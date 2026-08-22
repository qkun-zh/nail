use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::response::RuntimeLimits;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::json_response;

pub async fn read_config(State(state): State<AppState>) -> impl IntoResponse {
    json_response::<RuntimeLimits>(
        StatusCode::OK,
        RuntimeLimits {
            max_tags_per_article: state.config.server.max_tags_per_article as u64,
            max_comment_body_chars: state.config.server.max_comment_body_chars,
            max_version_note_chars: state.config.server.max_version_note_chars,
            max_title_chars: state.config.server.max_title_chars,
            max_summary_chars: state.config.server.max_summary_chars,
            max_pdf_size_bytes: state.config.server.max_pdf_size_bytes,
            max_text_field_bytes: state.config.server.max_text_field_bytes,
            download_token_ttl_seconds: state.config.cache.download_ttl_seconds,
            search_page_size: state.config.server.search_page_size,
            max_search_pages: state.config.server.max_search_pages,
        },
        "ok",
    )
}
