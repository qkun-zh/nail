use common::response::RuntimeLimits;
use leptos::prelude::*;

pub fn compile_time_defaults() -> RuntimeLimits {
    RuntimeLimits {
        max_tags_per_article: 8,
        max_comment_body_chars: 1024,
        max_version_note_chars: 1024,
        max_title_chars: 200,
        max_summary_chars: 2000,
        max_pdf_size_bytes: 33_554_432,
        max_text_field_bytes: 1_048_576,
        download_token_ttl_seconds: 60,
        search_page_size: 8,
        max_search_pages: 1024,
    }
}

pub fn apply_fallbacks(server: &RuntimeLimits) -> RuntimeLimits {
    let defaults = compile_time_defaults();
    RuntimeLimits {
        max_tags_per_article: nonzero_or(
            server.max_tags_per_article,
            defaults.max_tags_per_article,
        ),
        max_comment_body_chars: nonzero_or(
            server.max_comment_body_chars,
            defaults.max_comment_body_chars,
        ),
        max_version_note_chars: nonzero_or(
            server.max_version_note_chars,
            defaults.max_version_note_chars,
        ),
        max_title_chars: nonzero_or(server.max_title_chars, defaults.max_title_chars),
        max_summary_chars: nonzero_or(server.max_summary_chars, defaults.max_summary_chars),
        max_pdf_size_bytes: nonzero_or(server.max_pdf_size_bytes, defaults.max_pdf_size_bytes),
        max_text_field_bytes: nonzero_or(
            server.max_text_field_bytes,
            defaults.max_text_field_bytes,
        ),
        download_token_ttl_seconds: nonzero_or(
            server.download_token_ttl_seconds,
            defaults.download_token_ttl_seconds,
        ),
        search_page_size: nonzero_or(server.search_page_size, defaults.search_page_size),
        max_search_pages: nonzero_or(server.max_search_pages, defaults.max_search_pages),
    }
}

fn nonzero_or(value: u64, fallback: u64) -> u64 {
    if value == 0 { fallback } else { value }
}

pub fn provide_limits() -> RwSignal<RuntimeLimits> {
    let limits = RwSignal::new(compile_time_defaults());
    provide_context(limits);
    leptos::task::spawn_local(async move {
        if let Ok(server) =
            crate::request::http::get_json::<RuntimeLimits>("/config", false, None).await
        {
            limits.set(apply_fallbacks(&server));
        }
    });
    limits
}

pub fn use_limits() -> RwSignal<RuntimeLimits> {
    use_context::<RwSignal<RuntimeLimits>>()
        .unwrap_or_else(|| RwSignal::new(compile_time_defaults()))
}

#[cfg(test)]
#[path = "limits_tests.rs"]
mod tests;
