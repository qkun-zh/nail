use std::collections::HashSet;

use seekstorm::highlighter::{Highlight, highlighter};
use seekstorm::index::Document;
use seekstorm::search::{FacetFilter, QueryRewriting, QueryType, ResultType, Search, SearchMode};

use crate::error::Error;
use crate::field::SearchField;
use crate::index::Searcher;
use crate::outcome::{CommentHit, DocHit, FieldHit, SearchOutcome, VersionHit};

const MAX_DOCS_PER_ARTICLE: usize = 32;
const HIGHLIGHT_FRAGMENT_SIZE: usize = 4096;

// RF1: the fetch flag must mirror the search flag, otherwise a read racing
// an in-flight write fails on uncommitted documents.
const REALTIME: bool = true;

#[derive(Debug, Clone, Default)]
pub struct SearchRequest {
    pub query: Option<String>,
    pub fields: Vec<SearchField>,
    pub from_seconds: Option<u64>,
    pub to_seconds: Option<u64>,
    pub offset: usize,
    pub limit: usize,
}

impl Searcher {
    /// Executes a search and returns raw document hits.
    ///
    /// The hit window is sized `offset + limit * MAX_DOCS_PER_ARTICLE` so the
    /// caller can group hits by article afterwards; grouping and pagination
    /// policy live outside this crate.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Engine`] when a matched document cannot be fetched.
    pub async fn read(&self, request: SearchRequest) -> Result<SearchOutcome, Error> {
        let Some(query) = request.query.filter(|query| !query.trim().is_empty()) else {
            return Ok(SearchOutcome { hits: Vec::new() });
        };
        if request.fields.is_empty() {
            return Ok(SearchOutcome { hits: Vec::new() });
        }
        let field_names: Vec<String> = request
            .fields
            .iter()
            .map(|field| field.as_engine_field().to_string())
            .collect();

        let mut facet_filter: Vec<FacetFilter> = Vec::new();
        if request.from_seconds.is_some() || request.to_seconds.is_some() {
            let from =
                i64::try_from(request.from_seconds.unwrap_or(0).min(i64::MAX as u64)).unwrap_or(0);
            let to = i64::try_from(
                request
                    .to_seconds
                    .unwrap_or(u64::MAX)
                    .saturating_add(1)
                    .min(i64::MAX as u64),
            )
            .unwrap_or(i64::MAX);
            facet_filter.push(FacetFilter::Timestamp {
                field: "ts".to_string(),
                filter: from..to,
            });
        }

        let top_k = request
            .offset
            .saturating_add(request.limit.saturating_mul(MAX_DOCS_PER_ARTICLE))
            .max(1);
        let result = self
            .index
            .search(
                query,
                None,
                QueryType::Union,
                SearchMode::Lexical,
                false,
                0,
                top_k,
                ResultType::Topk,
                REALTIME,
                field_names.clone(),
                Vec::new(),
                facet_filter,
                Vec::new(),
                QueryRewriting::SearchOnly,
            )
            .await;

        let query_terms = result.query_terms.clone();
        let highlights = if result.results.is_empty() {
            None
        } else {
            let fields: Vec<Highlight> = field_names
                .iter()
                .map(|field| Highlight {
                    field: field.clone(),
                    name: format!("{field}_highlight"),
                    fragment_number: 0,
                    fragment_size: HIGHLIGHT_FRAGMENT_SIZE,
                    highlight_markup: true,
                    pre_tags: "<mark>".to_string(),
                    post_tags: "</mark>".to_string(),
                })
                .collect();
            Some(highlighter(&self.index, fields, query_terms.clone()).await)
        };

        let guard = self.index.read().await;
        let empty_fields = HashSet::new();
        let empty_distance_fields = Vec::new();
        let mut hits = Vec::with_capacity(result.results.len());
        for hit in &result.results {
            let document = guard
                .get_document(
                    hit.doc_id,
                    REALTIME,
                    &highlights,
                    &empty_fields,
                    &empty_distance_fields,
                )
                .await
                .map_err(|error| Error::Engine(format!("fetch search document failed: {error}")))?;
            if document.contains_key("comment_id") {
                hits.push(DocHit::Comment(comment_hit(&document)));
            } else {
                hits.push(DocHit::Version(version_hit(
                    &document,
                    &request.fields,
                    &query_terms,
                )));
            }
        }
        Ok(SearchOutcome { hits })
    }
}

fn highlight_name(field: &str) -> String {
    format!("{field}_highlight")
}

fn read_string_field(document: &Document, field: &str) -> String {
    document
        .get(field)
        .map(|value| match value {
            serde_json::Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

fn read_highlighted_or_raw(document: &Document, field: &str) -> String {
    let highlighted = read_string_field(document, &highlight_name(field));
    if highlighted.is_empty() {
        read_string_field(document, field)
    } else {
        highlighted
    }
}

fn field_hit(document: &Document, field: &str, query_terms: &[String]) -> bool {
    let folded = read_string_field(document, field).to_lowercase();
    query_terms.iter().any(|term| folded.contains(term))
}

fn version_hit(document: &Document, fields: &[SearchField], query_terms: &[String]) -> VersionHit {
    let mut article_hits = Vec::new();
    let mut version_hits = Vec::new();
    let mut version_number_hit = false;
    for field in fields {
        let engine_field = field.as_engine_field();
        match field {
            SearchField::Summary | SearchField::Tag => {
                if field_hit(document, engine_field, query_terms) {
                    article_hits.push(FieldHit {
                        field: *field,
                        snippet: read_highlighted_or_raw(document, engine_field),
                    });
                }
            }
            SearchField::Note => {
                if field_hit(document, engine_field, query_terms) {
                    version_hits.push(FieldHit {
                        field: *field,
                        snippet: read_highlighted_or_raw(document, engine_field),
                    });
                }
            }
            SearchField::VersionNumber => {
                version_number_hit = field_hit(document, engine_field, query_terms);
            }
            _ => {}
        }
    }
    VersionHit {
        article_id: read_string_field(document, "article_id"),
        version_id: read_string_field(document, "version_id"),
        version_number: read_highlighted_or_raw(document, "version_number"),
        title: read_highlighted_or_raw(document, "title"),
        author_id: read_string_field(document, "author_id"),
        author_name: read_highlighted_or_raw(document, "author_name"),
        article_hits,
        version_hits,
        version_number_hit,
    }
}

fn comment_hit(document: &Document) -> CommentHit {
    CommentHit {
        article_id: read_string_field(document, "article_id"),
        version_id: read_string_field(document, "version_id"),
        comment_id: read_string_field(document, "comment_id"),
        author_id: read_string_field(document, "author_id"),
        author_name: read_highlighted_or_raw(document, "author_name"),
        content: read_highlighted_or_raw(document, "content"),
    }
}
