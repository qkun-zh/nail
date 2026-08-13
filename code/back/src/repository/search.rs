use std::collections::HashSet;
use std::path::Path;

use agdb::{DbError, QueryBuilder};
use nail_common::search::{SearchRange, SearchSortDirection, SearchSortField};
use seekstorm::commit::Commit;
use seekstorm::highlighter::{Highlight, highlighter};
use seekstorm::index::{
    AccessType, Close, Clustering, DeleteDocuments, DocumentCompression, FieldType, FileType,
    FrequentwordType, IndexArc, IndexDocument, IndexDocuments, IndexMetaObject,
    LexicalSimilarity, NgramSet, SchemaField, StemmerType, StopwordType, TokenizerType,
    UpdateDocument, create_index, open_index,
};
use seekstorm::search::{
    FacetFilter, FacetValue, QueryRewriting, QueryType, ResultSort, ResultType, Search, SearchMode,
    SortOrder,
};
use seekstorm::vector::Inference;

use crate::repository::graph::{DbHandle, read_rows_sync, resolve_node_id_sync};
use crate::repository::schema::{
    EDGE_USER_TO_ARTICLE, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_USER, IdRow, KEY_TYPE,
};

pub mod document;

const FIELD_ID: &str = "id";
const FIELD_TITLE: &str = "title";
const FIELD_SUMMARY: &str = "summary";
const FIELD_AUTHOR: &str = "author";
const FIELD_NOTE: &str = "note";
const FIELD_TAG: &str = "tag";
const FIELD_COMMENT: &str = "comment";
const FIELD_TS: &str = "ts";

#[derive(Debug, Clone)]
pub struct SearchSort {
    pub field: SearchSortField,
    pub direction: SearchSortDirection,
}

#[derive(Debug, Clone, Default)]
pub struct SearchRequest {
    pub query: Option<String>,
    pub ranges: Vec<SearchRange>,
    pub sort: Vec<SearchSort>,
    pub from_seconds: Option<u64>,
    pub to_seconds: Option<u64>,
    pub offset: u64,
    pub limit: u64,
}

#[derive(Debug, Clone)]
pub struct SearchHitOutcome {
    pub range: SearchRange,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct SearchArticleOutcome {
    pub id: String,
    pub title: String,
    pub author: String,
    pub timestamp_seconds: i64,
    pub hits: Vec<SearchHitOutcome>,
}

#[derive(Debug, Clone)]
pub struct SearchOutcome {
    pub articles: Vec<SearchArticleOutcome>,
    pub total: u64,
}

#[derive(Clone)]
pub struct SearchIndex {
    index: IndexArc,
}

impl SearchIndex {
    pub async fn open_or_create(path: &str) -> anyhow::Result<Self> {
        let directory = Path::new(path);
        let index = if directory.exists() {
            open_index(directory)
                .await
                .map_err(|error| anyhow::anyhow!("open search index {path}: {error}"))?
        } else {
            std::fs::create_dir_all(directory)?;
            create_index(
                directory,
                index_meta(),
                &schema_fields(),
                &Vec::new(),
                11,
                true,
                None,
            )
            .await
            .map_err(|error| anyhow::anyhow!("create search index {path}: {error}"))?
        };
        Ok(Self { index })
    }

    pub async fn close(&self) {
        self.index.close().await;
    }

    pub async fn sync(&self, db: &DbHandle, article_id: &str) -> anyhow::Result<()> {
        match document::build_document(db, article_id).await? {
            Some(document) => match self.find_document_id(article_id).await? {
                Some(document_id) => self.index.update_document((document_id, document)).await,
                None => self.index.index_document(document, FileType::None).await,
            },
            None => {
                if let Some(document_id) = self.find_document_id(article_id).await? {
                    self.index.delete_documents(vec![document_id]).await;
                }
            }
        }
        self.index.commit().await;
        Ok(())
    }

    pub async fn sync_user(&self, db: &DbHandle, user_id: &str) -> anyhow::Result<u64> {
        let article_ids = article_ids_of_user(db, user_id).await?;
        let mut synced = 0u64;
        for article_id in &article_ids {
            if self.sync(db, article_id).await.is_ok() {
                synced += 1;
            }
        }
        Ok(synced)
    }

    pub async fn sync_all(&self, db: &DbHandle) -> anyhow::Result<u64> {
        let live = self.index.read().await.current_doc_count().await;
        if live > 0 {
            let all = self
                .index
                .search(
                    String::new(),
                    None,
                    QueryType::Intersection,
                    SearchMode::Lexical,
                    true,
                    0,
                    live,
                    ResultType::Topk,
                    false,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    QueryRewriting::SearchOnly,
                )
                .await;
            let ids: Vec<u64> = all.results.iter().map(|result| result.doc_id as u64).collect();
            if !ids.is_empty() {
                self.index.delete_documents(ids).await;
            }
        }

        let article_ids = all_article_ids(db).await?;
        let mut documents = Vec::with_capacity(article_ids.len());
        let mut count = 0u64;
        for article_id in &article_ids {
            let Some(document) = document::build_document(db, article_id).await? else {
                continue;
            };
            documents.push(document);
            count += 1;
        }
        if !documents.is_empty() {
            self.index.index_documents(documents).await;
        }
        self.index.commit().await;
        Ok(count)
    }

    pub async fn read(&self, request: SearchRequest) -> anyhow::Result<SearchOutcome> {
        let enable_empty_query = request.query.is_none();
        let query_string = request.query.unwrap_or_default();
        let effective_ranges = effective_ranges(&request.ranges);
        let field_names = request_field_names(&effective_ranges);

        let mut facet_filter: Vec<FacetFilter> = Vec::new();
        if request.from_seconds.is_some() || request.to_seconds.is_some() {
            let from = request.from_seconds.unwrap_or(0).min(i64::MAX as u64) as i64;
            let to = request
                .to_seconds
                .unwrap_or(u64::MAX)
                .saturating_add(1)
                .min(i64::MAX as u64) as i64;
            facet_filter.push(FacetFilter::Timestamp {
                field: FIELD_TS.to_string(),
                filter: from..to,
            });
        }

        let result_sort: Vec<ResultSort> = request
            .sort
            .iter()
            .map(|sort| ResultSort {
                field: sort_field_name(sort.field).to_string(),
                order: match sort.direction {
                    SearchSortDirection::Desc => SortOrder::Descending,
                    SearchSortDirection::Asc => SortOrder::Ascending,
                },
                base: FacetValue::None,
            })
            .collect();

        let result = self
            .index
            .search(
                query_string,
                None,
                QueryType::Intersection,
                SearchMode::Lexical,
                enable_empty_query,
                request.offset as usize,
                request.limit as usize,
                ResultType::TopkCount,
                true,
                field_names.clone(),
                Vec::new(),
                facet_filter,
                result_sort,
                QueryRewriting::SearchOnly,
            )
            .await;

        let highlight_fields: Vec<Highlight> = field_names
            .iter()
            .map(|field| Highlight {
                field: field.clone(),
                name: format!("{field}_highlight"),
                fragment_number: 0,
                fragment_size: 4096,
                highlight_markup: true,
                pre_tags: "<mark>".to_string(),
                post_tags: "</mark>".to_string(),
            })
            .collect();
        let query_terms = result.query_terms.clone();
        let highlights = if result.results.is_empty() {
            None
        } else {
            Some(highlighter(&self.index, highlight_fields, query_terms).await)
        };

        let mut articles = Vec::with_capacity(result.results.len());
        for hit in &result.results {
            let document = self
                .index
                .read()
                .await
                .get_document(hit.doc_id, true, &highlights, &HashSet::new(), &Vec::new())
                .await
                .map_err(|error| anyhow::anyhow!("fetch search document failed: {error}"))?;
            let id = document::read_string_field(&document, FIELD_ID);
            let title = document::read_string_field(&document, FIELD_TITLE);
            let author = document::read_string_field(&document, FIELD_AUTHOR);
            let timestamp_seconds = document::read_i64_field(&document, FIELD_TS);

            let mut hits = Vec::new();
            for range in &effective_ranges {
                let snippet = document::read_string_field(&document, &highlight_name(range));
                if snippet.contains("<mark>") {
                    hits.push(SearchHitOutcome {
                        range: *range,
                        snippet,
                    });
                }
            }
            articles.push(SearchArticleOutcome {
                id,
                title,
                author,
                timestamp_seconds,
                hits,
            });
        }

        Ok(SearchOutcome {
            articles,
            total: result.result_count_total as u64,
        })
    }

    async fn find_document_id(&self, article_id: &str) -> anyhow::Result<Option<u64>> {
        let result = self
            .index
            .search(
                String::new(),
                None,
                QueryType::Intersection,
                SearchMode::Lexical,
                true,
                0,
                1,
                ResultType::TopkCount,
                true,
                Vec::new(),
                Vec::new(),
                vec![FacetFilter::String16 {
                    field: FIELD_ID.to_string(),
                    filter: vec![article_id.to_string()],
                }],
                Vec::new(),
                QueryRewriting::SearchOnly,
            )
            .await;
        Ok(result.results.first().map(|result| result.doc_id as u64))
    }
}

fn effective_ranges(ranges: &[SearchRange]) -> Vec<SearchRange> {
    if ranges.is_empty() {
        return [
            SearchRange::Title,
            SearchRange::Summary,
            SearchRange::Author,
            SearchRange::Note,
            SearchRange::Tag,
            SearchRange::Comment,
        ]
        .to_vec();
    }
    ranges.to_vec()
}

fn request_field_names(ranges: &[SearchRange]) -> Vec<String> {
    effective_ranges(ranges)
        .iter()
        .map(|range| range_field_name(*range).to_string())
        .collect()
}

fn range_field_name(range: SearchRange) -> &'static str {
    match range {
        SearchRange::Title => FIELD_TITLE,
        SearchRange::Summary => FIELD_SUMMARY,
        SearchRange::Author => FIELD_AUTHOR,
        SearchRange::Comment => FIELD_COMMENT,
        SearchRange::Note => FIELD_NOTE,
        SearchRange::Tag => FIELD_TAG,
    }
}

fn highlight_name(range: &SearchRange) -> String {
    format!("{}_highlight", range_field_name(*range))
}

fn sort_field_name(field: SearchSortField) -> &'static str {
    match field {
        SearchSortField::Time => FIELD_TS,
        SearchSortField::Title => FIELD_TITLE,
        SearchSortField::Author => FIELD_AUTHOR,
    }
}

async fn article_ids_of_user(db: &DbHandle, user_id: &str) -> Result<Vec<String>, DbError> {
    let guard = db.read().await;
    let Some(user) = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)? else {
        return Ok(Vec::new());
    };
    let edges = guard.exec(
        QueryBuilder::search()
            .from(user)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_TO_ARTICLE)
            .query(),
    )?;
    let mut ids = Vec::with_capacity(edges.elements.len());
    for edge in &edges.elements {
        if let Some(row) = read_rows_sync::<IdRow>(&guard, &[edge.to])?
            .into_iter()
            .next()
        {
            ids.push(row.id);
        }
    }
    Ok(ids)
}

async fn all_article_ids(db: &DbHandle) -> Result<Vec<String>, DbError> {
    let guard = db.read().await;
    let all = guard.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_ARTICLE)
            .query(),
    )?;
    let mut ids = Vec::with_capacity(all.elements.len());
    for element in &all.elements {
        if let Some(row) = read_rows_sync::<IdRow>(&guard, &[element.id])?
            .into_iter()
            .next()
        {
            ids.push(row.id);
        }
    }
    Ok(ids)
}

fn schema_fields() -> Vec<SchemaField> {
    vec![
        SchemaField::new(FIELD_ID.to_string(), true, false, false, FieldType::String16, true, false, 1.0, false, false),
        SchemaField::new(FIELD_TITLE.to_string(), true, true, false, FieldType::String16, true, false, 3.0, false, false),
        SchemaField::new(FIELD_SUMMARY.to_string(), true, true, false, FieldType::Text, false, true, 1.0, false, false),
        SchemaField::new(FIELD_AUTHOR.to_string(), true, true, false, FieldType::String16, true, false, 2.0, false, false),
        SchemaField::new(FIELD_NOTE.to_string(), true, true, false, FieldType::Text, false, false, 1.0, false, false),
        SchemaField::new(FIELD_TAG.to_string(), true, true, false, FieldType::Json, false, false, 1.0, false, false),
        SchemaField::new(FIELD_COMMENT.to_string(), true, true, false, FieldType::Json, false, false, 1.0, false, false),
        SchemaField::new(FIELD_TS.to_string(), true, false, false, FieldType::Timestamp, true, false, 1.0, false, false),
    ]
}

fn index_meta() -> IndexMetaObject {
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
