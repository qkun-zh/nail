use seekstorm::index::Document;
use serde_json::json;

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

pub enum IndexDoc {
    Version(VersionDoc),
    Comment(CommentDoc),
}

impl IndexDoc {
    #[must_use]
    pub fn article_id(&self) -> &str {
        match self {
            IndexDoc::Version(document) => &document.article_id,
            IndexDoc::Comment(document) => &document.article_id,
        }
    }

    // Consumed by the index module in an upcoming slice.
    #[allow(dead_code)]
    pub(crate) fn to_document(&self) -> Document {
        let mut document = Document::new();
        match self {
            IndexDoc::Version(version) => {
                document.insert("version_id".to_string(), json!(version.version_id));
                document.insert("article_id".to_string(), json!(version.article_id));
                document.insert("version_number".to_string(), json!(version.version_number));
                document.insert("title".to_string(), json!(version.title));
                document.insert("summary".to_string(), json!(version.summary));
                document.insert("author_name".to_string(), json!(version.author_name));
                document.insert("author_id".to_string(), json!(version.author_id));
                document.insert("role".to_string(), json!(version.role));
                document.insert("note".to_string(), json!(version.note));
                document.insert("tags".to_string(), json!(version.tags));
                document.insert("ts".to_string(), json!(version.ts));
            }
            IndexDoc::Comment(comment) => {
                document.insert("comment_id".to_string(), json!(comment.comment_id));
                document.insert("version_id".to_string(), json!(comment.version_id));
                document.insert("article_id".to_string(), json!(comment.article_id));
                document.insert("author_name".to_string(), json!(comment.author_name));
                document.insert("author_id".to_string(), json!(comment.author_id));
                document.insert("role".to_string(), json!(comment.role));
                document.insert("content".to_string(), json!(comment.content));
                document.insert("ts".to_string(), json!(comment.ts));
            }
        }
        document
    }
}
