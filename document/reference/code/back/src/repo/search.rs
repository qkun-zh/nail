
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
use serde_json::Value;

use crate::repo::db::DbHandle;
use crate::repo::types::{
    ArticleRow, CommentRow, EDGE_ARTICLE_TO_TAG, EDGE_ARTICLE_TO_VERSION, EDGE_COMMENT_TO_VERSION,
    EDGE_USER_TO_ARTICLE, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, IdRow,
    KEY_ID, KEY_TYPE, TagRow, UserRow, VersionRow, alias_of,
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

pub async fn open_or_create_index(path: &str) -> anyhow::Result<SearchIndexHandle> {
    let dir = std::path::Path::new(path);
    if dir.exists() {
        return open_index(dir)
            .await
            .map_err(|e| anyhow::anyhow!("open search index {path}: {e}"));
    }
    std::fs::create_dir_all(dir)?;
    let schema = vec![
        SchemaField::new(
            FIELD_ID.to_string(),
            true,
            false,
            false,
            FieldType::String16,
            true,
            false,
            1.0,
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
            FIELD_AUTHOR.to_string(),
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
            FIELD_TAG.to_string(),
            true,
            true,
            false,
            FieldType::Json,
            false,
            false,
            1.0,
            false,
            false,
        ),
        SchemaField::new(
            FIELD_COMMENT.to_string(),
            true,
            true,
            false,
            FieldType::Json,
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
    ];
    let meta = IndexMetaObject {
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
    };
    create_index(dir, meta, &schema, &Vec::new(), 11, true, None)
        .await
        .map_err(|e| anyhow::anyhow!("create search index {path}: {e}"))
}

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

pub async fn search_articles(
    index: &SearchIndexHandle,
    db: &DbHandle,
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
            q.clone(),
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
    let mut docs = Vec::with_capacity(result.results.len());
    let query_terms = result.query_terms.clone();
    if !result.results.is_empty() {
        let highlights = Some(highlighter(index, highlight_fields, query_terms).await);
        for hit in &result.results {
            let doc = index
                .read()
                .await
                .get_document(hit.doc_id, false, &highlights, &HashSet::new(), &Vec::new())
                .await
                .map_err(|e| anyhow::anyhow!("fetch search doc failed: {e}"))?;
            let id = doc
                .get(FIELD_ID)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let title = doc
                .get(FIELD_TITLE)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let author = doc
                .get(FIELD_AUTHOR)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let ts_secs = doc.get(FIELD_TS).and_then(|v| v.as_i64()).unwrap_or(0);
            let mut hits = Vec::new();
            for field in &query.fields {
                let snippet = doc
                    .get(&format!("{field}_highlight"))
                    .and_then(|v| v.as_str())
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

    let total = if enable_empty_query {
        count_articles_with_time_window(db, query.from, query.to).await?
    } else {
        result.result_count_total as u64
    };

    Ok(SearchOutcome { docs, total })
}

async fn count_articles_with_time_window(
    db: &DbHandle,
    from: Option<u64>,
    to: Option<u64>,
) -> Result<u64, DbError> {
    let db = db.read().await;
    let all = db.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_ARTICLE)
            .query(),
    )?;
    if from.is_none() && to.is_none() {
        return Ok(all.elements.len() as u64);
    }
    let mut count = 0u64;
    for el in &all.elements {
        let Some(latest_version_id) = crate::repo::db::read_node_sync::<ArticleRow>(&db, el.id)?
            .and_then(|r| r.latest_version_id)
        else {
            continue;
        };
        let Some(ms) = common::time::uuidv7_timestamp_ms(&latest_version_id) else {
            continue;
        };
        let from_ms = from.unwrap_or(0).saturating_mul(1000);
        let to_ms = to
            .unwrap_or(u64::MAX)
            .saturating_mul(1000)
            .saturating_add(999);
        if ms >= from_ms && ms <= to_ms {
            count += 1;
        }
    }
    Ok(count)
}

pub async fn count_articles(db: &DbHandle) -> Result<u64, DbError> {
    count_articles_with_time_window(db, None, None).await
}

pub async fn list_articles_page(
    db: &DbHandle,
    limit: u64,
    offset: u64,
) -> Result<Vec<Value>, DbError> {
    let db = db.read().await;
    let mut rows = db.exec(
        QueryBuilder::search()
            .elements()
            .order_by([agdb::DbKeyOrder::Desc(agdb::DbValue::String(
                KEY_ID.to_string(),
            ))])
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_ARTICLE)
            .query(),
    )?;
    rows.elements = rows
        .elements
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();
    let mut out = Vec::with_capacity(rows.elements.len());
    for el in &rows.elements {
        let Some(row) = crate::repo::db::read_node_sync::<ArticleRow>(&db, el.id)? else {
            continue;
        };
        let id = crate::repo::db::read_node_sync::<IdRow>(&db, el.id)?
            .map(|r| r.id)
            .unwrap_or_default();
        out.push(serde_json::json!({ "id": id, "title": row.title, "summary": row.summary }));
    }
    Ok(out)
}


pub async fn sync_article(
    index: &SearchIndexHandle,
    db: &DbHandle,
    article_id: &str,
) -> anyhow::Result<()> {
    let Some(document) = build_document(db, article_id).await? else {
        return Ok(());
    };
    let existing = find_doc_id(index, article_id).await?;
    match existing {
        Some(doc_id) => index.update_document((doc_id, document)).await,
        None => index.index_document(document, FileType::None).await,
    }
    index.commit().await;
    Ok(())
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
        let ids: Vec<u64> = all.results.iter().map(|r| r.doc_id as u64).collect();
        if !ids.is_empty() {
            index.delete_documents(ids).await;
        }
    }

    let db_read = db.read().await;
    let all = db_read.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_ARTICLE)
            .query(),
    )?;
    drop(db_read);
    let mut documents = Vec::with_capacity(all.elements.len());
    let mut count = 0u64;
    for el in &all.elements {
        let db_read = db.read().await;
        let Some(article_id) =
            crate::repo::db::read_node_sync::<IdRow>(&db_read, el.id)?.map(|r| r.id)
        else {
            continue;
        };
        drop(db_read);
        let Some(document) = build_document(db, &article_id).await? else {
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
    Ok(result.results.first().map(|r| r.doc_id as u64))
}

async fn build_document(db: &DbHandle, article_id: &str) -> anyhow::Result<Option<Document>> {
    let db = db.read().await;
    let Some(article) =
        crate::repo::db::resolve_node_id_sync(&db, ENTITY_TYPE_ARTICLE, article_id)?
    else {
        return Ok(None);
    };
    let article_row = crate::repo::db::read_node_sync::<ArticleRow>(&db, article)?;
    let title = article_row
        .as_ref()
        .map(|r| r.title.clone())
        .unwrap_or_default();
    let summary = article_row
        .as_ref()
        .map(|r| r.summary.clone())
        .unwrap_or_default();
    let latest_version_id = article_row
        .as_ref()
        .and_then(|r| r.latest_version_id.clone())
        .unwrap_or_default();
    let author = {
        let edges = db.exec(
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
            Some(edge) => crate::repo::db::read_node_sync::<UserRow>(&db, edge.from)?
                .map(|r| r.name)
                .unwrap_or_default(),
            None => String::new(),
        }
    };
    let note = match latest_version_id.as_str() {
        "" => String::new(),
        latest => match crate::repo::db::resolve_node_id_sync(&db, ENTITY_TYPE_VERSION, latest)? {
            Some(version) => crate::repo::db::read_node_sync::<VersionRow>(&db, version)?
                .map(|r| r.note)
                .unwrap_or_default(),
            None => String::new(),
        },
    };
    let ts = common::time::uuidv7_timestamp_ms(&latest_version_id)
        .map(|ms| (ms / 1000) as i64)
        .unwrap_or(0);
    let tag_edges = db.exec(
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
        if let Some(name) =
            crate::repo::db::read_node_sync::<TagRow>(&db, edge.to)?.map(|r| r.tag_name)
        {
            tags.push(name);
        }
    }
    let version_edges = db.exec(
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
        let comment_edges = db.exec(
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
                crate::repo::db::read_node_sync::<CommentRow>(&db, comment_edge.from)?
                    .map(|r| r.content)
            {
                comments.push(content);
            }
        }
    }
    let document = serde_json::json!({
        FIELD_ID: article_id,
        FIELD_TITLE: title,
        FIELD_SUMMARY: summary,
        FIELD_AUTHOR: author,
        FIELD_NOTE: note,
        FIELD_TAG: tags,
        FIELD_COMMENT: comments,
        FIELD_TS: ts,
    });
    Ok(Some(
        document
            .as_object()
            .expect("document json is an object")
            .clone()
            .into_iter()
            .collect(),
    ))
}

pub async fn article_ids_of_user(db: &DbHandle, user_id: &str) -> Result<Vec<String>, DbError> {
    let db = db.read().await;
    let edges = db.exec(
        QueryBuilder::search()
            .from(alias_of(ENTITY_TYPE_USER, user_id))
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_TO_ARTICLE)
            .query(),
    )?;
    edges
        .elements
        .iter()
        .map(|el| crate::repo::db::read_node_sync::<IdRow>(&db, el.to).map(|r| r.map(|row| row.id)))
        .collect::<Result<Vec<_>, _>>()
        .map(|ids| ids.into_iter().flatten().collect())
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
