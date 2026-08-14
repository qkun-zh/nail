use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionView {
    pub id: String,
    pub version: String,
    pub created_at: u64,
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_author: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionListItem {
    pub id: String,
    pub version: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionListPage {
    pub version_list: Vec<VersionListItem>,
    pub page: u64,
    pub total: u64,
    pub has_next: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionIdView {
    pub version_id: String,
}
