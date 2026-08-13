use agdb::{DbError, DbErrorType, QueryBuilder};
use semver::Version;

use crate::repository::graph::{
    DbHandle, find_by_index_in_txn, read_node_in_txn, read_rows_sync, resolve_node_id_in_txn,
    resolve_node_id_sync,
};
use crate::repository::schema::{
    ArticleRow, EDGE_ARTICLE_TO_VERSION, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_VERSION,
    KEY_CONTENT_HASH, KEY_LATEST_VERSION_ID, KEY_TYPE, KEY_VERSION_NOTE, VersionRow, alias_of,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionEntry {
    pub version_number: String,
    pub content_hash: String,
    pub note: String,
}

#[derive(Debug)]
pub enum CreateVersionError {
    ArticleNotFound,
    VersionNotGreater,
    InvalidVersion,
    ContentHashExists,
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
            Self::ArticleNotFound => formatter.write_str("article not found"),
            Self::VersionNotGreater => formatter.write_str("version not strictly greater"),
            Self::InvalidVersion => formatter.write_str("invalid version number"),
            Self::ContentHashExists => formatter.write_str("identical content already exists"),
            Self::Db(error) => write!(formatter, "database query failed: {error}"),
        }
    }
}

impl std::error::Error for CreateVersionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Db(error) => Some(error),
            _ => None,
        }
    }
}

pub async fn create_version(
    db: &DbHandle,
    article_id: &str,
    version_id: &str,
    version_number: &str,
    content_hash: &str,
    note: &str,
) -> Result<(), CreateVersionError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let Some(article) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_ARTICLE, article_id)?
        else {
            return Err(CreateVersionError::ArticleNotFound);
        };
        if !find_by_index_in_txn(transaction, KEY_CONTENT_HASH, content_hash)?.is_empty() {
            return Err(CreateVersionError::ContentHashExists);
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
            let Some(stored) = read_node_in_txn::<VersionRow>(transaction, edge.to)?
                .map(|row| row.version_number)
            else {
                continue;
            };
            let parsed =
                Version::parse(&stored).map_err(|_| CreateVersionError::InvalidVersion)?;
            if max_existing.as_ref().is_none_or(|max| parsed > *max) {
                max_existing = Some(parsed);
            }
        }
        let new_version =
            Version::parse(version_number).map_err(|_| CreateVersionError::InvalidVersion)?;
        if let Some(max) = max_existing
            && new_version <= max
        {
            return Err(CreateVersionError::VersionNotGreater);
        }

        transaction.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([alias_of(ENTITY_TYPE_VERSION, version_id)])
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
        transaction.exec_mut(
            QueryBuilder::insert()
                .edges()
                .from(article)
                .to([alias_of(ENTITY_TYPE_VERSION, version_id)])
                .values([[(KEY_TYPE, EDGE_ARTICLE_TO_VERSION).into()]])
                .query(),
        )?;
        let current_latest = read_node_in_txn::<ArticleRow>(transaction, article)?
            .and_then(|row| row.latest_version_id);
        if current_latest.as_deref().is_none_or(|latest| version_id > latest) {
            transaction.exec_mut(
                QueryBuilder::insert()
                    .nodes()
                    .ids([article])
                    .values([[(KEY_LATEST_VERSION_ID, version_id).into()]])
                    .query(),
            )?;
        }
        Ok(())
    })
}

pub async fn read_version(db: &DbHandle, version_id: &str) -> Result<Option<VersionEntry>, DbError> {
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

pub async fn update_version_note(
    db: &DbHandle,
    version_id: &str,
    note: &str,
) -> Result<Option<String>, DbError> {
    let mut guard = db.write().await;
    let Some(id) = resolve_node_id_sync(&guard, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(None);
    };
    guard.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .ids([id])
            .values([[(KEY_VERSION_NOTE, note).into()]])
            .query(),
    )?;
    Ok(Some(note.to_string()))
}
