use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleView {
    pub id: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleListItem {
    pub id: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub member_count: u64,
}
