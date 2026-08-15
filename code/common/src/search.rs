use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchRange {
    Title,
    Summary,
    #[serde(rename = "author_name")]
    AuthorName,
    Comment,
    Note,
    Tag,
    #[serde(rename = "version_number")]
    VersionNumber,
}

impl SearchRange {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            SearchRange::Title => "title",
            SearchRange::Summary => "summary",
            SearchRange::AuthorName => "author",
            SearchRange::Comment => "comment",
            SearchRange::Note => "note",
            SearchRange::Tag => "tag",
            SearchRange::VersionNumber => "version",
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
