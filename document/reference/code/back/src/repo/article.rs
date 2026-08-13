
use agdb::{DbError, QueryBuilder};
use serde_json::Value;

use crate::repo::db::{DbHandle, read_rows_sync, resolve_node_id_sync};
use crate::repo::types::{ArticleRow, IdRow, VersionRow};
use crate::repo::types::{
    EDGE_ARTICLE_TO_TAG, EDGE_ARTICLE_TO_VERSION, EDGE_USER_TO_ARTICLE, ENTITY_TYPE_ARTICLE,
    ENTITY_TYPE_TAG, ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, KEY_CONTENT_HASH, KEY_SUMMARY,
    KEY_TITLE, KEY_TYPE, VISIBILITY_PUBLIC, alias_of,
};

pub mod edge;
pub mod version;
pub mod view;

pub use crate::repo::transfer::transfer_account_assets;
pub use crate::repo::transfer::transfer_article_ownership;
pub(crate) use crate::repo::types::VersionEntry;
pub(crate) use edge::relate;
pub use edge::{find_article_id_by_version, version_belongs_to_article};
pub use version::{CreateVersionError, create_version};
pub use view::enrich_articles_batch;

#[derive(Debug)]
pub enum CreateArticleError {
    AuthorNotFound,
    TitleAlreadyExists,
    #[allow(dead_code)]
    TagNotFound,
    Db(DbError),
}

impl From<DbError> for CreateArticleError {
    fn from(error: DbError) -> Self {
        CreateArticleError::Db(error)
    }
}

#[derive(Debug)]
pub enum UpdateArticleError {
    NotFound,
    TitleAlreadyExists,
    TagNotFound,
    Db(DbError),
}

impl From<DbError> for UpdateArticleError {
    fn from(error: DbError) -> Self {
        UpdateArticleError::Db(error)
    }
}

pub async fn read_article(db: &DbHandle, article_id: &str) -> Result<Option<Value>, DbError> {
    let db = db.read().await;
    let Some(id) = resolve_node_id_sync(&db, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(None);
    };
    let row = read_rows_sync::<ArticleRow>(&db, &[id])?.into_iter().next();
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(serde_json::json!({
        "id": article_id,
        "title": row.title,
        "summary": row.summary,
    })))
}

pub async fn read_version(
    db: &DbHandle,
    version_id: &str,
) -> Result<Option<VersionEntry>, DbError> {
    let db = db.read().await;
    let Some(id) = resolve_node_id_sync(&db, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(None);
    };
    let row = read_rows_sync::<VersionRow>(&db, &[id])?
        .into_iter()
        .next()
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, "version row missing"))?;
    Ok(Some(VersionEntry {
        version_number: row.version_number,
        content_hash: row.content_hash,
        note: row.note,
    }))
}

pub async fn find_version_by_hash(
    db: &DbHandle,
    content_hash: &str,
) -> Result<Option<(String, String)>, DbError> {
    let db = db.read().await;
    let ids = crate::repo::db::find_by_index_sync(&db, KEY_CONTENT_HASH, content_hash)?;
    let Some(version_id) = ids.first() else {
        return Ok(None);
    };
    let version_business_id = read_rows_sync::<IdRow>(&db, &[*version_id])?
        .first()
        .map(|r| r.id.clone())
        .unwrap_or_default();
    let edges = db.exec(
        QueryBuilder::search()
            .to(*version_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_TO_VERSION)
            .query(),
    )?;
    let title = match edges.elements.first() {
        Some(edge) => read_rows_sync::<ArticleRow>(&db, &[edge.from])?
            .first()
            .map(|r| r.title.clone())
            .unwrap_or_default(),
        None => String::new(),
    };
    Ok(Some((version_business_id, title)))
}

pub async fn read_article_versions(
    db: &DbHandle,
    article_id: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<Value>, u64), DbError> {
    let db = db.read().await;
    let Some(article) = resolve_node_id_sync(&db, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok((Vec::new(), 0));
    };
    let edges = db.exec(
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
    let version_ids: Vec<agdb::DbId> = edges.elements.iter().map(|el| el.to).collect();
    let total = version_ids.len() as u64;
    let id_rows = read_rows_sync::<IdRow>(&db, &version_ids)?;
    let version_rows = read_rows_sync::<VersionRow>(&db, &version_ids)?;
    let mut versions: Vec<(String, &VersionRow)> = version_ids
        .iter()
        .zip(id_rows.iter())
        .zip(version_rows.iter())
        .filter_map(|((_id, id_row), vrow)| Some((id_row.id.clone(), vrow)))
        .collect();
    versions.sort_by(|a, b| b.0.cmp(&a.0));
    let page: Vec<Value> = versions
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(|(business_id, row)| {
            serde_json::json!({
                "id": business_id,
                "version_number": row.version_number,
                "content_hash": row.content_hash,
                "note": row.note,
            })
        })
        .collect();
    Ok((page, total))
}

#[allow(clippy::too_many_arguments)]
pub async fn create_article(
    db: &DbHandle,
    article_id: &str,
    author_id: &str,
    title: &str,
    summary: &str,
    tag_names: &[String],
    version_id: &str,
    version_number: &str,
    content_hash: &str,
    note: &str,
) -> Result<(), CreateArticleError> {
    if uuid::Uuid::parse_str(version_id)
        .map(|u| u.get_version_num())
        .ok()
        != Some(7)
    {
        return Err(CreateArticleError::Db(DbError::query(
            agdb::DbErrorType::TypeError,
            format!("create_article: version_id is not a uuidv7: {version_id}"),
        )));
    }

    let mut db = db.write().await;
    db.transaction_mut(|txn| -> Result<(), CreateArticleError> {
        let author = crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_USER, author_id)?
            .ok_or(CreateArticleError::AuthorNotFound)?;
        if !crate::repo::db::find_by_index_in_txn(txn, KEY_TITLE, title)?.is_empty() {
            return Err(CreateArticleError::TitleAlreadyExists);
        }
        if !crate::repo::db::find_by_index_in_txn(txn, KEY_CONTENT_HASH, content_hash)?.is_empty() {
            return Err(CreateArticleError::TitleAlreadyExists);
        }
        let mut seen_tags = std::collections::HashSet::new();
        let mut tag_ids = Vec::with_capacity(tag_names.len());
        for name in tag_names {
            if seen_tags.insert(name) {
                let tag_ref = crate::repo::tag::get_or_create_tag_in_txn(txn, name)?;
                tag_ids.push(tag_ref.id);
            }
        }
        let version_alias = alias_of(ENTITY_TYPE_VERSION, version_id);
        txn.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([version_alias.clone()])
                .values(VersionRow {
                    db_id: None,
                    entity_type: ENTITY_TYPE_VERSION.to_string(),
                    id: version_id.to_string(),
                    version_number: version_number.to_string(),
                    content_hash: content_hash.to_string(),
                    note: note.to_string(),
                })
                .query(),
        )?;
        let article_alias = alias_of(ENTITY_TYPE_ARTICLE, article_id);
        txn.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([article_alias.clone()])
                .values(ArticleRow {
                    db_id: None,
                    entity_type: ENTITY_TYPE_ARTICLE.to_string(),
                    id: article_id.to_string(),
                    title: title.to_string(),
                    summary: summary.to_string(),
                    visibility: Some(VISIBILITY_PUBLIC.to_string()),
                    latest_version_id: Some(version_id.to_string()),
                })
                .query(),
        )?;
        relate(
            txn,
            EDGE_USER_TO_ARTICLE,
            author.into(),
            article_alias.clone().into(),
        )?;
        relate(
            txn,
            EDGE_ARTICLE_TO_VERSION,
            article_alias.clone().into(),
            version_alias.into(),
        )?;
        for tag_id in &tag_ids {
            relate(
                txn,
                EDGE_ARTICLE_TO_TAG,
                article_alias.clone().into(),
                alias_of(ENTITY_TYPE_TAG, tag_id).into(),
            )?;
        }
        Ok(())
    })
}

pub async fn update_article(
    db: &DbHandle,
    article_id: &str,
    author_id: &str,
    title: &str,
    summary: &str,
    tag_names: &[String],
) -> Result<(), UpdateArticleError> {
    let mut db = db.write().await;
    db.transaction_mut(|txn| -> Result<(), UpdateArticleError> {
        let author = crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_USER, author_id)?
            .ok_or(UpdateArticleError::NotFound)?;
        let article =
            crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_ARTICLE, article_id)?
                .ok_or(UpdateArticleError::NotFound)?;
        let title_conflict = crate::repo::db::find_by_index_in_txn(txn, KEY_TITLE, title)?
            .into_iter()
            .any(|other| other != article);
        if title_conflict {
            return Err(UpdateArticleError::TitleAlreadyExists);
        }
        let _ = author;
        txn.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .ids([article])
                .values([[(KEY_TITLE, title).into(), (KEY_SUMMARY, summary).into()]])
                .query(),
        )?;
        let old_edges = txn.exec(
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
        let old_ids: std::collections::HashSet<agdb::DbId> =
            old_edges.elements.iter().map(|el| el.to).collect();
        let mut seen_tags = std::collections::HashSet::new();
        let mut new_ids = Vec::with_capacity(tag_names.len());
        for name in tag_names {
            if seen_tags.insert(name) {
                let tag_ref = crate::repo::tag::get_or_create_tag_in_txn(txn, name)?;
                let tag_id =
                    crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_TAG, &tag_ref.id)?
                        .ok_or(UpdateArticleError::TagNotFound)?;
                new_ids.push(tag_id);
            }
        }
        for tag_id in &new_ids {
            if !old_ids.contains(tag_id) {
                relate(txn, EDGE_ARTICLE_TO_TAG, article.into(), (*tag_id).into())?;
            }
        }
        let new_set: std::collections::HashSet<agdb::DbId> = new_ids.iter().copied().collect();
        let stale_edge_ids: Vec<agdb::DbId> = old_edges
            .elements
            .iter()
            .filter(|el| !new_set.contains(&el.to))
            .map(|el| el.id)
            .collect();
        if !stale_edge_ids.is_empty() {
            txn.exec_mut(QueryBuilder::remove().ids(stale_edge_ids).query())?;
        }
        let orphans = txn.exec(
            QueryBuilder::search()
                .elements()
                .where_()
                .key(KEY_TYPE)
                .value(ENTITY_TYPE_TAG)
                .and()
                .edge_count_to(agdb::CountComparison::Equal(0))
                .query(),
        )?;
        if !orphans.elements.is_empty() {
            let ids: Vec<agdb::DbId> = orphans.elements.iter().map(|el| el.id).collect();
            txn.exec_mut(QueryBuilder::remove().ids(ids).query())?;
        }
        Ok(())
    })
}

impl std::fmt::Display for CreateArticleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateArticleError::AuthorNotFound => write!(f, "author not found"),
            CreateArticleError::TitleAlreadyExists => write!(f, "title already exists"),
            CreateArticleError::TagNotFound => write!(f, "tag not found"),
            CreateArticleError::Db(e) => write!(f, "database query failed: {e}"),
        }
    }
}
impl std::error::Error for CreateArticleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CreateArticleError::Db(e) => Some(e),
            _ => None,
        }
    }
}

impl std::fmt::Display for UpdateArticleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateArticleError::NotFound => write!(f, "article or author not found"),
            UpdateArticleError::TitleAlreadyExists => write!(f, "title already exists"),
            UpdateArticleError::TagNotFound => write!(f, "tag not found"),
            UpdateArticleError::Db(e) => write!(f, "database query failed: {e}"),
        }
    }
}
impl std::error::Error for UpdateArticleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UpdateArticleError::Db(e) => Some(e),
            _ => None,
        }
    }
}

pub async fn update_version_note(
    db: &DbHandle,
    version_id: &str,
    note: &str,
) -> Result<Option<String>, DbError> {
    let mut db = db.write().await;
    let Some(id) = resolve_node_id_sync(&db, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(None);
    };
    db.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .ids([id])
            .values([[(crate::repo::types::KEY_VERSION_NOTE, note).into()]])
            .query(),
    )?;
    Ok(Some(note.to_string()))
}
