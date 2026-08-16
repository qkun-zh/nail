use std::collections::{HashMap, HashSet};
use std::path::Path;

use agdb::{DbError, QueryBuilder};
use nail_common::search::SearchRange;
use seekstorm::commit::Commit;
use seekstorm::highlighter::{Highlight, highlighter};
use seekstorm::index::{
    AccessType, Close, Clustering, DeleteDocuments, DocumentCompression, FieldType,
    FrequentwordType, IndexArc, IndexDocuments, IndexMetaObject, LexicalSimilarity, NgramSet,
    SchemaField, StemmerType, StopwordType, TokenizerType, create_index, open_index,
};
use seekstorm::search::{FacetFilter, QueryRewriting, QueryType, ResultType, Search, SearchMode};
use seekstorm::vector::Inference;

use crate::repository::graph::{DbHandle, read_rows_sync, resolve_node_id_sync};
use crate::repository::schema::{
    ArticleRow, EDGE_ARTICLE_HOLD_VERSION, EDGE_COMMENT_ATTACH_VERSION, EDGE_USER_AUTHOR_ARTICLE,
    EDGE_USER_AUTHOR_COMMENT, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, IdRow,
    KEY_TYPE, UserRow, VersionRow,
};

pub mod document;

const FIELD_DOC_TYPE: &str = "doc_type";
const FIELD_VERSION_ID: &str = "version_id";
const FIELD_ARTICLE_ID: &str = "article_id";
const FIELD_COMMENT_ID: &str = "comment_id";
const FIELD_VERSION_NUMBER: &str = "version_number";
const FIELD_TITLE: &str = "title";
const FIELD_SUMMARY: &str = "summary";
const FIELD_AUTHOR_NAME: &str = "author_name";
const FIELD_NOTE: &str = "note";
const FIELD_TAGS: &str = "tags";
const FIELD_CONTENT: &str = "content";
const FIELD_TS: &str = "ts";

const MAX_DOCS_PER_ARTICLE: u64 = 32;

const INDEX_SCHEMA_VERSION: &str = "2";
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
        for hit in &result.results {
            let document = self
                .index
                .read()
                .await
                .get_document(hit.doc_id, false, &highlights, &HashSet::new(), &Vec::new())
                .await
                .map_err(|error| anyhow::anyhow!("fetch search document failed: {error}"))?;
            if document.contains_key(FIELD_COMMENT_ID) {
                let comment = document::read_comment_outcome(&document, &effective_ranges);
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

fn effective_ranges(ranges: &[SearchRange]) -> Vec<SearchRange> {
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
        SearchRange::AuthorName => FIELD_AUTHOR_NAME,
        SearchRange::Comment => FIELD_CONTENT,
        SearchRange::Note => FIELD_NOTE,
        SearchRange::Tag => FIELD_TAGS,
        SearchRange::VersionNumber => FIELD_VERSION_NUMBER,
    }
}

async fn enrich_comment_headers(
    db: &DbHandle,
    comments: &mut [SearchCommentOutcome],
) -> anyhow::Result<()> {
    if comments.is_empty() {
        return Ok(());
    }
    let guard = db.read().await;

    let article_ids: HashSet<String> = comments.iter().map(|c| c.article_id.clone()).collect();
    let version_ids: HashSet<String> = comments.iter().map(|c| c.version_id.clone()).collect();

    let mut article_by_id: HashMap<String, agdb::DbId> = HashMap::new();
    for id in &article_ids {
        if let Some(node) = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, id)? {
            article_by_id.insert(id.clone(), node);
        }
    }
    let mut version_by_id: HashMap<String, agdb::DbId> = HashMap::new();
    for id in &version_ids {
        if let Some(node) = resolve_node_id_sync(&guard, ENTITY_TYPE_VERSION, id)? {
            version_by_id.insert(id.clone(), node);
        }
    }

    let article_nodes: Vec<agdb::DbId> = article_by_id.values().copied().collect();
    let title_by_node: HashMap<agdb::DbId, String> =
        read_rows_sync::<ArticleRow>(&guard, &article_nodes)?
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row.title)))
            .collect();

    let version_nodes: Vec<agdb::DbId> = version_by_id.values().copied().collect();
    let version_number_by_node: HashMap<agdb::DbId, String> =
        read_rows_sync::<VersionRow>(&guard, &version_nodes)?
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row.version_number)))
            .collect();

    let mut author_by_article: HashMap<agdb::DbId, agdb::DbId> = HashMap::new();
    let mut user_nodes: Vec<agdb::DbId> = Vec::new();
    for article_node in &article_nodes {
        let edges = guard.exec(
            QueryBuilder::search()
                .to(*article_node)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_USER_AUTHOR_ARTICLE)
                .query(),
        )?;
        if let Some(edge) = edges.elements.first() {
            author_by_article.insert(*article_node, edge.from);
            user_nodes.push(edge.from);
        }
    }
    let author_name_by_node: HashMap<agdb::DbId, String> =
        read_rows_sync::<UserRow>(&guard, &user_nodes)?
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row.name)))
            .collect();

    for comment in comments.iter_mut() {
        let article_node = article_by_id.get(comment.article_id.as_str());
        comment.article_title = article_node
            .and_then(|node| title_by_node.get(node))
            .cloned()
            .unwrap_or_default();
        comment.article_author_name = article_node
            .and_then(|node| author_by_article.get(node))
            .and_then(|user_node| author_name_by_node.get(user_node))
            .cloned()
            .unwrap_or_default();
        comment.version_number = version_by_id
            .get(comment.version_id.as_str())
            .and_then(|node| version_number_by_node.get(node))
            .cloned()
            .unwrap_or_default();
    }
    Ok(())
}

async fn article_ids_of_user(db: &DbHandle, user_id: &str) -> Result<Vec<String>, DbError> {
    let guard = db.read().await;
    let Some(user) = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)? else {
        return Ok(Vec::new());
    };
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    let edges = guard.exec(
        QueryBuilder::search()
            .from(user)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_AUTHOR_ARTICLE)
            .query(),
    )?;
    for edge in &edges.elements {
        if let Some(row) = read_rows_sync::<IdRow>(&guard, &[edge.to])?
            .into_iter()
            .next()
            && seen.insert(row.id.clone())
        {
            ids.push(row.id);
        }
    }

    let comment_edges = guard.exec(
        QueryBuilder::search()
            .from(user)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_AUTHOR_COMMENT)
            .query(),
    )?;
    for edge in &comment_edges.elements {
        if let Some(article_id) = article_id_of_comment(&guard, edge.to)?
            && seen.insert(article_id.clone())
        {
            ids.push(article_id);
        }
    }
    Ok(ids)
}

fn article_id_of_comment(
    guard: &agdb::DbAny,
    comment: agdb::DbId,
) -> Result<Option<String>, DbError> {
    let version_edges = guard.exec(
        QueryBuilder::search()
            .from(comment)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_COMMENT_ATTACH_VERSION)
            .query(),
    )?;
    let Some(version_edge) = version_edges.elements.first() else {
        return Ok(None);
    };
    let article_edges = guard.exec(
        QueryBuilder::search()
            .to(version_edge.to)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_HOLD_VERSION)
            .query(),
    )?;
    let Some(article_edge) = article_edges.elements.first() else {
        return Ok(None);
    };
    Ok(read_rows_sync::<IdRow>(guard, &[article_edge.from])?
        .into_iter()
        .next()
        .map(|row| row.id))
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
