use serde::{Deserialize, Serialize};
use std::fmt;

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
}

impl ResponseEnvelope<serde_json::Value> {
    pub fn err(code: u16, message: impl Into<String>) -> Self {
        Self {
            code,
            data: None,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkResponse {
    pub ok: bool,
    pub session_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub ok: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowResponse {
    pub ok: bool,
    pub email_subject: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailUpdateSendResponse {
    pub ok: bool,
    pub reason: Option<String>,
    pub old_email_subject: Option<String>,
    pub new_email_subject: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailUpdateConfirmResponse {
    pub ok: bool,
    pub session_token: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckEmailResponse {
    pub ok: bool,
    pub matches: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeregisterUserResponse {
    pub ok: bool,
    pub reason: Option<String>,
    pub email_subject: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameResponse {
    pub ok: bool,
    pub name: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorCheckResponse {
    pub ok: bool,
    pub is_author: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub field: String,
    pub label: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArticleItem {
    pub id: String,
    pub title: String,
    pub author: String,
    pub time: String,
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArticleResponse {
    pub ok: bool,
    pub page: u64,
    pub total: u64,
    pub total_pages: u64,
    pub has_more: bool,
    pub has_prev: bool,
    pub truncated: bool,
    pub article_list: Vec<SearchArticleItem>,
}

impl fmt::Display for PowResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ok: {}}}", self.ok)
    }
}

impl fmt::Display for OkResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ok: {}}}", self.ok)
    }
}

impl OkResponse {
    pub fn ok() -> Self {
        Self {
            ok: true,
            session_token: None,
        }
    }
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ok: {}, reason: {}}}", self.ok, self.reason)
    }
}

#[cfg(test)]
#[path = "../../../test/unit/common/response/tests.rs"]
mod tests;
