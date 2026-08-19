use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagListItem {
    pub id: String,
    pub name: String,
    pub article_count: u64,
}
