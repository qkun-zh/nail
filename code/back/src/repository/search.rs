use std::collections::HashSet;
use std::path::Path;

use nail_common::search::SearchRange;
use seekstorm::commit::Commit;
use seekstorm::highlighter::{Highlight, highlighter};
use seekstorm::index::{
    Close, DeleteDocuments, IndexArc, IndexDocuments, create_index, open_index,
};
use seekstorm::search::{FacetFilter, QueryRewriting, QueryType, ResultType, Search, SearchMode};

use crate::repository::graph::DbHandle;

pub(crate) mod db;
pub mod document;
pub(crate) mod query;
pub(crate) mod schema;

use db::{all_article_ids, article_ids_of_user, enrich_comment_headers};
use query::{effective_ranges, request_field_names};
use schema::{FIELD_ARTICLE_ID, FIELD_COMMENT_ID, FIELD_TS, index_meta, schema_fields};

const MAX_DOCS_PER_ARTICLE: u64 = 32;

const INDEX_SCHEMA_VERSION: &str = "3";
const SCHEMA_VERSION_FILENAME: &str = "nail_schema_version";

#[derive(Debug, Clone, Default)]
pub struct SearchRequest {
    pub query: Option<String>,
    pub ranges: Vec<SearchRange>,
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
pub struct SearchVersionOutcome {
    pub article_id: String,
    pub version_id: String,
    pub version_number: String,
    pub title: String,
    pub author_id: String,
    pub author_name: String,
    pub article_hits: Vec<SearchHitOutcome>,
    pub version_hits: Vec<SearchHitOutcome>,
    pub version_number_hit: bool,
}

#[derive(Debug, Clone)]
pub struct SearchCommentOutcome {
    pub article_id: String,
    pub version_id: String,
    pub comment_id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub article_title: String,
    pub article_author_name: String,
    pub version_number: String,
}

#[derive(Debug, Clone)]
pub enum SearchDocOutcome {
    Version(SearchVersionOutcome),
    Comment(SearchCommentOutcome),
}

#[derive(Debug, Clone)]
pub struct SearchOutcome {
    pub docs: Vec<SearchDocOutcome>,
}

#[derive(Clone)]
pub struct SearchIndex {
    index: IndexArc,
    recreated: bool,
}

impl SearchIndex {
    pub async fn open_or_create(path: &str) -> anyhow::Result<Self> {
        Self::open_or_create_with_segments(path, 11).await
    }

    pub async fn open_or_create_with_segments(
        path: &str,
        segment_number_bits: usize,
    ) -> anyhow::Result<Self> {
        let directory = Path::new(path);
        let marker = directory.join(SCHEMA_VERSION_FILENAME);

        let mut recreated = false;
        if directory.exists()
            && read_schema_version(&marker).as_deref() != Some(INDEX_SCHEMA_VERSION)
        {
            tracing::warn!(path, "search index schema mismatch; rebuilding from graph");
            std::fs::remove_dir_all(directory)?;
            recreated = true;
        }

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
                segment_number_bits,
                true,
                None,
            )
            .await
            .map_err(|error| anyhow::anyhow!("create search index {path}: {error}"))?
        };
        write_schema_version(&marker)?;
        Ok(Self { index, recreated })
    }

    pub fn was_recreated(&self) -> bool {
        self.recreated
    }

    pub async fn close(&self) {
        self.index.close().await;
    }

    pub async fn sync(&self, db: &DbHandle, article_id: &str) -> anyhow::Result<()> {
        let documents = document::build_documents(db, article_id).await?;
        let existing = self.find_document_ids_by_article(article_id).await?;
        if !existing.is_empty() {
            self.index.delete_documents(existing).await;
        }
        if !documents.is_empty() {
            self.index.index_documents(documents).await;
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
                    QueryType::Union,
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
            let ids: Vec<u64> = all
                .results
                .iter()
                .map(|result| result.doc_id as u64)
                .collect();
            if !ids.is_empty() {
                self.index.delete_documents(ids).await;
            }
        }

        let article_ids = all_article_ids(db).await?;
        let mut documents = Vec::new();
        let mut count = 0u64;
        for article_id in &article_ids {
            let built = document::build_documents(db, article_id).await?;
            count += built.len() as u64;
            documents.extend(built);
        }
        if !documents.is_empty() {
            self.index.index_documents(documents).await;
        }
        self.index.commit().await;
        Ok(count)
    }

    pub async fn read(
        &self,
        db: &DbHandle,
        request: SearchRequest,
    ) -> anyhow::Result<SearchOutcome> {
        let Some(query_string) = request.query else {
            return Ok(SearchOutcome { docs: Vec::new() });
        };
        let effective_ranges = effective_ranges(&request.ranges);
        if effective_ranges.is_empty() {
            return Ok(SearchOutcome { docs: Vec::new() });
        }
        let field_names = request_field_names(&effective_ranges);

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
                field: FIELD_TS.to_string(),
                filter: from..to,
            });
        }

        let top_k = usize::try_from((request.offset + request.limit * MAX_DOCS_PER_ARTICLE).max(1))
            .unwrap_or(usize::MAX);

        let result = self
            .index
            .search(
                query_string,
                None,
                QueryType::Union,
                SearchMode::Lexical,
                false,
                0,
                top_k,
                ResultType::Topk,
                true,
                field_names.clone(),
                Vec::new(),
                facet_filter,
                Vec::new(),
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
        let query_terms = result.query_terms;

        let mut version_hits = Vec::new();
        let mut comment_hits = Vec::new();
        let index = self.index.read().await;
        let empty_fields = HashSet::new();
        let empty_distance_fields = Vec::new();
        for hit in &result.results {
            let document = index
                .get_document(
                    hit.doc_id,
                    false,
                    &highlights,
                    &empty_fields,
                    &empty_distance_fields,
                )
                .await
                .map_err(|error| anyhow::anyhow!("fetch search document failed: {error}"))?;
            if document.contains_key(FIELD_COMMENT_ID) {
                let comment = document::read_comment_outcome(&document);
                comment_hits.push(comment);
            } else {
                let version =
                    document::read_version_outcome(&document, &effective_ranges, &query_terms);
                version_hits.push(version);
            }
        }

        let mut enriched = comment_hits;
        enrich_comment_headers(db, &mut enriched).await?;

        let mut docs = Vec::with_capacity(version_hits.len() + enriched.len());
        docs.extend(version_hits.into_iter().map(SearchDocOutcome::Version));
        docs.extend(enriched.into_iter().map(SearchDocOutcome::Comment));

        Ok(SearchOutcome { docs })
    }

    async fn find_document_ids_by_article(&self, article_id: &str) -> anyhow::Result<Vec<u64>> {
        let result = self
            .index
            .search(
                String::new(),
                None,
                QueryType::Union,
                SearchMode::Lexical,
                true,
                0,
                u32::MAX as usize,
                ResultType::TopkCount,
                true,
                Vec::new(),
                Vec::new(),
                vec![FacetFilter::String32 {
                    field: FIELD_ARTICLE_ID.to_string(),
                    filter: vec![article_id.to_string()],
                }],
                Vec::new(),
                QueryRewriting::SearchOnly,
            )
            .await;
        Ok(result
            .results
            .iter()
            .map(|result| result.doc_id as u64)
            .collect())
    }
}

fn read_schema_version(marker: &Path) -> Option<String> {
    std::fs::read_to_string(marker)
        .ok()
        .map(|content| content.trim().to_string())
}

fn write_schema_version(marker: &Path) -> anyhow::Result<()> {
    std::fs::write(marker, INDEX_SCHEMA_VERSION).map_err(|error| {
        anyhow::anyhow!("write search schema marker {}: {error}", marker.display())
    })
}
