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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_author: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleListItem {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub author_id: String,
    pub author_name: String,
    pub tags: Vec<TagRef>,
    pub latest_version: String,
    pub latest_version_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleListPage {
    pub article_list: Vec<ArticleListItem>,
    pub page: u64,
    pub total: u64,
    pub total_pages: u64,
    pub has_next: bool,
    pub has_prev: bool,
    pub truncated: bool,
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
