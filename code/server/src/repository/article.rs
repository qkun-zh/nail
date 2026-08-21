use std::collections::{HashMap, HashSet};

use common::response::NamedRef;
use database::{Database, EdgeKind, Error, NodeId, NodeKind, Value, WriteScope};

use crate::repository::access::GraphRead;
use crate::repository::delete::{has_soft_deleted_flag, highest_version_number};
use crate::repository::schema::{
    ArticleRow, KEY_CONTENT_HASH, KEY_SUMMARY, KEY_TITLE, TagRow, UserRow, VersionRow,
};
use crate::repository::tag::create_tag_in_scope;
use crate::repository::version::VersionDraft;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleDraft {
    pub article_id: String,
    pub author_id: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub first_version: VersionDraft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleUpdate {
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleView {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub author_id: String,
    pub author_name: String,
    pub tags: Vec<NamedRef>,
    pub latest_version: String,
    pub latest_version_id: String,
}

#[derive(Debug)]
pub enum CreateArticleError {
    AuthorMissing,
    TitleTaken,
    ContentHashTaken,
    Db(Error),
}

impl From<Error> for CreateArticleError {
    fn from(error: Error) -> Self {
        Self::Db(error)
    }
}

impl std::fmt::Display for CreateArticleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthorMissing => formatter.write_str("author not found"),
            Self::TitleTaken => formatter.write_str("title already exists"),
            Self::ContentHashTaken => formatter.write_str("identical content already exists"),
            Self::Db(error) => write!(formatter, "database query failed: {error}"),
        }
    }
}

impl std::error::Error for CreateArticleError {}

#[derive(Debug)]
pub enum UpdateArticleError {
    Missing,
    TitleTaken,
    Db(Error),
}

impl From<Error> for UpdateArticleError {
    fn from(error: Error) -> Self {
        Self::Db(error)
    }
}

impl std::fmt::Display for UpdateArticleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => formatter.write_str("article not found"),
            Self::TitleTaken => formatter.write_str("title already exists"),
            Self::Db(error) => write!(formatter, "database query failed: {error}"),
        }
    }
}

impl std::error::Error for UpdateArticleError {}

pub fn create_article(db: &Database, draft: &ArticleDraft) -> Result<(), CreateArticleError> {
    db.write(|scope| {
        let Some(author) = scope.resolve(NodeKind::User, &draft.author_id)? else {
            return Ok(Err(CreateArticleError::AuthorMissing));
        };
        if scope.find_by_key(KEY_TITLE, &draft.title)?.is_some() {
            return Ok(Err(CreateArticleError::TitleTaken));
        }
        if scope
            .find_by_key(KEY_CONTENT_HASH, &draft.first_version.content_hash)?
            .is_some()
        {
            return Ok(Err(CreateArticleError::ContentHashTaken));
        }

        let mut seen_tags = HashSet::new();
        let mut tag_nodes: Vec<NodeId> = Vec::with_capacity(draft.tags.len());
        for name in &draft.tags {
            if seen_tags.insert(name) {
                let tag = create_tag_in_scope(scope, name)?;
                let node = scope
                    .resolve(NodeKind::Tag, &tag.id)?
                    .ok_or_else(|| Error::Invalid("inserted tag missing".to_string()))?;
                tag_nodes.push(node);
            }
        }

        scope.insert_node(&VersionRow {
            id: draft.first_version.version_id.clone(),
            version_number: draft.first_version.version_number.clone(),
            content_hash: draft.first_version.content_hash.clone(),
            note: draft.first_version.note.clone(),
        })?;
        let version_node = scope
            .resolve(NodeKind::Version, &draft.first_version.version_id)?
            .ok_or_else(|| Error::Invalid("inserted version missing".to_string()))?;

        scope.insert_node(&ArticleRow {
            id: draft.article_id.clone(),
            title: draft.title.clone(),
            summary: draft.summary.clone(),
            latest_version_id: Some(draft.first_version.version_id.clone()),
        })?;
        let article_node = scope
            .resolve(NodeKind::Article, &draft.article_id)?
            .ok_or_else(|| Error::Invalid("inserted article missing".to_string()))?;

        scope.insert_edge(
            NodeKind::User,
            author,
            EdgeKind::UserAuthorArticle,
            NodeKind::Article,
            article_node,
        )?;
        scope.insert_edge(
            NodeKind::Article,
            article_node,
            EdgeKind::ArticleHoldVersion,
            NodeKind::Version,
            version_node,
        )?;
        for tag_node in &tag_nodes {
            scope.insert_edge(
                NodeKind::Article,
                article_node,
                EdgeKind::ArticleApplyTag,
                NodeKind::Tag,
                *tag_node,
            )?;
        }
        Ok(Ok(()))
    })
    .map_err(CreateArticleError::from)
    .and_then(std::convert::identity)
}

pub fn read_article(db: &Database, article_id: &str) -> Result<Option<ArticleView>, Error> {
    db.read(|scope| {
        let Some(id) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok(None);
        };
        Ok(enrich_articles(scope, &[id])?.into_iter().next())
    })
}

pub fn article_exists(db: &Database, article_id: &str) -> Result<bool, Error> {
    db.read(|scope| Ok(scope.resolve(NodeKind::Article, article_id)?.is_some()))
}

pub fn update_article(
    db: &Database,
    article_id: &str,
    update: &ArticleUpdate,
) -> Result<(), UpdateArticleError> {
    db.write(|scope| {
        let Some(article) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok(Err(UpdateArticleError::Missing));
        };
        let title_conflict = scope
            .find_by_key(KEY_TITLE, &update.title)?
            .is_some_and(|other| other != article);
        if title_conflict {
            return Ok(Err(UpdateArticleError::TitleTaken));
        }

        scope.set_key(article, KEY_TITLE, Value::Text(update.title.clone()))?;
        scope.set_key(article, KEY_SUMMARY, Value::Text(update.summary.clone()))?;

        let old_edges = scope.outgoing(article, EdgeKind::ArticleApplyTag)?;
        let old_ids: HashSet<NodeId> = old_edges.iter().copied().collect();

        let mut seen_tags = HashSet::new();
        let mut new_ids: Vec<NodeId> = Vec::with_capacity(update.tags.len());
        for name in &update.tags {
            if seen_tags.insert(name) {
                let tag = create_tag_in_scope(scope, name)?;
                let Some(tag_id) = scope.resolve(NodeKind::Tag, &tag.id)? else {
                    return Ok(Err(UpdateArticleError::Missing));
                };
                new_ids.push(tag_id);
            }
        }

        for tag_id in &new_ids {
            if !old_ids.contains(tag_id) {
                scope.insert_edge(
                    NodeKind::Article,
                    article,
                    EdgeKind::ArticleApplyTag,
                    NodeKind::Tag,
                    *tag_id,
                )?;
            }
        }

        for stale in old_edges.iter().filter(|edge| !new_ids.contains(edge)) {
            scope.remove_edge(article, EdgeKind::ArticleApplyTag, *stale)?;
        }

        remove_orphan_tags(scope)?;
        Ok(Ok(()))
    })
    .map_err(UpdateArticleError::from)
    .and_then(std::convert::identity)
}

fn remove_orphan_tags(scope: &mut WriteScope<'_, '_>) -> Result<(), Error> {
    let tags = scope.all_nodes(NodeKind::Tag)?;
    let mut orphans = Vec::new();
    for tag in tags {
        if scope.count_incoming(tag, EdgeKind::ArticleApplyTag)? == 0 {
            orphans.push(tag);
        }
    }
    scope.remove(&orphans)
}

#[cfg(test)]
pub fn owner_of(db: &Database, article_id: &str) -> Result<Option<String>, Error> {
    db.read(|scope| {
        let Some(article) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok(None);
        };
        Ok(scope
            .incoming(article, EdgeKind::UserAuthorArticle)?
            .first()
            .and_then(|user| {
                scope
                    .scope_read_node::<crate::repository::schema::IdRow>(*user)
                    .transpose()
            })
            .transpose()?
            .map(|row| row.id))
    })
}

fn enrich_articles(scope: &impl GraphRead, ids: &[NodeId]) -> Result<Vec<ArticleView>, Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut article_by_node: HashMap<NodeId, ArticleRow> = HashMap::new();
    for &id in ids {
        if let Some(row) = scope.scope_read_node::<ArticleRow>(id)? {
            article_by_node.insert(id, row);
        }
    }

    let mut owner_of: HashMap<NodeId, NodeId> = HashMap::new();
    let mut tag_nodes: HashSet<NodeId> = HashSet::new();
    let mut tags_by_article: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for &article in ids {
        if !article_by_node.contains_key(&article) {
            continue;
        }
        if let Some(owner) = scope
            .scope_incoming(article, EdgeKind::UserAuthorArticle)?
            .first()
        {
            owner_of.insert(article, *owner);
        }
        let article_tag_nodes = scope.scope_outgoing(article, EdgeKind::ArticleApplyTag)?;
        for tag_node in &article_tag_nodes {
            tag_nodes.insert(*tag_node);
        }
        tags_by_article.insert(article, article_tag_nodes);
    }

    let owner_ids: Vec<NodeId> = owner_of.values().copied().collect();
    let mut author_by_node: HashMap<NodeId, UserRow> = HashMap::new();
    for owner in &owner_ids {
        if let Some(row) = scope.scope_read_node::<UserRow>(*owner)? {
            author_by_node.insert(*owner, row);
        }
    }

    let tag_node_list: Vec<NodeId> = tag_nodes.iter().copied().collect();
    let mut tag_name_by_node: HashMap<NodeId, String> = HashMap::new();
    let mut tag_id_by_node: HashMap<NodeId, String> = HashMap::new();
    for tag_node in &tag_node_list {
        if let Some(row) = scope.scope_read_node::<TagRow>(*tag_node)? {
            tag_name_by_node.insert(*tag_node, row.tag_name);
            tag_id_by_node.insert(*tag_node, row.id);
        }
    }

    let latest_ids: Vec<String> = article_by_node
        .values()
        .filter_map(|row| row.latest_version_id.clone())
        .collect();
    let version_by_business = read_version_numbers(scope, &latest_ids)?;

    let mut items = Vec::with_capacity(ids.len());
    for id in ids {
        let Some(row) = article_by_node.get(id) else {
            continue;
        };
        let owner = owner_of.get(id).and_then(|node| author_by_node.get(node));
        let author_id = owner.map(|row| row.id.clone()).unwrap_or_default();
        let author_name = owner.map(|row| row.name.clone()).unwrap_or_default();
        let stored_latest_id = row.latest_version_id.clone().unwrap_or_default();
        let mut latest_version_id = stored_latest_id.clone();
        let mut latest_version = version_by_business
            .get(stored_latest_id.as_str())
            .cloned()
            .unwrap_or_default();
        if latest_version.is_empty()
            && !stored_latest_id.is_empty()
            && let Some((live_id, live_number)) = live_latest_version(scope, *id)?
        {
            latest_version_id = live_id;
            latest_version = live_number;
        }
        if latest_version.is_empty() && !stored_latest_id.is_empty() {
            latest_version_id = String::new();
        }
        let mut tags: Vec<NamedRef> = tags_by_article
            .get(id)
            .map(|tag_nodes| {
                tag_nodes
                    .iter()
                    .filter_map(|tag_node| {
                        let name = tag_name_by_node.get(tag_node)?;
                        let tag_id = tag_id_by_node.get(tag_node)?;
                        Some(NamedRef {
                            id: tag_id.clone(),
                            name: name.clone(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        tags.sort_by(|left, right| left.id.cmp(&right.id));
        items.push(ArticleView {
            id: row.id.clone(),
            title: row.title.clone(),
            summary: row.summary.clone(),
            author_id,
            author_name,
            tags,
            latest_version,
            latest_version_id,
        });
    }
    Ok(items)
}

fn read_version_numbers(
    scope: &impl GraphRead,
    version_ids: &[String],
) -> Result<HashMap<String, String>, Error> {
    let mut resolved = Vec::with_capacity(version_ids.len());
    for version_id in version_ids {
        if let Some(node) = scope.scope_resolve(NodeKind::Version, version_id)?
            && !has_soft_deleted_flag(scope, node)?
        {
            resolved.push(node);
        }
    }
    let rows = scope.scope_read_nodes::<VersionRow>(&resolved)?;
    let mut map = HashMap::new();
    for row in rows {
        map.insert(row.id, row.version_number);
    }
    Ok(map)
}

fn live_latest_version(
    scope: &impl GraphRead,
    article: NodeId,
) -> Result<Option<(String, String)>, Error> {
    let nodes = scope.scope_outgoing(article, EdgeKind::ArticleHoldVersion)?;
    let rows = scope.scope_read_nodes::<VersionRow>(&nodes)?;
    let mut live = Vec::with_capacity(rows.len());
    for (node, row) in nodes.into_iter().zip(rows) {
        if !has_soft_deleted_flag(scope, node)? {
            live.push(row);
        }
    }
    Ok(highest_version_number(live).map(|row| (row.id, row.version_number)))
}

pub fn articles_of_user(
    db: &Database,
    user_id: &str,
) -> Result<Vec<common::response::article::ArticleListItem>, Error> {
    db.read(|scope| {
        let Some(user) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(Vec::new());
        };
        let edges = scope.outgoing(user, EdgeKind::UserAuthorArticle)?;
        let mut articles = Vec::new();
        for article in edges {
            if has_soft_deleted_flag(scope, article)? {
                continue;
            }
            if let Some(row) = scope.scope_read_node::<ArticleRow>(article)? {
                let created_at = common::time::uuidv7_timestamp_secs(&row.id).unwrap_or(0);
                articles.push(common::response::article::ArticleListItem {
                    id: row.id,
                    title: row.title,
                    created_at,
                });
            }
        }
        articles.sort_by_key(|article| std::cmp::Reverse(article.created_at));
        Ok(articles)
    })
}
