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
    #[serde(rename = "article_id")]
    ArticleId,
    #[serde(rename = "version_id")]
    VersionId,
    #[serde(rename = "comment_id")]
    CommentId,
    #[serde(rename = "author_id")]
    AuthorId,
    Role,
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
            SearchRange::ArticleId => "article id",
            SearchRange::VersionId => "version id",
            SearchRange::CommentId => "comment id",
            SearchRange::AuthorId => "author id",
            SearchRange::Role => "role",
        }
    }
}

#[cfg(test)]
#[path = "../../../test/unit/common/search/tests.rs"]
mod tests;
