
use agdb::{DbError, QueryBuilder};
use semver::Version;

use crate::repo::db::DbHandle;
use crate::repo::types::{
    ArticleRow, EDGE_ARTICLE_TO_VERSION, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_VERSION,
    KEY_CONTENT_HASH, KEY_LATEST_VERSION_ID, KEY_TYPE, VersionRow, alias_of,
};

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
        CreateVersionError::Db(error)
    }
}

impl std::fmt::Display for CreateVersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateVersionError::ArticleNotFound => write!(f, "article not found"),
            CreateVersionError::VersionNotGreater => write!(f, "version not strictly greater"),
            CreateVersionError::InvalidVersion => write!(f, "invalid version number"),
            CreateVersionError::ContentHashExists => write!(f, "identical content already exists"),
            CreateVersionError::Db(e) => write!(f, "database query failed: {e}"),
        }
    }
}
impl std::error::Error for CreateVersionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CreateVersionError::Db(e) => Some(e),
            _ => None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn create_version(
    db: &DbHandle,
    article_id: &str,
    version_id: &str,
    version_number: &str,
    content_hash: &str,
    note: &str,
) -> Result<(), CreateVersionError> {
    if uuid::Uuid::parse_str(version_id)
        .map(|u| u.get_version_num())
        .ok()
        != Some(7)
    {
        return Err(CreateVersionError::Db(DbError::query(
            agdb::DbErrorType::TypeError,
            format!("create_version: version_id is not a uuidv7: {version_id}"),
        )));
    }

    let mut db = db.write().await;
    db.transaction_mut(|txn| -> Result<(), CreateVersionError> {
        let Some(article) =
            crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_ARTICLE, article_id)?
        else {
            return Err(CreateVersionError::ArticleNotFound);
        };
        if !crate::repo::db::find_by_index_in_txn(txn, KEY_CONTENT_HASH, content_hash)?.is_empty() {
            return Err(CreateVersionError::ContentHashExists);
        }
        let edges = txn.exec(
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
            let Some(stored) = crate::repo::db::read_node_in_txn::<VersionRow>(txn, edge.to)?
                .map(|r| r.version_number)
            else {
                continue;
            };
            let parsed = Version::parse(&stored).map_err(|_| CreateVersionError::InvalidVersion)?;
            if max_existing.as_ref().is_none_or(|m| parsed > *m) {
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

        let alias = alias_of(ENTITY_TYPE_VERSION, version_id);
        txn.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([alias])
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
        super::relate(
            txn,
            EDGE_ARTICLE_TO_VERSION,
            article.into(),
            alias_of(ENTITY_TYPE_VERSION, version_id).into(),
        )?;
        let current_latest = crate::repo::db::read_node_in_txn::<ArticleRow>(txn, article)?
            .and_then(|r| r.latest_version_id);
        if current_latest.as_deref().is_none_or(|cur| version_id > cur) {
            txn.exec_mut(
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
