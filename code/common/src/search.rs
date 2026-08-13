use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleSearchParams {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub ranges: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub from: Option<u64>,
    #[serde(default)]
    pub to: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub page: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchRange {
    Title,
    Summary,
    Author,
    Comment,
    Note,
    Tag,
}

impl SearchRange {
    pub fn label(self) -> &'static str {
        match self {
            SearchRange::Title => "Title",
            SearchRange::Summary => "Summary",
            SearchRange::Author => "Author",
            SearchRange::Comment => "Comment",
            SearchRange::Note => "Version note",
            SearchRange::Tag => "Tag",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchSortField {
    Time,
    Title,
    Author,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchSortDirection {
    Asc,
    Desc,
}

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
    pub has_more: bool,
    pub has_prev: bool,
    pub truncated: bool,
}

#[cfg(test)]
#[path = "../../../test/unit/common/search/tests.rs"]
mod tests;
