use database::{Database, EdgeKind, Error, NodeKind, Value};
use semver::Version;

use crate::repository::access::GraphRead;
use crate::repository::delete::{has_soft_deleted_flag, highest_version_number};
use crate::repository::schema::{
    ArticleRow, IdRow, KEY_CONTENT_HASH, KEY_LATEST_VERSION_ID, KEY_VERSION_NOTE, VersionRow,
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
    Db(Error),
}

impl From<Error> for CreateVersionError {
    fn from(error: Error) -> Self {
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

pub fn create_version(
    db: &Database,
    article_id: &str,
    draft: &VersionDraft,
) -> Result<(), CreateVersionError> {
    db.write(|scope| {
        let Some(article) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok(Err(CreateVersionError::ArticleMissing));
        };
        if has_soft_deleted_flag(scope, article)? {
            return Ok(Err(CreateVersionError::ArticleMissing));
        }
        if scope
            .find_by_key(KEY_CONTENT_HASH, &draft.content_hash)?
            .is_some()
        {
            return Ok(Err(CreateVersionError::ContentHashTaken));
        }
        let edges = scope.outgoing(article, EdgeKind::ArticleHoldVersion)?;
        let stored_rows = scope.read_nodes::<VersionRow>(&edges)?;
        for row in &stored_rows {
            if Version::parse(&row.version_number).is_err() {
                return Ok(Err(CreateVersionError::InvalidNumber));
            }
        }
        let Ok(new_version) = Version::parse(&draft.version_number) else {
            return Ok(Err(CreateVersionError::InvalidNumber));
        };
        if let Some(max) = highest_version_number(stored_rows) {
            let Ok(max_number) = Version::parse(&max.version_number) else {
                return Ok(Err(CreateVersionError::InvalidNumber));
            };
            if new_version <= max_number {
                return Ok(Err(CreateVersionError::NotGreater));
            }
        }

        scope.insert_node(&VersionRow {
            id: draft.version_id.clone(),
            version_number: draft.version_number.clone(),
            content_hash: draft.content_hash.clone(),
            note: draft.note.clone(),
        })?;
        let version_node = scope
            .resolve(NodeKind::Version, &draft.version_id)?
            .ok_or_else(|| Error::Invalid("inserted version missing".to_string()))?;
        scope.insert_edge(
            NodeKind::Article,
            article,
            EdgeKind::ArticleHoldVersion,
            NodeKind::Version,
            version_node,
        )?;
        scope.set_key(
            article,
            KEY_LATEST_VERSION_ID,
            Value::Text(draft.version_id.clone()),
        )?;
        Ok(Ok(()))
    })
    .map_err(CreateVersionError::from)
    .and_then(std::convert::identity)
}

pub fn read_version(db: &Database, version_id: &str) -> Result<Option<VersionEntry>, Error> {
    db.read(|scope| {
        let Some(id) = scope.resolve(NodeKind::Version, version_id)? else {
            return Ok(None);
        };
        let row = scope.scope_read_node::<VersionRow>(id)?.ok_or_else(|| {
            Error::Invalid(format!(
                "version {version_id} exists but has no readable row"
            ))
        })?;
        Ok(Some(VersionEntry {
            version_number: row.version_number,
            content_hash: row.content_hash,
            note: row.note,
        }))
    })
}

pub fn update_version(db: &Database, version_id: &str, note: &str) -> Result<(), Error> {
    db.write(|scope| {
        let Some(id) = scope.resolve(NodeKind::Version, version_id)? else {
            return Ok(());
        };
        if has_soft_deleted_flag(scope, id)? {
            return Ok(());
        }
        scope.set_key(id, KEY_VERSION_NOTE, Value::Text(note.to_string()))?;
        Ok(())
    })
}

pub fn count_versions_of(db: &Database, article_id: &str) -> Result<u64, Error> {
    db.read(|scope| {
        let Some(article) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok(0);
        };
        if has_soft_deleted_flag(scope, article)? {
            return Ok(0);
        }
        live_versions_of(scope, article).map(|versions| versions.len() as u64)
    })
}

fn live_versions_of(
    scope: &impl GraphRead,
    article: database::NodeId,
) -> Result<Vec<VersionRow>, Error> {
    let nodes = scope.scope_outgoing(article, EdgeKind::ArticleHoldVersion)?;
    let rows = scope.scope_read_nodes::<VersionRow>(&nodes)?;
    let mut live = Vec::with_capacity(rows.len());
    for (node, row) in nodes.into_iter().zip(rows) {
        if !has_soft_deleted_flag(scope, node)? {
            live.push(row);
        }
    }
    Ok(live)
}

pub fn versions_of(
    db: &Database,
    article_id: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<VersionListItem>, bool), Error> {
    db.read(|scope| {
        let Some(article) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok((Vec::new(), false));
        };
        if has_soft_deleted_flag(scope, article)? {
            return Ok((Vec::new(), false));
        }
        let mut live = live_versions_of(scope, article)?;
        let has_next = (live.len() as u64) > offset + limit;
        let page: Vec<VersionListItem> = live
            .drain(..)
            .skip(usize::try_from(offset).unwrap_or(usize::MAX))
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .map(|row| VersionListItem {
                id: row.id,
                version_number: row.version_number,
            })
            .collect();
        Ok((page, has_next))
    })
}

pub fn content_hash_owner(
    db: &Database,
    content_hash: &str,
) -> Result<Option<ContentHashOwner>, Error> {
    db.read(|scope| {
        let Some(version_node) = scope.find_by_key(KEY_CONTENT_HASH, content_hash)? else {
            return Ok(None);
        };
        let version_id = scope
            .scope_read_node::<IdRow>(version_node)?
            .map(|row| row.id)
            .unwrap_or_default();
        let article_title = scope
            .incoming(version_node, EdgeKind::ArticleHoldVersion)?
            .first()
            .and_then(|article| scope.scope_read_node::<ArticleRow>(*article).transpose())
            .transpose()?
            .map(|row| row.title)
            .unwrap_or_default();
        Ok(Some(ContentHashOwner {
            version_id,
            article_title,
        }))
    })
}

pub fn parent_article_of(db: &Database, version_id: &str) -> Result<Option<String>, Error> {
    db.read(|scope| {
        let Some(version) = scope.resolve(NodeKind::Version, version_id)? else {
            return Ok(None);
        };
        Ok(scope
            .incoming(version, EdgeKind::ArticleHoldVersion)?
            .first()
            .and_then(|article| scope.scope_read_node::<IdRow>(*article).transpose())
            .transpose()?
            .map(|row| row.id))
    })
}
