use seekstorm::index::{
    AccessType, Clustering, DocumentCompression, FieldType, FrequentwordType, IndexMetaObject,
    LexicalSimilarity, NgramSet, SchemaField, StemmerType, StopwordType, TokenizerType,
};
use seekstorm::vector::Inference;

pub(crate) const FIELD_DOC_TYPE: &str = "doc_type";
pub(crate) const FIELD_VERSION_ID: &str = "version_id";
pub(crate) const FIELD_ARTICLE_ID: &str = "article_id";
pub(crate) const FIELD_COMMENT_ID: &str = "comment_id";
pub(crate) const FIELD_VERSION_NUMBER: &str = "version_number";
pub(crate) const FIELD_TITLE: &str = "title";
pub(crate) const FIELD_SUMMARY: &str = "summary";
pub(crate) const FIELD_AUTHOR_NAME: &str = "author_name";
pub(crate) const FIELD_NOTE: &str = "note";
pub(crate) const FIELD_TAGS: &str = "tags";
pub(crate) const FIELD_CONTENT: &str = "content";
pub(crate) const FIELD_TS: &str = "ts";

pub(crate) fn schema_fields() -> Vec<SchemaField> {
    vec![
        SchemaField::new(
            FIELD_DOC_TYPE.to_string(),
            false,
            false,
            false,
            FieldType::StringSet16,
            true,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            FIELD_VERSION_ID.to_string(),
            true,
            false,
            false,
            FieldType::String32,
            true,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            FIELD_ARTICLE_ID.to_string(),
            true,
            false,
            false,
            FieldType::String32,
            true,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            FIELD_COMMENT_ID.to_string(),
            true,
            false,
            false,
            FieldType::String32,
            true,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            FIELD_VERSION_NUMBER.to_string(),
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
            FIELD_TITLE.to_string(),
            true,
            true,
            false,
            FieldType::String16,
            true,
            false,
            3.0,
            false,
            false,
        ),
        SchemaField::new(
            FIELD_SUMMARY.to_string(),
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
            FIELD_AUTHOR_NAME.to_string(),
            true,
            true,
            false,
            FieldType::String16,
            true,
            false,
            2.0,
            false,
            false,
        ),
        SchemaField::new(
            FIELD_NOTE.to_string(),
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
            FIELD_TAGS.to_string(),
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
            FIELD_CONTENT.to_string(),
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
            FIELD_TS.to_string(),
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

pub(crate) fn index_meta() -> IndexMetaObject {
    IndexMetaObject {
        id: 0,
        name: "nail_articles".to_string(),
        lexical_similarity: LexicalSimilarity::Bm25f,
        tokenizer: TokenizerType::UnicodeAlphanumericFolded,
        stemmer: StemmerType::None,
        stop_words: StopwordType::None,
        frequent_words: FrequentwordType::None,
        ngram_indexing: NgramSet::SingleTerm as u8,
        document_compression: DocumentCompression::Snappy,
        access_type: AccessType::Mmap,
        spelling_correction: None,
        query_completion: None,
        clustering: Clustering::None,
        inference: Inference::None,
    }
}
