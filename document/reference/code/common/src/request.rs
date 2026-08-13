use crate::pow::Pow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRequest {
    pub pow: Pow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailUpdateSendRequest {
    pub old_email_pow: Pow,
    pub new_email_pow: Pow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailUpdateConfirmRequest {
    pub pow: Pow,
    pub old_email_token: String,
    pub new_email_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckEmailRequest {
    pub pow: Pow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeregisterUserRequest {
    pub pow: Pow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeregisterUserConfirmRequest {
    pub pow: Pow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameSetRequest {
    pub pow: Pow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub pow: Pow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifySessionRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateArticleRequest {
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub tags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteArticleRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCommentRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorCheckRequest {
    #[serde(default)]
    pub article_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub comment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBody {
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmailReadRequest {
    #[serde(default)]
    pub pow: Option<Pow>,
    #[serde(default)]
    pub old_email_pow: Option<Pow>,
    #[serde(default)]
    pub new_email_pow: Option<Pow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDeleteRequest {
    #[serde(default)]
    pub mode: Option<String>,
    pub pow: Pow,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeList {
    #[serde(default)]
    pub add: Vec<String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoleUpdateRequest {
    #[serde(default)]
    pub permissions: Option<ChangeList>,
    #[serde(default)]
    pub tags: Option<ChangeList>,
    #[serde(default)]
    pub users: Option<ChangeList>,
}

#[cfg(test)]
#[path = "../../../test/unit/common/request/tests.rs"]
mod tests;
