use seekstorm::index::Document;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct VersionDoc {
    pub version_id: String,
    pub article_id: String,
    pub version_number: String,
    pub title: String,
    pub summary: String,
    pub author_name: String,
    pub author_id: String,
    pub role: String,
    pub note: String,
    pub tags: Vec<String>,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CommentDoc {
    pub comment_id: String,
    pub version_id: String,
    pub article_id: String,
    pub author_name: String,
    pub author_id: String,
    pub role: String,
    pub content: String,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchDoc {
    Version(VersionDoc),
    Comment(CommentDoc),
}

impl SearchDoc {
    #[must_use]
    pub fn article_id(&self) -> &str {
        match self {
            SearchDoc::Version(document) => &document.article_id,
            SearchDoc::Comment(document) => &document.article_id,
        }
    }

    pub(crate) fn to_document(&self) -> Result<Document, serde_json::Error> {
        let value = match self {
            SearchDoc::Version(document) => serde_json::to_value(document)?,
            SearchDoc::Comment(document) => serde_json::to_value(document)?,
        };
        let object = value.as_object().cloned().unwrap_or_default();
        Ok(object.into_iter().collect())
    }
}
