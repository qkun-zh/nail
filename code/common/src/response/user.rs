use serde::{Deserialize, Serialize};

use super::article::ArticleListItem;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<RoleRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub articles: Option<Vec<ArticleListItem>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserNameView {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserIdView {
    pub user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserListItem {
    pub id: String,
    pub name: String,
    pub roles: Vec<RoleRef>,
}
