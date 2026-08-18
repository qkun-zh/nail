use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagView {
    pub id: String,
    pub name: String,
    pub article_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagListItem {
    pub id: String,
    pub name: String,
    pub article_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagListPage {
    pub tag_list: Vec<TagListItem>,
    pub has_next: bool,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagNameView {
    pub id: String,
    pub name: String,
}
