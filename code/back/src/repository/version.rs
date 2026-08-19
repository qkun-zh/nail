use agdb::{DbError, DbErrorType, QueryBuilder};
use semver::Version;

use crate::repository::graph::{
    DbHandle, find_by_index, highest_version_number, incoming_edges, insert_edge, outgoing_edges,
    read_node, read_rows, resolve_node_id,
};
use crate::repository::schema::{
    ArticleRow, EDGE_ARTICLE_HOLD_VERSION, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_VERSION, IdRow,
    KEY_CONTENT_HASH, KEY_LATEST_VERSION_ID, KEY_SOFT_DELETED, KEY_TYPE, KEY_VERSION_NOTE,
    VersionRow, alias_of,
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
        let Some(article) = resolve_node_id(transaction, ENTITY_TYPE_ARTICLE, article_id)? else {
            return Err(CreateVersionError::ArticleMissing);
        };
        if crate::repository::delete::has_soft_deleted_flag(transaction, article)? {
            return Err(CreateVersionError::ArticleMissing);
        }
        if !find_by_index(transaction, KEY_CONTENT_HASH, &draft.content_hash)?.is_empty() {
            return Err(CreateVersionError::ContentHashTaken);
        }
        let edges = outgoing_edges(transaction, article, EDGE_ARTICLE_HOLD_VERSION)?;
        let mut stored_rows = Vec::new();
        for edge in &edges {
            if let Some(row) = read_node::<VersionRow>(transaction, edge.to)? {
                Version::parse(&row.version_number)
                    .map_err(|_| CreateVersionError::InvalidNumber)?;
                stored_rows.push(row);
            }
        }
        let new_version =
            Version::parse(&draft.version_number).map_err(|_| CreateVersionError::InvalidNumber)?;
        if let Some(max) = highest_version_number(stored_rows)
            && new_version
                <= Version::parse(&max.version_number)
                    .map_err(|_| CreateVersionError::InvalidNumber)?
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
            EDGE_ARTICLE_HOLD_VERSION,
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
    let Some(id) = resolve_node_id(&guard, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(None);
    };
    let row = read_rows::<VersionRow>(&guard, &[id])?
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
    let Some(id) = resolve_node_id(&guard, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(());
    };
    if crate::repository::delete::has_soft_deleted_flag(&guard, id)? {
        return Ok(());
    }
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
) -> Result<(Vec<VersionListItem>, bool), DbError> {
    let guard = db.read().await;
    let Some(article) = resolve_node_id(&guard, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok((Vec::new(), false));
    };
    if crate::repository::delete::has_soft_deleted_flag(&guard, article)? {
        return Ok((Vec::new(), false));
    }
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    let nodes = guard.exec(
        QueryBuilder::search()
            .from(article)
            .offset(offset)
            .limit(limit.saturating_add(1) as u64)
            .where_()
            .distance(agdb::CountComparison::Equal(2))
            .and()
            .node()
            .and()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_VERSION)
            .and()
            .not()
            .keys(KEY_SOFT_DELETED)
            .query(),
    )?;
    let has_next = nodes.elements.len() > limit;
    let version_nodes: Vec<agdb::DbId> = nodes
        .elements
        .iter()
        .take(limit)
        .map(|element| element.id)
        .collect();
    let id_rows = read_rows::<IdRow>(&guard, &version_nodes)?;
    let version_rows = read_rows::<VersionRow>(&guard, &version_nodes)?;
    let list: Vec<VersionListItem> = id_rows
        .into_iter()
        .zip(version_rows)
        .map(|(id_row, version_row)| VersionListItem {
            id: id_row.id,
            version_number: version_row.version_number,
        })
        .collect();
    Ok((list, has_next))
}

pub async fn content_hash_owner(
    db: &DbHandle,
    content_hash: &str,
) -> Result<Option<ContentHashOwner>, DbError> {
    let guard = db.read().await;
    let ids = find_by_index(&guard, KEY_CONTENT_HASH, content_hash)?;
    let Some(version_id) = ids.first() else {
        return Ok(None);
    };
    let version_business_id = read_rows::<IdRow>(&guard, &[*version_id])?
        .first()
        .map(|row| row.id.clone())
        .unwrap_or_default();
    let edges = incoming_edges(&guard, *version_id, EDGE_ARTICLE_HOLD_VERSION)?;
    let article_title = match edges.first() {
        Some(edge) => read_rows::<ArticleRow>(&guard, &[edge.from])?
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
    let Some(version) = resolve_node_id(&guard, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(None);
    };
    let edges = incoming_edges(&guard, version, EDGE_ARTICLE_HOLD_VERSION)?;
    Ok(edges.first().and_then(|edge| {
        read_rows::<IdRow>(&guard, &[edge.from])
            .ok()
            .and_then(|rows| rows.into_iter().next())
            .map(|row| row.id)
    }))
}
