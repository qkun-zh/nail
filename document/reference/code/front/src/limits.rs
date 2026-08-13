
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Limits {
    #[serde(default = "default_max_tags")]
    pub max_tags_per_article: usize,
    #[serde(default = "default_comment_chars")]
    pub max_comment_body_chars: usize,
    #[serde(default = "default_note_chars")]
    pub max_version_note_chars: usize,
    #[serde(default = "default_title_chars")]
    pub max_title_chars: usize,
    #[serde(default = "default_summary_chars")]
    pub max_summary_chars: usize,
    #[serde(default = "default_pdf_bytes")]
    pub max_pdf_size_bytes: u64,
    #[serde(default = "default_page_size")]
    pub search_page_size: usize,
    #[serde(default = "default_max_pages")]
    pub max_search_pages: usize,
}

fn default_max_tags() -> usize {
    8
}
fn default_comment_chars() -> usize {
    1024
}
fn default_note_chars() -> usize {
    1024
}
fn default_title_chars() -> usize {
    200
}
fn default_summary_chars() -> usize {
    2000
}
fn default_pdf_bytes() -> u64 {
    32 * 1024 * 1024
}
fn default_page_size() -> usize {
    8
}
fn default_max_pages() -> usize {
    1024
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_tags_per_article: default_max_tags(),
            max_comment_body_chars: default_comment_chars(),
            max_version_note_chars: default_note_chars(),
            max_title_chars: default_title_chars(),
            max_summary_chars: default_summary_chars(),
            max_pdf_size_bytes: default_pdf_bytes(),
            search_page_size: default_page_size(),
            max_search_pages: default_max_pages(),
        }
    }
}

fn normalize_limits(mut limits: Limits) -> Limits {
    if limits.max_tags_per_article == 0 {
        limits.max_tags_per_article = default_max_tags();
    }
    if limits.max_comment_body_chars == 0 {
        limits.max_comment_body_chars = default_comment_chars();
    }
    if limits.max_version_note_chars == 0 {
        limits.max_version_note_chars = default_note_chars();
    }
    if limits.max_title_chars == 0 {
        limits.max_title_chars = default_title_chars();
    }
    if limits.max_summary_chars == 0 {
        limits.max_summary_chars = default_summary_chars();
    }
    if limits.max_pdf_size_bytes == 0 {
        limits.max_pdf_size_bytes = default_pdf_bytes();
    }
    if limits.search_page_size == 0 {
        limits.search_page_size = default_page_size();
    }
    if limits.max_search_pages == 0 {
        limits.max_search_pages = default_max_pages();
    }
    limits
}

fn console_error(message: &str) {
    web_sys::console::error_1(&message.into());
}

pub fn provide_limits() {
    let limits = RwSignal::new(Limits::default());
    provide_context(limits);
    let api_base_url = crate::conf::AppConfig::load().api_base_url;
    spawn_local(async move {
        let url = format!("{api_base_url}/api/config/read");
        match gloo_net::http::Request::get(&url).send().await {
            Ok(resp) if resp.ok() => {
                match resp.json::<common::response::ResponseEnvelope<Limits>>().await {
                    Ok(envelope) if (200..300).contains(&envelope.code) => {
                        match envelope.data {
                            Some(fetched) => limits.set(normalize_limits(fetched)),
                            None => console_error(&format!("limits: empty data from {url}")),
                        }
                    }
                    Ok(envelope) => {
                        console_error(&format!("limits: code {} from {url}", envelope.code));
                    }
                    Err(e) => console_error(&format!("limits: schema mismatch from {url}: {e}")),
                }
            }
            Ok(resp) => console_error(&format!("limits: HTTP {} from {url}", resp.status())),
            Err(e) => console_error(&format!("limits: request failed ({url}): {e}")),
        }
    });
}

pub fn use_limits() -> RwSignal<Limits> {
    use_context::<RwSignal<Limits>>().expect("provide_limits must be called at App root")
}
