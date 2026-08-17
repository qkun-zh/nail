use std::collections::{HashMap, HashSet};

use agdb::{DbError, QueryBuilder};

use crate::repository::graph::{DbHandle, read_rows_sync, resolve_node_id_sync};
use crate::repository::schema::{
    ArticleRow, EDGE_ARTICLE_HOLD_VERSION, EDGE_COMMENT_ATTACH_VERSION, EDGE_USER_AUTHOR_ARTICLE,
    EDGE_USER_AUTHOR_COMMENT, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, IdRow,
    KEY_SOFT_DELETED, KEY_TYPE, UserRow, VersionRow,
};

use super::SearchCommentOutcome;

pub(super) async fn enrich_comment_headers(
    db: &DbHandle,
    comments: &mut [SearchCommentOutcome],
) -> anyhow::Result<()> {
    if comments.is_empty() {
        return Ok(());
    }
    let guard = db.read().await;

    let article_ids: HashSet<String> = comments.iter().map(|c| c.article_id.clone()).collect();
    let version_ids: HashSet<String> = comments.iter().map(|c| c.version_id.clone()).collect();

    let mut article_by_id: HashMap<String, agdb::DbId> = HashMap::new();
    for id in &article_ids {
        if let Some(node) = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, id)? {
            article_by_id.insert(id.clone(), node);
        }
    }
    let mut version_by_id: HashMap<String, agdb::DbId> = HashMap::new();
    for id in &version_ids {
        if let Some(node) = resolve_node_id_sync(&guard, ENTITY_TYPE_VERSION, id)? {
            version_by_id.insert(id.clone(), node);
        }
    }

    let article_nodes: Vec<agdb::DbId> = article_by_id.values().copied().collect();
    let title_by_node: HashMap<agdb::DbId, String> =
        read_rows_sync::<ArticleRow>(&guard, &article_nodes)?
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row.title)))
            .collect();

    let version_nodes: Vec<agdb::DbId> = version_by_id.values().copied().collect();
    let version_number_by_node: HashMap<agdb::DbId, String> =
        read_rows_sync::<VersionRow>(&guard, &version_nodes)?
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row.version_number)))
            .collect();

    let mut author_by_article: HashMap<agdb::DbId, agdb::DbId> = HashMap::new();
    let mut user_nodes: Vec<agdb::DbId> = Vec::new();
    for article_node in &article_nodes {
        let edges = guard.exec(
            QueryBuilder::search()
                .to(*article_node)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_USER_AUTHOR_ARTICLE)
                .query(),
        )?;
        if let Some(edge) = edges.elements.first() {
            author_by_article.insert(*article_node, edge.from);
            user_nodes.push(edge.from);
        }
    }
    let author_name_by_node: HashMap<agdb::DbId, String> =
        read_rows_sync::<UserRow>(&guard, &user_nodes)?
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row.name)))
            .collect();

    for comment in comments.iter_mut() {
        let article_node = article_by_id.get(comment.article_id.as_str());
        comment.article_title = article_node
            .and_then(|node| title_by_node.get(node))
            .cloned()
            .unwrap_or_default();
        comment.article_author_name = article_node
            .and_then(|node| author_by_article.get(node))
            .and_then(|user_node| author_name_by_node.get(user_node))
            .cloned()
            .unwrap_or_default();
        comment.version_number = version_by_id
            .get(comment.version_id.as_str())
            .and_then(|node| version_number_by_node.get(node))
            .cloned()
            .unwrap_or_default();
    }
    Ok(())
}

pub(super) async fn article_ids_of_user(
    db: &DbHandle,
    user_id: &str,
) -> Result<Vec<String>, DbError> {
    let guard = db.read().await;
    let Some(user) = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)? else {
        return Ok(Vec::new());
    };
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    let articles = guard.exec(
        QueryBuilder::search()
            .from(user)
            .where_()
            .distance(agdb::CountComparison::Equal(2))
            .and()
            .node()
            .and()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_ARTICLE)
            .and()
            .not()
            .keys(KEY_SOFT_DELETED)
            .query(),
    )?;
    for element in &articles.elements {
        if let Some(row) = read_rows_sync::<IdRow>(&guard, &[element.id])?
            .into_iter()
            .next()
            && seen.insert(row.id.clone())
        {
            ids.push(row.id);
        }
    }

    let comment_edges = guard.exec(
        QueryBuilder::search()
            .from(user)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_AUTHOR_COMMENT)
            .query(),
    )?;
    for edge in &comment_edges.elements {
        if let Some(article_id) = article_id_of_comment(&guard, edge.to)?
            && seen.insert(article_id.clone())
        {
            ids.push(article_id);
        }
    }
    Ok(ids)
}

fn article_id_of_comment(
    guard: &agdb::DbAny,
    comment: agdb::DbId,
) -> Result<Option<String>, DbError> {
    let version_edges = guard.exec(
        QueryBuilder::search()
            .from(comment)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_COMMENT_ATTACH_VERSION)
            .query(),
    )?;
    let Some(version_edge) = version_edges.elements.first() else {
        return Ok(None);
    };
    let article_edges = guard.exec(
        QueryBuilder::search()
            .to(version_edge.to)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_HOLD_VERSION)
            .query(),
    )?;
    let Some(article_edge) = article_edges.elements.first() else {
        return Ok(None);
    };
    if crate::repository::delete::has_soft_deleted_flag(guard, article_edge.from)? {
        return Ok(None);
    }
    Ok(read_rows_sync::<IdRow>(guard, &[article_edge.from])?
        .into_iter()
        .next()
        .map(|row| row.id))
}

pub(super) async fn all_article_ids(db: &DbHandle) -> Result<Vec<String>, DbError> {
    let guard = db.read().await;
    let all = guard.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_ARTICLE)
            .and()
            .not()
            .keys(KEY_SOFT_DELETED)
            .query(),
    )?;
    let mut ids = Vec::with_capacity(all.elements.len());
    for element in &all.elements {
        if let Some(row) = read_rows_sync::<IdRow>(&guard, &[element.id])?
            .into_iter()
            .next()
        {
            ids.push(row.id);
        }
    }
    Ok(ids)
}
