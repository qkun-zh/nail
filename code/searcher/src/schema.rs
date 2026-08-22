use std::fs;
use std::path::Path;

use seekstorm::index::{FieldType, IndexMetaObject, SchemaField};

use crate::error::Error;

pub(crate) const SCHEMA_VERSION: &str = "7";
const MARKER_FILENAME: &str = "nail_schema_version";
const META_FILENAME: &str = "meta.json";
const SCHEMA_FILENAME: &str = "schema.json";

#[allow(clippy::too_many_lines)]
pub(crate) fn fields() -> Vec<SchemaField> {
    vec![
        SchemaField::new(
            "version_id".to_string(),
            true,
            true,
            false,
            FieldType::String32,
            true,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            "article_id".to_string(),
            true,
            true,
            false,
            FieldType::String32,
            true,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            "comment_id".to_string(),
            true,
            true,
            false,
            FieldType::String32,
            true,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            "version_number".to_string(),
            true,
            true,
            false,
            FieldType::Text,
            false,
            false,
            2.0,
            false,
            false,
        ),
        SchemaField::new(
            "title".to_string(),
            true,
            true,
            false,
            FieldType::Text,
            false,
            false,
            3.0,
            false,
            false,
        ),
        SchemaField::new(
            "summary".to_string(),
            true,
            true,
            false,
            FieldType::Text,
            false,
            true,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            "author_name".to_string(),
            true,
            true,
            false,
            FieldType::Text,
            false,
            false,
            2.0,
            false,
            false,
        ),
        SchemaField::new(
            "author_id".to_string(),
            true,
            true,
            false,
            FieldType::String32,
            true,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            "role".to_string(),
            true,
            true,
            false,
            FieldType::String16,
            true,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            "note".to_string(),
            true,
            true,
            false,
            FieldType::Text,
            false,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            "tags".to_string(),
            true,
            true,
            false,
            FieldType::StringSet16,
            false,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            "content".to_string(),
            true,
            true,
            false,
            FieldType::Text,
            false,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            "ts".to_string(),
            true,
            false,
            false,
            FieldType::Timestamp,
            true,
            false,
            1.0,
            false,
            false,
        ),
    ]
}

pub(crate) fn meta() -> IndexMetaObject {
    IndexMetaObject {
        id: 0,
        name: "nail_articles".to_string(),
        lexical_similarity: seekstorm::index::LexicalSimilarity::Bm25f,
        tokenizer: seekstorm::index::TokenizerType::UnicodeAlphanumericFolded,
        stemmer: seekstorm::index::StemmerType::None,
        stop_words: seekstorm::index::StopwordType::None,
        frequent_words: seekstorm::index::FrequentwordType::None,
        ngram_indexing: seekstorm::index::NgramSet::SingleTerm as u8,
        document_compression: seekstorm::index::DocumentCompression::Snappy,
        access_type: seekstorm::index::AccessType::Mmap,
        spelling_correction: None,
        query_completion: None,
        clustering: seekstorm::index::Clustering::None,
        inference: seekstorm::vector::Inference::None,
    }
}

pub(crate) fn read_marker(index_path: &Path) -> Option<String> {
    fs::read_to_string(index_path.join(MARKER_FILENAME))
        .ok()
        .map(|content| content.trim().to_string())
}

pub(crate) fn write_marker(index_path: &Path) -> Result<(), Error> {
    fs::create_dir_all(index_path)?;
    fs::write(index_path.join(MARKER_FILENAME), SCHEMA_VERSION)?;
    Ok(())
}

pub(crate) fn validate_dir(index_path: &Path) -> Result<(), Error> {
    let meta_path = index_path.join(META_FILENAME);
    if meta_path.exists() {
        let raw = fs::read_to_string(&meta_path)?;
        if serde_json::from_str::<IndexMetaObject>(&raw).is_err() {
            return Err(Error::IndexCorrupt(format!(
                "unreadable {}",
                meta_path.display()
            )));
        }
    }
    let schema_path = index_path.join(SCHEMA_FILENAME);
    if schema_path.exists() {
        let raw = fs::read_to_string(&schema_path)?;
        if serde_json::from_str::<Vec<SchemaField>>(&raw).is_err() {
            return Err(Error::IndexCorrupt(format!(
                "unreadable {}",
                schema_path.display()
            )));
        }
    }
    Ok(())
}
