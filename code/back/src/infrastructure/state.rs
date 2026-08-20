use std::sync::Arc;

use crate::infrastructure::authorizer::Authorizer;
use crate::infrastructure::config::AppConfig;
use crate::repository::graph::DbHandle;
use crate::repository::search::SearchIndex;

#[derive(Clone)]
pub struct AppState {
    pub authorizer: Authorizer,
    pub database: DbHandle,
    pub searcher: SearchIndex,
    pub cache: cache::Caches,
    pub configurator: Configurator,
    pub emailer: emailer::Emailer,
}

#[derive(Clone)]
pub struct Configurator(Arc<AppConfig>);

impl Configurator {
    pub fn new(config: AppConfig) -> Self {
        Self(Arc::new(config))
    }

    pub fn listen_addr(&self) -> &str {
        &self.0.server.listen_addr
    }

    pub fn pdf_storage_path(&self) -> &str {
        &self.0.server.pdf_storage_path
    }

    pub fn pow_difficulty_iterations(&self) -> u64 {
        self.0.server.pow_difficulty_iterations
    }

    pub fn download_token_ttl_seconds(&self) -> u64 {
        self.0.cache.download_ttl_seconds
    }

    pub fn max_pdf_size_bytes(&self) -> u64 {
        self.0.server.max_pdf_size_bytes
    }

    pub fn max_tags_per_article(&self) -> usize {
        self.0.server.max_tags_per_article
    }

    pub fn max_title_chars(&self) -> u64 {
        self.0.server.max_title_chars
    }

    pub fn max_summary_chars(&self) -> u64 {
        self.0.server.max_summary_chars
    }

    pub fn max_comment_body_chars(&self) -> u64 {
        self.0.server.max_comment_body_chars
    }

    pub fn max_version_note_chars(&self) -> u64 {
        self.0.server.max_version_note_chars
    }

    pub fn max_text_field_bytes(&self) -> u64 {
        self.0.server.max_text_field_bytes
    }

    pub fn max_search_query_chars(&self) -> u64 {
        self.0.server.max_search_query_chars
    }

    pub fn search_page_size(&self) -> u64 {
        self.0.server.search_page_size
    }

    pub fn max_search_pages(&self) -> u64 {
        self.0.server.max_search_pages
    }

    pub fn max_request_body_bytes(&self) -> u64 {
        self.0.server.max_request_body_bytes()
    }

    pub fn email_allowed_domains(&self) -> &[String] {
        &self.0.email_allowed_domains
    }
}
