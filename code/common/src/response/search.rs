use serde::{Deserialize, Serialize};

use crate::search::SearchRange;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    pub field: SearchRange,
    pub label: String,
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchArticleItem {
    pub article_id: String,
    pub title: String,
    pub author_id: String,
    pub author_name: String,
    pub time: String,
    pub article_hits: Vec<SearchHit>,
    pub versions: Vec<SearchVersionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchVersionItem {
    pub version_id: String,
    pub version_number: String,
    pub time: String,
    pub version_hits: Vec<SearchHit>,
    pub comments: Vec<SearchCommentItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCommentItem {
    pub comment_id: String,
    pub author_id: String,
    pub author_name: String,
    pub time: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchPage {
    pub article_list: Vec<SearchArticleItem>,
    pub page: u64,
    pub has_next: bool,
}
