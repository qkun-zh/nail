use crate::pow::Pow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeleteMode {
    Transfer,
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailReadIntent {
    Authenticate,
    ChangeEmail,
    Deregister,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteBody {
    #[serde(default)]
    pub mode: Option<DeleteMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDeleteRequest {
    #[serde(default)]
    pub mode: Option<DeleteMode>,
    pub pow: Pow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EmailReadRequest {
    #[serde(default)]
    pub pow: Option<Pow>,
    #[serde(default)]
    pub old_email_pow: Option<Pow>,
    #[serde(default)]
    pub new_email_pow: Option<Pow>,
}

impl EmailReadRequest {
    pub fn has_consistent_email_pow_pair(&self) -> bool {
        self.old_email_pow.is_some() == self.new_email_pow.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenRequest {
    pub pow: Pow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub pow: Pow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameSetRequest {
    pub pow: Pow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeregisterUserRequest {
    pub pow: Pow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeregisterUserConfirmRequest {
    pub pow: Pow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UserUpdateRequest {
    #[serde(default)]
    pub pow: Option<Pow>,
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
    pub tags: Option<ChangeList>,
    #[serde(default)]
    pub users: Option<ChangeList>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleSearchParams {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub ranges: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub from: Option<u64>,
    #[serde(default)]
    pub to: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub page: Option<u64>,
}

#[cfg(test)]
#[path = "../../../test/unit/common/request/tests.rs"]
mod tests;
