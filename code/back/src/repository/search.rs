use std::collections::HashSet;

use agdb::{DbError, QueryBuilder};
use seekstorm::commit::Commit;
use seekstorm::highlighter::{Highlight, highlighter};
use seekstorm::index::{
    AccessType, Clustering, DeleteDocuments, Document, DocumentCompression, FieldType, FileType,
    FrequentwordType, IndexDocument, IndexDocuments, IndexMetaObject, LexicalSimilarity, NgramSet,
    SchemaField, StemmerType, StopwordType, TokenizerType, UpdateDocument, create_index,
    open_index,
};
use seekstorm::search::{
    FacetFilter, FacetValue, QueryRewriting, QueryType, ResultSort, ResultType, Search, SearchMode,
    SortOrder,
};
use seekstorm::vector::Inference;

use crate::repository::graph::DbHandle;
use crate::repository::schema::{
    ArticleRow, CommentRow, EDGE_ARTICLE_TO_TAG, EDGE_ARTICLE_TO_VERSION, EDGE_COMMENT_TO_VERSION,
    EDGE_USER_TO_ARTICLE, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, IdRow,
    KEY_TYPE, TagRow, UserRow, VersionRow,
};

pub type SearchIndexHandle = seekstorm::index::IndexArc;

pub const FIELD_ID: &str = "id";
pub const FIELD_TITLE: &str = "title";
pub const FIELD_SUMMARY: &str = "summary";
pub const FIELD_AUTHOR: &str = "author";
pub const FIELD_NOTE: &str = "note";
pub const FIELD_TAG: &str = "tag";
pub const FIELD_COMMENT: &str = "comment";
pub const FIELD_TS: &str = "ts";

pub struct SearchQuery {
    pub q: Option<String>,
    pub fields: Vec<String>,
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub sort: Vec<(String, bool)>,
    pub offset: u64,
    pub limit: u64,
}

pub struct SearchHitDoc {
    pub id: String,
    pub title: String,
    pub author: String,
    pub ts_secs: i64,
    pub hits: Vec<(String, String)>,
}

pub struct SearchOutcome {
    pub docs: Vec<SearchHitDoc>,
    pub total: u64,
}

pub async fn open_or_create_index(path: &str) -> anyhow::Result<SearchIndexHandle> {
    let directory = std::path::Path::new(path);
    if directory.exists() {
        return open_index(directory)
            .await
            .map_err(|error| anyhow::anyhow!("open search index {path}: {error}"));
    }
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
    .map_err(|error| anyhow::anyhow!("create search index {path}: {error}"))
}

pub async fn search_articles(
    index: &SearchIndexHandle,
    query: SearchQuery,
) -> anyhow::Result<SearchOutcome> {
    let enable_empty_query = query.q.is_none();
    let q = query.q.unwrap_or_default();
    let mut facet_filter: Vec<FacetFilter> = Vec::new();
    if query.from.is_some() || query.to.is_some() {
        let from = query.from.unwrap_or(0);
        let to = query.to.unwrap_or(u64::MAX);
        let low = from.min(to) as i64;
        let high = to.saturating_add(1).min(i64::MAX as u64) as i64;
        facet_filter.push(FacetFilter::Timestamp {
            field: FIELD_TS.to_string(),
            filter: low..high,
        });
    }
    let result_sort: Vec<ResultSort> = query
        .sort
        .iter()
        .map(|(field, descending)| ResultSort {
            field: field.clone(),
            order: if *descending {
                SortOrder::Descending
            } else {
                SortOrder::Ascending
            },
            base: FacetValue::None,
        })
        .collect();

    let result = index
        .search(
            q,
            None,
            QueryType::Intersection,
            SearchMode::Lexical,
            enable_empty_query,
            query.offset as usize,
            query.limit as usize,
            ResultType::TopkCount,
            false,
            query.fields.clone(),
            Vec::new(),
            facet_filter,
            result_sort,
            QueryRewriting::SearchOnly,
        )
        .await;

    let highlight_fields: Vec<Highlight> = query
        .fields
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
    let mut docs = Vec::with_capacity(result.results.len());
    if !result.results.is_empty() {
        let highlights = Some(highlighter(index, highlight_fields, query_terms).await);
        for hit in &result.results {
            let document = index
                .read()
                .await
                .get_document(hit.doc_id, false, &highlights, &HashSet::new(), &Vec::new())
                .await
                .map_err(|error| anyhow::anyhow!("fetch search doc failed: {error}"))?;
            let id = document
                .get(FIELD_ID)
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            let title = document
                .get(FIELD_TITLE)
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            let author = document
                .get(FIELD_AUTHOR)
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            let ts_secs = document.get(FIELD_TS).and_then(|value| value.as_i64()).unwrap_or(0);
            let mut hits = Vec::new();
            for field in &query.fields {
                let snippet = document
                    .get(&format!("{field}_highlight"))
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if snippet.contains("<mark>") {
                    hits.push((field.clone(), snippet.to_string()));
                }
            }
            docs.push(SearchHitDoc {
                id,
                title,
                author,
                ts_secs,
                hits,
            });
        }
    }

    Ok(SearchOutcome {
        docs,
        total: result.result_count_total as u64,
    })
}

pub async fn sync_article(
    index: &SearchIndexHandle,
    db: &DbHandle,
    article_id: &str,
) -> anyhow::Result<()> {
    let Some(document) = build_document(db, article_id).await? else {
        return Ok(());
    };
    match find_doc_id(index, article_id).await? {
        Some(doc_id) => index.update_document((doc_id, document)).await,
        None => index.index_document(document, FileType::None).await,
    }
    index.commit().await;
    Ok(())
}

pub async fn sync_articles_of_user(
    index: &SearchIndexHandle,
    db: &DbHandle,
    user_id: &str,
) -> anyhow::Result<u64> {
    let article_ids = article_ids_of_user(db, user_id).await?;
    let mut synced = 0u64;
    for article_id in &article_ids {
        if sync_article(index, db, article_id).await.is_ok() {
            synced += 1;
        }
    }
    Ok(synced)
}

pub async fn rebuild_index(index: &SearchIndexHandle, db: &DbHandle) -> anyhow::Result<u64> {
    let live = index.read().await.current_doc_count().await;
    if live > 0 {
        let all = index
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
            index.delete_documents(ids).await;
        }
    }

    let guard = db.read().await;
    let all = guard.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_ARTICLE)
            .query(),
    )?;
    let article_ids: Vec<String> = all
        .elements
        .iter()
        .filter_map(|element| {
            crate::repository::graph::read_rows_sync::<IdRow>(&guard, &[element.id])
                .ok()
                .and_then(|rows| rows.into_iter().next())
                .map(|row| row.id)
        })
        .collect();
    drop(guard);

    let mut documents = Vec::with_capacity(article_ids.len());
    let mut count = 0u64;
    for article_id in &article_ids {
        let Some(document) = build_document(db, article_id).await? else {
            continue;
        };
        documents.push(document);
        count += 1;
    }
    if !documents.is_empty() {
        index.index_documents(documents).await;
    }
    index.commit().await;
    Ok(count)
}

async fn find_doc_id(index: &SearchIndexHandle, article_id: &str) -> anyhow::Result<Option<u64>> {
    let result = index
        .search(
            String::new(),
            None,
            QueryType::Intersection,
            SearchMode::Lexical,
            true,
            0,
            1,
            ResultType::TopkCount,
            false,
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

async fn build_document(db: &DbHandle, article_id: &str) -> anyhow::Result<Option<Document>> {
    let guard = db.read().await;
    let Some(article) =
        crate::repository::graph::resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, article_id)?
    else {
        return Ok(None);
    };
    let article_row = crate::repository::graph::read_rows_sync::<ArticleRow>(&guard, &[article])?
        .into_iter()
        .next();
    let title = article_row.as_ref().map(|row| row.title.clone()).unwrap_or_default();
    let summary = article_row
        .as_ref()
        .map(|row| row.summary.clone())
        .unwrap_or_default();
    let latest_version_id = article_row
        .as_ref()
        .and_then(|row| row.latest_version_id.clone())
        .unwrap_or_default();

    let author = {
        let edges = guard.exec(
            QueryBuilder::search()
                .to(article)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_USER_TO_ARTICLE)
                .query(),
        )?;
        match edges.elements.first() {
            Some(edge) => crate::repository::graph::read_rows_sync::<UserRow>(&guard, &[edge.from])?
                .into_iter()
                .next()
                .map(|row| row.name)
                .unwrap_or_default(),
            None => String::new(),
        }
    };

    let note = if latest_version_id.is_empty() {
        String::new()
    } else {
        match crate::repository::graph::resolve_node_id_sync(
            &guard,
            ENTITY_TYPE_VERSION,
            &latest_version_id,
        )? {
            Some(version) => {
                crate::repository::graph::read_rows_sync::<VersionRow>(&guard, &[version])?
                    .into_iter()
                    .next()
                    .map(|row| row.note)
                    .unwrap_or_default()
            }
            None => String::new(),
        }
    };

    let ts = nail_common::time::uuidv7_timestamp_ms(&latest_version_id)
        .map(|millis| (millis / 1000) as i64)
        .unwrap_or(0);

    let tag_edges = guard.exec(
        QueryBuilder::search()
            .from(article)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_TO_TAG)
            .query(),
    )?;
    let mut tags: Vec<String> = Vec::with_capacity(tag_edges.elements.len());
    for edge in &tag_edges.elements {
        if let Some(name) = crate::repository::graph::read_rows_sync::<TagRow>(&guard, &[edge.to])?
            .into_iter()
            .next()
            .map(|row| row.tag_name)
        {
            tags.push(name);
        }
    }

    let version_edges = guard.exec(
        QueryBuilder::search()
            .from(article)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_TO_VERSION)
            .query(),
    )?;
    let mut comments: Vec<String> = Vec::new();
    for version_edge in &version_edges.elements {
        let comment_edges = guard.exec(
            QueryBuilder::search()
                .to(version_edge.to)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_COMMENT_TO_VERSION)
                .query(),
        )?;
        for comment_edge in &comment_edges.elements {
            if let Some(content) =
                crate::repository::graph::read_rows_sync::<CommentRow>(&guard, &[comment_edge.from])?
                    .into_iter()
                    .next()
                    .map(|row| row.content)
            {
                comments.push(content);
            }
        }
    }

    let mut document = Document::new();
    document.insert(FIELD_ID.to_string(), serde_json::json!(article_id));
    document.insert(FIELD_TITLE.to_string(), serde_json::json!(title));
    document.insert(FIELD_SUMMARY.to_string(), serde_json::json!(summary));
    document.insert(FIELD_AUTHOR.to_string(), serde_json::json!(author));
    document.insert(FIELD_NOTE.to_string(), serde_json::json!(note));
    document.insert(FIELD_TAG.to_string(), serde_json::json!(tags));
    document.insert(FIELD_COMMENT.to_string(), serde_json::json!(comments));
    document.insert(FIELD_TS.to_string(), serde_json::json!(ts));
    Ok(Some(document))
}

pub async fn article_ids_of_user(db: &DbHandle, user_id: &str) -> Result<Vec<String>, DbError> {
    let guard = db.read().await;
    let Some(user) =
        crate::repository::graph::resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)?
    else {
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
        if let Some(row) = crate::repository::graph::read_rows_sync::<IdRow>(&guard, &[edge.to])?
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
