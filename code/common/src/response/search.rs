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
    pub id: String,
    pub title: String,
    pub author: String,
    pub time: String,
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchPage {
    pub article_list: Vec<SearchArticleItem>,
    pub total: u64,
    pub page: u64,
    pub total_pages: u64,
    pub has_next: bool,
    pub has_prev: bool,
    pub truncated: bool,
}
