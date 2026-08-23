use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchRange {
    Title,
    Summary,
    AuthorName,
    Comment,
    Note,
    Tag,
    VersionNumber,
    ArticleId,
    VersionId,
    CommentId,
    AuthorId,
    Role,
}

impl SearchRange {
    pub const ALL: [SearchRange; 12] = [
        SearchRange::Title,
        SearchRange::Summary,
        SearchRange::AuthorName,
        SearchRange::Comment,
        SearchRange::Note,
        SearchRange::Tag,
        SearchRange::VersionNumber,
        SearchRange::ArticleId,
        SearchRange::VersionId,
        SearchRange::CommentId,
        SearchRange::AuthorId,
        SearchRange::Role,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SearchRange::Title => "title",
            SearchRange::Summary => "summary",
            SearchRange::AuthorName => "author_name",
            SearchRange::Comment => "comment",
            SearchRange::Note => "note",
            SearchRange::Tag => "tag",
            SearchRange::VersionNumber => "version_number",
            SearchRange::ArticleId => "article_id",
            SearchRange::VersionId => "version_id",
            SearchRange::CommentId => "comment_id",
            SearchRange::AuthorId => "author_id",
            SearchRange::Role => "role",
        }
    }

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

impl FromStr for SearchRange {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|range| range.as_str() == value)
            .ok_or_else(|| format!("unknown search range: {value}"))
    }
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
