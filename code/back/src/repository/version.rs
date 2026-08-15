use agdb::{DbError, DbErrorType, QueryBuilder};
use semver::Version;

use crate::repository::graph::{
    DbHandle, find_by_index_in_txn, find_by_index_sync, insert_edge, read_node_in_txn,
    read_rows_sync, resolve_node_id_in_txn, resolve_node_id_sync,
};
use crate::repository::schema::{
    ArticleRow, EDGE_ARTICLE_TO_VERSION, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_VERSION, IdRow,
    KEY_CONTENT_HASH, KEY_LATEST_VERSION_ID, KEY_TYPE, KEY_VERSION_NOTE, VersionRow, alias_of,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionDraft {
    pub version_id: String,
    pub version_number: String,
    pub content_hash: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionEntry {
    pub version_number: String,
    pub content_hash: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionListItem {
    pub id: String,
    pub version_number: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentHashOwner {
    pub version_id: String,
    pub article_title: String,
}

#[derive(Debug)]
pub enum CreateVersionError {
    ArticleMissing,
    NotGreater,
    InvalidNumber,
    ContentHashTaken,
    Db(DbError),
}

impl From<DbError> for CreateVersionError {
    fn from(error: DbError) -> Self {
        Self::Db(error)
    }
}

impl std::fmt::Display for CreateVersionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArticleMissing => formatter.write_str("article not found"),
            Self::NotGreater => formatter.write_str("version not strictly greater"),
            Self::InvalidNumber => formatter.write_str("invalid version number"),
            Self::ContentHashTaken => formatter.write_str("identical content already exists"),
            Self::Db(error) => write!(formatter, "database query failed: {error}"),
        }
    }
}

impl std::error::Error for CreateVersionError {}

pub async fn create_version(
    db: &DbHandle,
    article_id: &str,
    draft: &VersionDraft,
) -> Result<(), CreateVersionError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let Some(article) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_ARTICLE, article_id)?
        else {
            return Err(CreateVersionError::ArticleMissing);
        };
        if !find_by_index_in_txn(transaction, KEY_CONTENT_HASH, &draft.content_hash)?.is_empty() {
            return Err(CreateVersionError::ContentHashTaken);
        }
        let edges = transaction.exec(
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
        let mut max_existing: Option<Version> = None;
        for edge in &edges.elements {
            let Some(stored) =
                read_node_in_txn::<VersionRow>(transaction, edge.to)?.map(|row| row.version_number)
            else {
                continue;
            };
            let parsed = Version::parse(&stored).map_err(|_| CreateVersionError::InvalidNumber)?;
            if max_existing.as_ref().is_none_or(|max| parsed > *max) {
                max_existing = Some(parsed);
            }
        }
        let new_version =
            Version::parse(&draft.version_number).map_err(|_| CreateVersionError::InvalidNumber)?;
        if let Some(max) = max_existing
            && new_version <= max
        {
            return Err(CreateVersionError::NotGreater);
        }

        transaction.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([alias_of(ENTITY_TYPE_VERSION, &draft.version_id)])
                .values(VersionRow {
                    db_id: None,
                    entity_type: ENTITY_TYPE_VERSION.to_string(),
                    id: draft.version_id.clone(),
                    version_number: draft.version_number.clone(),
                    content_hash: draft.content_hash.clone(),
                    note: draft.note.clone(),
                })
                .query(),
        )?;
        insert_edge(
            transaction,
            EDGE_ARTICLE_TO_VERSION,
            article.into(),
            alias_of(ENTITY_TYPE_VERSION, &draft.version_id).into(),
        )?;
        transaction.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .ids([article])
                .values([[(KEY_LATEST_VERSION_ID, draft.version_id.as_str()).into()]])
                .query(),
        )?;
        Ok(())
    })
}

pub async fn read_version(
    db: &DbHandle,
    version_id: &str,
) -> Result<Option<VersionEntry>, DbError> {
    let guard = db.read().await;
    let Some(id) = resolve_node_id_sync(&guard, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(None);
    };
    let row = read_rows_sync::<VersionRow>(&guard, &[id])?
        .into_iter()
        .next()
        .ok_or_else(|| DbError::query(DbErrorType::NotFound, "version row missing"))?;
    Ok(Some(VersionEntry {
        version_number: row.version_number,
        content_hash: row.content_hash,
        note: row.note,
    }))
}

pub async fn update_version(db: &DbHandle, version_id: &str, note: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let Some(id) = resolve_node_id_sync(&guard, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(());
    };
    guard.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .ids([id])
            .values([[(KEY_VERSION_NOTE, note).into()]])
            .query(),
    )?;
    Ok(())
}

pub async fn versions_of(
    db: &DbHandle,
    article_id: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<VersionListItem>, u64), DbError> {
    let guard = db.read().await;
    let Some(article) = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok((Vec::new(), 0));
    };
    let edges = guard.exec(
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
    let version_ids: Vec<agdb::DbId> = edges.elements.iter().map(|edge| edge.to).collect();
    let total = version_ids.len() as u64;
    let id_rows = read_rows_sync::<IdRow>(&guard, &version_ids)?;
    let version_rows = read_rows_sync::<VersionRow>(&guard, &version_ids)?;
    let mut list: Vec<VersionListItem> = id_rows
        .into_iter()
        .zip(version_rows)
        .map(|(id_row, version_row)| VersionListItem {
            id: id_row.id,
            version_number: version_row.version_number,
        })
        .collect();
    list.sort_by(|left, right| right.id.cmp(&left.id));
    let page = list
        .into_iter()
        .skip(usize::try_from(offset).unwrap_or(usize::MAX))
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .collect();
    Ok((page, total))
}

pub async fn content_hash_owner(
    db: &DbHandle,
    content_hash: &str,
) -> Result<Option<ContentHashOwner>, DbError> {
    let guard = db.read().await;
    let ids = find_by_index_sync(&guard, KEY_CONTENT_HASH, content_hash)?;
    let Some(version_id) = ids.first() else {
        return Ok(None);
    };
    let version_business_id = read_rows_sync::<IdRow>(&guard, &[*version_id])?
        .first()
        .map(|row| row.id.clone())
        .unwrap_or_default();
    let edges = guard.exec(
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
    let article_title = match edges.elements.first() {
        Some(edge) => read_rows_sync::<ArticleRow>(&guard, &[edge.from])?
            .first()
            .map(|row| row.title.clone())
            .unwrap_or_default(),
        None => String::new(),
    };
    Ok(Some(ContentHashOwner {
        version_id: version_business_id,
        article_title,
    }))
}

pub async fn parent_article_of(db: &DbHandle, version_id: &str) -> Result<Option<String>, DbError> {
    let guard = db.read().await;
    let Some(version) = resolve_node_id_sync(&guard, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(None);
    };
    let edges = guard.exec(
        QueryBuilder::search()
            .to(version)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_TO_VERSION)
            .query(),
    )?;
    Ok(edges.elements.first().and_then(|edge| {
        read_rows_sync::<IdRow>(&guard, &[edge.from])
            .ok()
            .and_then(|rows| rows.into_iter().next())
            .map(|row| row.id)
    }))
}
