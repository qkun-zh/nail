use std::collections::{HashMap, HashSet};

use agdb::{DbError, QueryBuilder};
use nail_common::tag::TagRef;

use crate::repository::graph::{
    DbHandle, find_by_index_in_txn, insert_edge, read_rows_sync, resolve_node_id_in_txn,
    resolve_node_id_sync,
};
use crate::repository::schema::{
    ArticleRow, EDGE_ARTICLE_TO_TAG, EDGE_ARTICLE_TO_VERSION, EDGE_USER_TO_ARTICLE,
    ENTITY_TYPE_ARTICLE, ENTITY_TYPE_TAG, ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, IdRow,
    KEY_CONTENT_HASH, KEY_SUMMARY, KEY_TITLE, KEY_TYPE, TagRow, UserRow, VersionRow, alias_of,
};
use crate::repository::tag::create_tag_in_txn;
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
    pub tags: Vec<TagRef>,
    pub latest_version: String,
    pub latest_version_id: String,
}

#[derive(Debug)]
pub enum CreateArticleError {
    AuthorMissing,
    TitleTaken,
    ContentHashTaken,
    Db(DbError),
}

impl From<DbError> for CreateArticleError {
    fn from(error: DbError) -> Self {
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
    Db(DbError),
}

impl From<DbError> for UpdateArticleError {
    fn from(error: DbError) -> Self {
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

pub async fn create_article(db: &DbHandle, draft: &ArticleDraft) -> Result<(), CreateArticleError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        if resolve_node_id_in_txn(transaction, ENTITY_TYPE_USER, &draft.author_id)?.is_none() {
            return Err(CreateArticleError::AuthorMissing);
        }
        if !find_by_index_in_txn(transaction, KEY_TITLE, &draft.title)?.is_empty() {
            return Err(CreateArticleError::TitleTaken);
        }
        if !find_by_index_in_txn(
            transaction,
            KEY_CONTENT_HASH,
            &draft.first_version.content_hash,
        )?
        .is_empty()
        {
            return Err(CreateArticleError::ContentHashTaken);
        }

        let mut seen_tags = HashSet::new();
        let mut tag_ids: Vec<String> = Vec::with_capacity(draft.tags.len());
        for name in &draft.tags {
            if seen_tags.insert(name) {
                let tag = create_tag_in_txn(transaction, name)?;
                tag_ids.push(tag.id);
            }
        }

        transaction.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([alias_of(
                    ENTITY_TYPE_VERSION,
                    &draft.first_version.version_id,
                )])
                .values(VersionRow {
                    db_id: None,
                    entity_type: ENTITY_TYPE_VERSION.to_string(),
                    id: draft.first_version.version_id.clone(),
                    version_number: draft.first_version.version_number.clone(),
                    content_hash: draft.first_version.content_hash.clone(),
                    note: draft.first_version.note.clone(),
                })
                .query(),
        )?;

        let article_alias = alias_of(ENTITY_TYPE_ARTICLE, &draft.article_id);
        transaction.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([article_alias.clone()])
                .values(ArticleRow {
                    db_id: None,
                    entity_type: ENTITY_TYPE_ARTICLE.to_string(),
                    id: draft.article_id.clone(),
                    title: draft.title.clone(),
                    summary: draft.summary.clone(),
                    latest_version_id: Some(draft.first_version.version_id.clone()),
                })
                .query(),
        )?;

        insert_edge(
            transaction,
            EDGE_USER_TO_ARTICLE,
            alias_of(ENTITY_TYPE_USER, &draft.author_id).into(),
            article_alias.clone().into(),
        )?;
        insert_edge(
            transaction,
            EDGE_ARTICLE_TO_VERSION,
            article_alias.clone().into(),
            alias_of(ENTITY_TYPE_VERSION, &draft.first_version.version_id).into(),
        )?;
        for tag_id in &tag_ids {
            insert_edge(
                transaction,
                EDGE_ARTICLE_TO_TAG,
                article_alias.clone().into(),
                alias_of(ENTITY_TYPE_TAG, tag_id).into(),
            )?;
        }
        Ok(())
    })
}

pub async fn read_article(db: &DbHandle, article_id: &str) -> Result<Option<ArticleView>, DbError> {
    let guard = db.read().await;
    let Some(id) = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(None);
    };
    Ok(enrich_articles(&guard, &[id])?.into_iter().next())
}

pub async fn read_articles(
    db: &DbHandle,
    limit: u64,
    offset: u64,
) -> Result<(Vec<ArticleView>, u64), DbError> {
    let guard = db.read().await;
    let result = guard.exec(
        QueryBuilder::search()
            .elements()
            .order_by([agdb::DbKeyOrder::Desc(agdb::DbValue::String(
                crate::repository::schema::KEY_ID.to_string(),
            ))])
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_ARTICLE)
            .query(),
    )?;
    let total = result.elements.len() as u64;
    let page_ids: Vec<agdb::DbId> = result
        .elements
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(|element| element.id)
        .collect();
    let items = enrich_articles(&guard, &page_ids)?;
    Ok((items, total))
}

pub async fn update_article(
    db: &DbHandle,
    article_id: &str,
    update: &ArticleUpdate,
) -> Result<(), UpdateArticleError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let Some(article) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_ARTICLE, article_id)?
        else {
            return Err(UpdateArticleError::Missing);
        };
        let title_conflict = find_by_index_in_txn(transaction, KEY_TITLE, &update.title)?
            .into_iter()
            .any(|other| other != article);
        if title_conflict {
            return Err(UpdateArticleError::TitleTaken);
        }

        transaction.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .ids([article])
                .values([[
                    (KEY_TITLE, update.title.as_str()).into(),
                    (KEY_SUMMARY, update.summary.as_str()).into(),
                ]])
                .query(),
        )?;

        let old_edges = transaction.exec(
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
        let old_ids: HashSet<agdb::DbId> = old_edges.elements.iter().map(|edge| edge.to).collect();

        let mut seen_tags = HashSet::new();
        let mut new_ids: Vec<agdb::DbId> = Vec::with_capacity(update.tags.len());
        for name in &update.tags {
            if seen_tags.insert(name) {
                let tag = create_tag_in_txn(transaction, name)?;
                let tag_id = resolve_node_id_in_txn(transaction, ENTITY_TYPE_TAG, &tag.id)?
                    .ok_or(UpdateArticleError::Missing)?;
                new_ids.push(tag_id);
            }
        }

        for tag_id in &new_ids {
            if !old_ids.contains(tag_id) {
                insert_edge(
                    transaction,
                    EDGE_ARTICLE_TO_TAG,
                    article.into(),
                    (*tag_id).into(),
                )?;
            }
        }

        let new_set: HashSet<agdb::DbId> = new_ids.iter().copied().collect();
        let stale_edge_ids: Vec<agdb::DbId> = old_edges
            .elements
            .iter()
            .filter(|edge| !new_set.contains(&edge.to))
            .map(|edge| edge.id)
            .collect();
        if !stale_edge_ids.is_empty() {
            transaction.exec_mut(QueryBuilder::remove().ids(stale_edge_ids).query())?;
        }

        let orphan_tags = transaction.exec(
            QueryBuilder::search()
                .elements()
                .where_()
                .key(KEY_TYPE)
                .value(ENTITY_TYPE_TAG)
                .and()
                .edge_count_to(agdb::CountComparison::Equal(0))
                .query(),
        )?;
        if !orphan_tags.elements.is_empty() {
            let ids: Vec<agdb::DbId> = orphan_tags.elements.iter().map(|edge| edge.id).collect();
            transaction.exec_mut(QueryBuilder::remove().ids(ids).query())?;
        }
        Ok(())
    })
}

#[cfg(test)]
pub async fn owner_of(db: &DbHandle, article_id: &str) -> Result<Option<String>, DbError> {
    let guard = db.read().await;
    let Some(article) = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(None);
    };
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
    Ok(edges.elements.first().and_then(|edge| {
        read_rows_sync::<IdRow>(&guard, &[edge.from])
            .ok()
            .and_then(|rows| rows.into_iter().next())
            .map(|row| row.id)
    }))
}

fn enrich_articles(guard: &agdb::DbAny, ids: &[agdb::DbId]) -> Result<Vec<ArticleView>, DbError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let article_rows = read_rows_sync::<ArticleRow>(guard, ids)?;
    let article_by_node: HashMap<agdb::DbId, ArticleRow> = article_rows
        .into_iter()
        .filter_map(|row| row.db_id.map(|node| (node, row)))
        .collect();

    let node_set: HashSet<agdb::DbId> = ids.iter().copied().collect();

    let owner_edges = guard.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(EDGE_USER_TO_ARTICLE)
            .query(),
    )?;
    let owner_of: HashMap<agdb::DbId, agdb::DbId> = owner_edges
        .elements
        .iter()
        .filter(|edge| node_set.contains(&edge.to))
        .map(|edge| (edge.to, edge.from))
        .collect();
    let owner_ids: Vec<agdb::DbId> = owner_of.values().copied().collect();
    let author_by_node: HashMap<agdb::DbId, UserRow> =
        read_rows_sync::<UserRow>(guard, &owner_ids)?
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row)))
            .collect();

    let tag_edges = guard.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_TO_TAG)
            .query(),
    )?;
    let tag_nodes: HashSet<agdb::DbId> = tag_edges.elements.iter().map(|edge| edge.to).collect();
    let tag_node_list: Vec<agdb::DbId> = tag_nodes.iter().copied().collect();
    let tag_name_by_node: HashMap<agdb::DbId, String> =
        read_rows_sync::<TagRow>(guard, &tag_node_list)?
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row.tag_name)))
            .collect();
    let tag_id_by_node: HashMap<agdb::DbId, String> =
        read_rows_sync::<IdRow>(guard, &tag_node_list)?
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row.id)))
            .collect();

    let mut tags_by_article: HashMap<agdb::DbId, Vec<TagRef>> = HashMap::new();
    for edge in &tag_edges.elements {
        if !node_set.contains(&edge.from) {
            continue;
        }
        let (Some(name), Some(id)) = (tag_name_by_node.get(&edge.to), tag_id_by_node.get(&edge.to))
        else {
            continue;
        };
        tags_by_article.entry(edge.from).or_default().push(TagRef {
            id: id.clone(),
            name: name.clone(),
        });
    }
    for tags in tags_by_article.values_mut() {
        tags.sort_by(|left, right| left.id.cmp(&right.id));
    }

    let latest_ids: Vec<String> = article_by_node
        .values()
        .filter_map(|row| row.latest_version_id.clone())
        .collect();
    let version_by_business = read_version_numbers(guard, &latest_ids)?;

    let mut items = Vec::with_capacity(ids.len());
    for id in ids {
        let Some(row) = article_by_node.get(id) else {
            continue;
        };
        let owner = owner_of.get(id).and_then(|node| author_by_node.get(node));
        let author_id = owner.map(|row| row.id.clone()).unwrap_or_default();
        let author_name = owner.map(|row| row.name.clone()).unwrap_or_default();
        let latest_version_id = row.latest_version_id.clone().unwrap_or_default();
        let latest_version = version_by_business
            .get(latest_version_id.as_str())
            .cloned()
            .unwrap_or_default();
        items.push(ArticleView {
            id: row.id.clone(),
            title: row.title.clone(),
            summary: row.summary.clone(),
            author_id,
            author_name,
            tags: tags_by_article.remove(id).unwrap_or_default(),
            latest_version,
            latest_version_id,
        });
    }
    Ok(items)
}

fn read_version_numbers(
    guard: &agdb::DbAny,
    version_ids: &[String],
) -> Result<HashMap<String, String>, DbError> {
    let mut resolved = Vec::with_capacity(version_ids.len());
    for version_id in version_ids {
        if let Some(node) = resolve_node_id_sync(guard, ENTITY_TYPE_VERSION, version_id)? {
            resolved.push(node);
        }
    }
    let rows = read_rows_sync::<VersionRow>(guard, &resolved)?;
    let mut map = HashMap::new();
    for row in rows {
        map.insert(row.id, row.version_number);
    }
    Ok(map)
}
