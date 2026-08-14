use serde::{Deserialize, Serialize};

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

#[cfg(test)]
#[path = "../../../test/unit/common/search/tests.rs"]
mod tests;
