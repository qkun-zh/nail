use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        match value {
            "title" => Ok(SearchRange::Title),
            "summary" => Ok(SearchRange::Summary),
            "author_name" => Ok(SearchRange::AuthorName),
            "comment" => Ok(SearchRange::Comment),
            "note" => Ok(SearchRange::Note),
            "tag" => Ok(SearchRange::Tag),
            "version_number" => Ok(SearchRange::VersionNumber),
            "article_id" => Ok(SearchRange::ArticleId),
            "version_id" => Ok(SearchRange::VersionId),
            "comment_id" => Ok(SearchRange::CommentId),
            "author_id" => Ok(SearchRange::AuthorId),
            "role" => Ok(SearchRange::Role),
            _ => Err(format!("unknown search range: {value}")),
        }
    }
}

impl Serialize for SearchRange {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SearchRange {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SearchRangeVisitor;

        impl Visitor<'_> for SearchRangeVisitor {
            type Value = SearchRange;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a search range string")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                value.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(SearchRangeVisitor)
    }
}

#[cfg(test)]
#[path = "../../../test/unit/common/search/tests.rs"]
mod tests;
