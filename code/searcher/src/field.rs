#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchField {
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

impl SearchField {
    pub(crate) const fn as_engine_field(self) -> &'static str {
        match self {
            SearchField::Title => "title",
            SearchField::Summary => "summary",
            SearchField::AuthorName => "author_name",
            SearchField::Comment => "content",
            SearchField::Note => "note",
            SearchField::Tag => "tags",
            SearchField::VersionNumber => "version_number",
            SearchField::ArticleId => "article_id",
            SearchField::VersionId => "version_id",
            SearchField::CommentId => "comment_id",
            SearchField::AuthorId => "author_id",
            SearchField::Role => "role",
        }
    }
}
