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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentListPage {
    pub comments: Vec<CommentView>,
    pub has_next: bool,
    pub total: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_author: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentIdView {
    pub comment_id: String,
}
