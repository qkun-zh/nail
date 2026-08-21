use crate::field::SearchField;

#[derive(Debug, Clone)]
pub struct FieldHit {
    pub field: SearchField,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct VersionHit {
    pub article_id: String,
    pub version_id: String,
    pub version_number: String,
    pub title: String,
    pub author_id: String,
    pub author_name: String,
    pub article_hits: Vec<FieldHit>,
    pub version_hits: Vec<FieldHit>,
    pub version_number_hit: bool,
}

#[derive(Debug, Clone)]
pub struct CommentHit {
    pub article_id: String,
    pub version_id: String,
    pub comment_id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum DocHit {
    Version(VersionHit),
    Comment(CommentHit),
}

#[derive(Debug, Clone, Default)]
pub struct SearchOutcome {
    pub hits: Vec<DocHit>,
}
