use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentView {
    pub id: String,
    pub content: String,
    pub user_id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub created_at: u64,
    pub user_name: String,
    #[serde(default)]
    pub child_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentIdView {
    pub comment_id: String,
}
