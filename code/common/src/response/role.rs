use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleView {
    pub name: String,
    pub permissions: Vec<String>,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleListItem {
    pub name: String,
    pub permissions: Vec<String>,
    pub member_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleListPage {
    pub role_list: Vec<RoleListItem>,
    pub has_next: bool,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleNameView {
    pub name: String,
}
