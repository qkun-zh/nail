use std::collections::HashSet;

use agdb::{DbError, QueryBuilder};

use crate::repository::graph::{
    DbHandle, insert_edge, read_node_in_txn, read_rows_sync, resolve_node_id_in_txn,
    resolve_node_id_sync,
};
use crate::repository::schema::{
    CommentRow, EDGE_COMMENT_TO_COMMENT, EDGE_COMMENT_TO_VERSION, EDGE_USER_TO_COMMENT,
    ENTITY_TYPE_COMMENT, ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, IdRow, KEY_COMMENT_CONTENT,
    KEY_TYPE, alias_of,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentTreeItem {
    pub id: String,
    pub content: String,
    pub author_id: String,
    pub parent_id: Option<String>,
}

#[derive(Debug)]
pub enum CreateCommentError {
    TargetNotFound,
    CommentIdExists,
    CommentTreeTooDeep,
    Db(DbError),
}

impl From<DbError> for CreateCommentError {
    fn from(error: DbError) -> Self {
        Self::Db(error)
    }
}

impl std::fmt::Display for CreateCommentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetNotFound => formatter.write_str("comment target not found"),
            Self::CommentIdExists => formatter.write_str("comment id already exists"),
            Self::CommentTreeTooDeep => formatter.write_str("comment thread too deep"),
            Self::Db(error) => write!(formatter, "database query failed: {error}"),
        }
    }
}

impl std::error::Error for CreateCommentError {}

pub async fn create_top_level_comment(
    db: &DbHandle,
    comment_id: &str,
    user_id: &str,
    version_id: &str,
    content: &str,
) -> Result<(), CreateCommentError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let Some(user) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_USER, user_id)? else {
            return Err(CreateCommentError::TargetNotFound);
        };
        let Some(version) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_VERSION, version_id)?
        else {
            return Err(CreateCommentError::TargetNotFound);
        };
        if resolve_node_id_in_txn(transaction, ENTITY_TYPE_COMMENT, comment_id)?.is_some() {
            return Err(CreateCommentError::CommentIdExists);
        }
        let comment_alias = alias_of(ENTITY_TYPE_COMMENT, comment_id);
        transaction.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([comment_alias.clone()])
                .values(CommentRow {
                    db_id: None,
                    entity_type: ENTITY_TYPE_COMMENT.to_string(),
                    id: comment_id.to_string(),
                    content: content.to_string(),
                })
                .query(),
        )?;
        insert_edge(
            transaction,
            EDGE_USER_TO_COMMENT,
            user.into(),
            comment_alias.clone().into(),
        )?;
        insert_edge(
            transaction,
            EDGE_COMMENT_TO_VERSION,
            comment_alias.into(),
            version.into(),
        )?;
        Ok(())
    })
}

pub async fn create_reply_comment(
    db: &DbHandle,
    comment_id: &str,
    user_id: &str,
    parent_comment_id: &str,
    content: &str,
    max_tree_depth: usize,
) -> Result<(), CreateCommentError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let Some(user) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_USER, user_id)? else {
            return Err(CreateCommentError::TargetNotFound);
        };
        let Some(parent) =
            resolve_node_id_in_txn(transaction, ENTITY_TYPE_COMMENT, parent_comment_id)?
        else {
            return Err(CreateCommentError::TargetNotFound);
        };
        if parent_chain_depth_in_txn(transaction, parent_comment_id, max_tree_depth)?
            >= max_tree_depth
        {
            return Err(CreateCommentError::CommentTreeTooDeep);
        }
        if resolve_node_id_in_txn(transaction, ENTITY_TYPE_COMMENT, comment_id)?.is_some() {
            return Err(CreateCommentError::CommentIdExists);
        }
        let comment_alias = alias_of(ENTITY_TYPE_COMMENT, comment_id);
        transaction.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([comment_alias.clone()])
                .values(CommentRow {
                    db_id: None,
                    entity_type: ENTITY_TYPE_COMMENT.to_string(),
                    id: comment_id.to_string(),
                    content: content.to_string(),
                })
                .query(),
        )?;
        insert_edge(
            transaction,
            EDGE_USER_TO_COMMENT,
            user.into(),
            comment_alias.clone().into(),
        )?;
        insert_edge(
            transaction,
            EDGE_COMMENT_TO_COMMENT,
            comment_alias.into(),
            parent.into(),
        )?;
        Ok(())
    })
}

fn parent_chain_depth_in_txn(
    transaction: &mut agdb::DbAnyTransactionMut,
    comment_id: &str,
    max_tree_depth: usize,
) -> Result<usize, DbError> {
    let mut depth = 0usize;
    let mut current = comment_id.to_string();
    loop {
        let Some(current_node) =
            resolve_node_id_in_txn(transaction, ENTITY_TYPE_COMMENT, &current)?
        else {
            return Ok(depth);
        };
        let edges = transaction.exec(
            QueryBuilder::search()
                .from(current_node)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_COMMENT_TO_COMMENT)
                .query(),
        )?;
        let Some(parent_node) = edges.elements.first().map(|edge| edge.to) else {
            return Ok(depth);
        };
        let Some(parent_id) =
            read_node_in_txn::<IdRow>(transaction, parent_node)?.map(|row| row.id)
        else {
            return Ok(depth);
        };
        current = parent_id;
        depth += 1;
        if depth > max_tree_depth {
            return Ok(depth);
        }
    }
}

pub async fn owner_of_comment(db: &DbHandle, comment_id: &str) -> Result<Option<String>, DbError> {
    let guard = db.read().await;
    let Some(comment) = resolve_node_id_sync(&guard, ENTITY_TYPE_COMMENT, comment_id)? else {
        return Ok(None);
    };
    let edges = guard.exec(
        QueryBuilder::search()
            .to(comment)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_TO_COMMENT)
            .query(),
    )?;
    Ok(edges.elements.first().and_then(|edge| {
        read_rows_sync::<IdRow>(&guard, &[edge.from])
            .ok()
            .and_then(|rows| rows.into_iter().next())
            .map(|row| row.id)
    }))
}

pub async fn read_comments_page_by_version(
    db: &DbHandle,
    version_id: &str,
    max_tree_depth: usize,
    limit: u64,
    offset: u64,
) -> Result<(Vec<CommentTreeItem>, u64), DbError> {
    let guard = db.read().await;
    let mut out: Vec<CommentTreeItem> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let Some(version) = resolve_node_id_sync(&guard, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok((out, 0));
    };
    let top_edges = guard.exec(
        QueryBuilder::search()
            .to(version)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_COMMENT_TO_VERSION)
            .query(),
    )?;
    let mut top_ids: Vec<String> = top_edges
        .elements
        .iter()
        .filter_map(|edge| {
            read_rows_sync::<IdRow>(&guard, &[edge.from])
                .ok()
                .and_then(|rows| rows.into_iter().next())
                .map(|row| row.id)
        })
        .collect();
    top_ids.sort_by(|left, right| right.cmp(left));
    let total = top_ids.len() as u64;
    let page_ids: Vec<String> = top_ids
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();
    if page_ids.is_empty() {
        return Ok((out, total));
    }

    let top_rows = read_comment_items(&guard, &page_ids)?;
    for item in &top_rows {
        seen.insert(item.id.clone());
    }
    out.extend(top_rows);

    let mut depth = 0usize;
    let mut parents: Vec<String> = page_ids;
    while !parents.is_empty() {
        if depth > max_tree_depth {
            break;
        }
        depth += 1;
        let mut kids: Vec<String> = Vec::new();
        for parent_id in &parents {
            let Some(parent) = resolve_node_id_sync(&guard, ENTITY_TYPE_COMMENT, parent_id)? else {
                continue;
            };
            let reply_edges = guard.exec(
                QueryBuilder::search()
                    .to(parent)
                    .where_()
                    .distance(agdb::CountComparison::Equal(1))
                    .and()
                    .edge()
                    .and()
                    .key(KEY_TYPE)
                    .value(EDGE_COMMENT_TO_COMMENT)
                    .query(),
            )?;
            for edge in &reply_edges.elements {
                if let Some(kid_id) = read_rows_sync::<IdRow>(&guard, &[edge.from])?
                    .into_iter()
                    .next()
                    .map(|row| row.id)
                    && seen.insert(kid_id.clone())
                {
                    kids.push(kid_id);
                }
            }
        }
        kids.sort();
        let rows = read_comment_items(&guard, &kids)?;
        out.extend(rows);
        parents = kids;
    }
    Ok((out, total))
}

fn read_comment_items(
    guard: &agdb::DbAny,
    comment_ids: &[String],
) -> Result<Vec<CommentTreeItem>, DbError> {
    let mut items = Vec::with_capacity(comment_ids.len());
    for comment_id in comment_ids {
        let Some(comment) = resolve_node_id_sync(guard, ENTITY_TYPE_COMMENT, comment_id)? else {
            continue;
        };
        let content = read_rows_sync::<CommentRow>(guard, &[comment])?
            .into_iter()
            .next()
            .map(|row| row.content)
            .unwrap_or_default();
        let author_id = read_incoming_node_id(guard, comment, EDGE_USER_TO_COMMENT)?;
        let parent_id = read_outgoing_node_id(guard, comment, EDGE_COMMENT_TO_COMMENT)?;
        items.push(CommentTreeItem {
            id: comment_id.clone(),
            content,
            author_id,
            parent_id,
        });
    }
    Ok(items)
}

fn read_incoming_node_id(
    guard: &agdb::DbAny,
    node: agdb::DbId,
    edge_type: &str,
) -> Result<String, DbError> {
    let edges = guard.exec(
        QueryBuilder::search()
            .to(node)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(edge_type)
            .query(),
    )?;
    Ok(edges
        .elements
        .first()
        .and_then(|edge| {
            read_rows_sync::<IdRow>(guard, &[edge.from])
                .ok()
                .and_then(|rows| rows.into_iter().next())
                .map(|row| row.id)
        })
        .unwrap_or_default())
}

fn read_outgoing_node_id(
    guard: &agdb::DbAny,
    node: agdb::DbId,
    edge_type: &str,
) -> Result<Option<String>, DbError> {
    let edges = guard.exec(
        QueryBuilder::search()
            .from(node)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(edge_type)
            .query(),
    )?;
    Ok(edges.elements.first().and_then(|edge| {
        read_rows_sync::<IdRow>(guard, &[edge.to])
            .ok()
            .and_then(|rows| rows.into_iter().next())
            .map(|row| row.id)
    }))
}

pub async fn update_comment_content(
    db: &DbHandle,
    comment_id: &str,
    content: &str,
) -> Result<bool, DbError> {
    let mut guard = db.write().await;
    let Some(node) = resolve_node_id_sync(&guard, ENTITY_TYPE_COMMENT, comment_id)? else {
        return Ok(false);
    };
    guard.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .ids([node])
            .values([[(KEY_COMMENT_CONTENT, content).into()]])
            .query(),
    )?;
    Ok(true)
}

pub async fn version_of_comment(
    db: &DbHandle,
    comment_id: &str,
) -> Result<Option<String>, DbError> {
    let guard = db.read().await;
    let mut current = comment_id.to_string();
    loop {
        let Some(comment) = resolve_node_id_sync(&guard, ENTITY_TYPE_COMMENT, &current)? else {
            return Ok(None);
        };
        let parent_edges = guard.exec(
            QueryBuilder::search()
                .from(comment)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_COMMENT_TO_COMMENT)
                .query(),
        )?;
        if let Some(parent_node) = parent_edges.elements.first().map(|edge| edge.to)
            && let Some(parent_id) = read_rows_sync::<IdRow>(&guard, &[parent_node])?
                .into_iter()
                .next()
                .map(|row| row.id)
        {
            current = parent_id;
            continue;
        }
        let version_edges = guard.exec(
            QueryBuilder::search()
                .from(comment)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_COMMENT_TO_VERSION)
                .query(),
        )?;
        return Ok(version_edges.elements.first().and_then(|edge| {
            read_rows_sync::<IdRow>(&guard, &[edge.to])
                .ok()
                .and_then(|rows| rows.into_iter().next())
                .map(|row| row.id)
        }));
    }
}
