use crate::infrastructure::limits::{apply_fallbacks, compile_time_defaults};
use nail_common::response::RuntimeLimits;

fn limits() -> RuntimeLimits {
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

#[test]
fn defaults_mirror_the_server_config() {
    assert_eq!(compile_time_defaults(), limits());
}

#[test]
fn zero_numeric_limits_fall_back_to_defaults() {
    let server = RuntimeLimits {
        search_page_size: 0,
        max_pdf_size_bytes: 0,
        ..limits()
    };
    let merged = apply_fallbacks(&server);
    assert_eq!(merged.search_page_size, 8);
    assert_eq!(merged.max_pdf_size_bytes, 33_554_432);
}

#[test]
fn fully_populated_server_values_are_kept() {
    let server = RuntimeLimits {
        max_tags_per_article: 3,
        max_comment_body_chars: 500,
        max_version_note_chars: 900,
        max_title_chars: 120,
        max_summary_chars: 1500,
        max_pdf_size_bytes: 1_000_000,
        max_text_field_bytes: 50_000,
        download_token_ttl_seconds: 30,
        search_page_size: 20,
        max_search_pages: 500,
    };
    assert_eq!(apply_fallbacks(&server), server);
}

#[test]
fn every_zero_numeric_field_falls_back_individually() {
    let server = RuntimeLimits {
        max_tags_per_article: 0,
        max_comment_body_chars: 0,
        max_version_note_chars: 0,
        max_title_chars: 0,
        max_summary_chars: 0,
        max_pdf_size_bytes: 0,
        max_text_field_bytes: 0,
        download_token_ttl_seconds: 0,
        search_page_size: 0,
        max_search_pages: 0,
    };
    let merged = apply_fallbacks(&server);
    assert_eq!(merged.max_tags_per_article, 8);
    assert_eq!(merged.max_comment_body_chars, 1024);
    assert_eq!(merged.max_version_note_chars, 1024);
    assert_eq!(merged.max_title_chars, 200);
    assert_eq!(merged.max_summary_chars, 2000);
    assert_eq!(merged.max_pdf_size_bytes, 33_554_432);
    assert_eq!(merged.max_text_field_bytes, 1_048_576);
    assert_eq!(merged.download_token_ttl_seconds, 60);
    assert_eq!(merged.search_page_size, 8);
    assert_eq!(merged.max_search_pages, 1024);
}
