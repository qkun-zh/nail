use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeleteMode {
    Transfer,
    Hard,
    Soft,
}

impl DeleteMode {
    /// Wire format for query parameters; matches the serde lowercase rename.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transfer => "transfer",
            Self::Hard => "hard",
            Self::Soft => "soft",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenPurpose {
    CreateUser,
    UpdateUserEmail,
    DeleteUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DeleteQuery {
    #[serde(default)]
    pub mode: Option<DeleteMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDeleteQuery {
    #[serde(default)]
    pub mode: Option<DeleteMode>,
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub purpose: TokenPurpose,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub old_email: Option<String>,
    #[serde(default)]
    pub new_email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenRequest {
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UserUpdateRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub old_email_token: Option<String>,
    #[serde(default)]
    pub new_email_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateArticleRequest {
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub tags: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ChangeList {
    #[serde(default)]
    pub add: Vec<String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RoleUpdateRequest {
    #[serde(default)]
    pub permissions: Option<ChangeList>,
    #[serde(default)]
    pub users: Option<ChangeList>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TagUpdateRequest {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ArticleSearchParams {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub ranges: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub page: Option<u64>,
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
