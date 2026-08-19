use serde::{Deserialize, Serialize};

pub mod article;
pub mod comment;
pub mod content;
pub mod email;
pub mod role;
pub mod search;
pub mod session;
pub mod tag;
pub mod user;
pub mod version;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseEnvelope<T> {
    pub code: u16,
    pub data: Option<T>,
    pub message: String,
}

impl<T> ResponseEnvelope<T> {
    pub fn ok(code: u16, data: T, message: impl Into<String>) -> Self {
        Self {
            code,
            data: Some(data),
            message: message.into(),
        }
    }

    pub fn err(code: u16, message: impl Into<String>) -> Self {
        Self {
            code,
            data: None,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeLimits {
    pub max_tags_per_article: u64,
    pub max_comment_body_chars: u64,
    pub max_version_note_chars: u64,
    pub max_title_chars: u64,
    pub max_summary_chars: u64,
    pub max_pdf_size_bytes: u64,
    pub max_text_field_bytes: u64,
    pub download_token_ttl_seconds: u64,
    pub search_page_size: u64,
    pub max_search_pages: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListPage<T> {
    pub items: Vec<T>,
    pub has_next: bool,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmptyView {}

#[cfg(test)]
#[path = "../../../test/unit/common/response/tests.rs"]
mod tests;
