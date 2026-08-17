use serde::{Deserialize, Serialize};

use crate::tag::TagRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleView {
    pub id: String,
    pub author_id: String,
    pub author_name: String,
    pub title: String,
    pub summary: String,
    pub created_at: u64,
    pub tags: Vec<TagRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleIdView {
    pub article_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateArticleView {
    pub article_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleListItem {
    pub id: String,
    pub title: String,
    pub created_at: u64,
}
