use agdb::{DbError, QueryBuilder};

use crate::repository::graph::{
    DbHandle, incoming_edges, insert_edge, outgoing_edges, read_node, read_rows, resolve_node_id,
};
use crate::repository::schema::{
    CommentRow, EDGE_COMMENT_ATTACH_VERSION, EDGE_COMMENT_REPLY_COMMENT, EDGE_USER_AUTHOR_COMMENT,
    ENTITY_TYPE_COMMENT, ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, IdRow, KEY_COMMENT_CONTENT,
    KEY_SOFT_DELETED, KEY_TYPE, alias_of,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentTreeItem {
    pub id: String,
    pub content: String,
    pub author_id: String,
    pub parent_id: Option<String>,
    pub child_count: u64,
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
        let Some(user) = resolve_node_id(transaction, ENTITY_TYPE_USER, user_id)? else {
            return Err(CreateCommentError::TargetNotFound);
        };
        let Some(version) = resolve_node_id(transaction, ENTITY_TYPE_VERSION, version_id)? else {
            return Err(CreateCommentError::TargetNotFound);
        };
        if crate::repository::delete::has_soft_deleted_flag(transaction, version)? {
            return Err(CreateCommentError::TargetNotFound);
        }
        if resolve_node_id(transaction, ENTITY_TYPE_COMMENT, comment_id)?.is_some() {
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
            EDGE_USER_AUTHOR_COMMENT,
            user.into(),
            comment_alias.clone().into(),
        )?;
        insert_edge(
            transaction,
            EDGE_COMMENT_ATTACH_VERSION,
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
        let Some(user) = resolve_node_id(transaction, ENTITY_TYPE_USER, user_id)? else {
            return Err(CreateCommentError::TargetNotFound);
        };
        let Some(parent) = resolve_node_id(transaction, ENTITY_TYPE_COMMENT, parent_comment_id)?
        else {
            return Err(CreateCommentError::TargetNotFound);
        };
        if crate::repository::delete::has_soft_deleted_flag(transaction, parent)? {
            return Err(CreateCommentError::TargetNotFound);
        }
        if parent_chain_depth_in_txn(transaction, parent_comment_id, max_tree_depth)?
            >= max_tree_depth
        {
            return Err(CreateCommentError::CommentTreeTooDeep);
        }
        if resolve_node_id(transaction, ENTITY_TYPE_COMMENT, comment_id)?.is_some() {
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
            EDGE_USER_AUTHOR_COMMENT,
            user.into(),
            comment_alias.clone().into(),
        )?;
        insert_edge(
            transaction,
            EDGE_COMMENT_REPLY_COMMENT,
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
        let Some(current_node) = resolve_node_id(transaction, ENTITY_TYPE_COMMENT, &current)?
        else {
            return Ok(depth);
        };
        let edges = outgoing_edges(transaction, current_node, EDGE_COMMENT_REPLY_COMMENT)?;
        let Some(parent_node) = edges.first().map(|edge| edge.to) else {
            return Ok(depth);
        };
        let Some(parent_id) = read_node::<IdRow>(transaction, parent_node)?.map(|row| row.id)
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
    let Some(comment) = resolve_node_id(&guard, ENTITY_TYPE_COMMENT, comment_id)? else {
        return Ok(None);
    };
    let edges = incoming_edges(&guard, comment, EDGE_USER_AUTHOR_COMMENT)?;
    Ok(edges.first().and_then(|edge| {
        read_rows::<IdRow>(&guard, &[edge.from])
            .ok()
            .and_then(|rows| rows.into_iter().next())
            .map(|row| row.id)
    }))
}

pub async fn read_comments_page_by_version(
    db: &DbHandle,
    version_id: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<CommentTreeItem>, bool), DbError> {
    let guard = db.read().await;
    let Some(version) = resolve_node_id(&guard, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok((Vec::new(), false));
    };
    if crate::repository::delete::has_soft_deleted_flag(&guard, version)? {
        return Ok((Vec::new(), false));
    }
    read_comments_page_sync(&guard, version, limit, offset)
}

fn read_comments_page_sync(
    guard: &agdb::DbAny,
    version: agdb::DbId,
    limit: u64,
    offset: u64,
) -> Result<(Vec<CommentTreeItem>, bool), DbError> {
    let (page_ids, has_next) = incoming_comment_ids_page(guard, version, limit, offset)?;
    let items = read_comment_items(guard, &page_ids)?;
    Ok((items, has_next))
}

pub async fn read_comment_children_page(
    db: &DbHandle,
    parent_comment_id: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<CommentTreeItem>, bool), DbError> {
    let guard = db.read().await;
    let Some(parent) = resolve_node_id(&guard, ENTITY_TYPE_COMMENT, parent_comment_id)? else {
        return Err(DbError::query(
            agdb::DbErrorType::NotFound,
            "parent comment not found",
        ));
    };
    if crate::repository::delete::has_soft_deleted_flag(&guard, parent)? {
        return Err(DbError::query(
            agdb::DbErrorType::NotFound,
            "parent comment not found",
        ));
    }
    let (page_ids, has_next) = incoming_comment_ids_page(&guard, parent, limit, offset)?;
    let items = read_comment_items(&guard, &page_ids)?;
    Ok((items, has_next))
}

pub async fn read_comment_item(
    db: &DbHandle,
    comment_id: &str,
) -> Result<Option<CommentTreeItem>, DbError> {
    let guard = db.read().await;
    read_comment_item_any_sync(&guard, comment_id)
}

fn incoming_comment_ids_page(
    guard: &agdb::DbAny,
    node: agdb::DbId,
    limit: u64,
    offset: u64,
) -> Result<(Vec<String>, bool), DbError> {
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    let comments = guard.exec(
        QueryBuilder::search()
            .to(node)
            .offset(offset)
            .limit(limit.saturating_add(1) as u64)
            .where_()
            .distance(agdb::CountComparison::Equal(2))
            .and()
            .node()
            .and()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_COMMENT)
            .and()
            .not()
            .keys(KEY_SOFT_DELETED)
            .query(),
    )?;
    let has_next = comments.elements.len() > limit;
    let mut ids = Vec::with_capacity(comments.elements.len().min(limit));
    for element in comments.elements.iter().take(limit) {
        if let Some(id) = read_rows::<IdRow>(guard, &[element.id])?
            .into_iter()
            .next()
            .map(|row| row.id)
        {
            ids.push(id);
        }
    }
    Ok((ids, has_next))
}

fn child_count_sync(guard: &agdb::DbAny, comment: agdb::DbId) -> Result<u64, DbError> {
    let edges = incoming_edges(guard, comment, EDGE_COMMENT_REPLY_COMMENT)?;
    Ok(edges.len() as u64)
}

fn read_comment_item_any_sync(
    guard: &agdb::DbAny,
    comment_id: &str,
) -> Result<Option<CommentTreeItem>, DbError> {
    let Some(comment) = resolve_node_id(guard, ENTITY_TYPE_COMMENT, comment_id)? else {
        return Ok(None);
    };
    let content = read_rows::<CommentRow>(guard, &[comment])?
        .into_iter()
        .next()
        .map(|row| row.content)
        .unwrap_or_default();
    let author_id = read_incoming_node_id(guard, comment, EDGE_USER_AUTHOR_COMMENT)?;
    let parent_id = read_outgoing_node_id(guard, comment, EDGE_COMMENT_REPLY_COMMENT)?;
    let child_count = child_count_sync(guard, comment)?;
    Ok(Some(CommentTreeItem {
        id: comment_id.to_string(),
        content,
        author_id,
        parent_id,
        child_count,
    }))
}

fn read_comment_item_sync(
    guard: &agdb::DbAny,
    comment_id: &str,
) -> Result<Option<CommentTreeItem>, DbError> {
    let Some(comment) = resolve_node_id(guard, ENTITY_TYPE_COMMENT, comment_id)? else {
        return Ok(None);
    };
    if crate::repository::delete::has_soft_deleted_flag(guard, comment)? {
        return Ok(None);
    }
    read_comment_item_any_sync(guard, comment_id)
}

fn read_comment_items(
    guard: &agdb::DbAny,
    comment_ids: &[String],
) -> Result<Vec<CommentTreeItem>, DbError> {
    let mut items = Vec::with_capacity(comment_ids.len());
    for comment_id in comment_ids {
        if let Some(item) = read_comment_item_sync(guard, comment_id)? {
            items.push(item);
        }
    }
    Ok(items)
}

fn read_incoming_node_id(
    guard: &agdb::DbAny,
    node: agdb::DbId,
    edge_type: &str,
) -> Result<String, DbError> {
    let edges = incoming_edges(guard, node, edge_type)?;
    Ok(edges
        .first()
        .and_then(|edge| {
            read_rows::<IdRow>(guard, &[edge.from])
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
    let edges = outgoing_edges(guard, node, edge_type)?;
    Ok(edges.first().and_then(|edge| {
        read_rows::<IdRow>(guard, &[edge.to])
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
    let Some(node) = resolve_node_id(&guard, ENTITY_TYPE_COMMENT, comment_id)? else {
        return Ok(false);
    };
    if crate::repository::delete::has_soft_deleted_flag(&guard, node)? {
        return Ok(false);
    }
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
        let Some(comment) = resolve_node_id(&guard, ENTITY_TYPE_COMMENT, &current)? else {
            return Ok(None);
        };
        let parent_edges = outgoing_edges(&guard, comment, EDGE_COMMENT_REPLY_COMMENT)?;
        if let Some(parent_node) = parent_edges.first().map(|edge| edge.to)
            && let Some(parent_id) = read_rows::<IdRow>(&guard, &[parent_node])?
                .into_iter()
                .next()
                .map(|row| row.id)
        {
            current = parent_id;
            continue;
        }
        let version_edges = outgoing_edges(&guard, comment, EDGE_COMMENT_ATTACH_VERSION)?;
        return Ok(version_edges.first().and_then(|edge| {
            read_rows::<IdRow>(&guard, &[edge.to])
                .ok()
                .and_then(|rows| rows.into_iter().next())
                .map(|row| row.id)
        }));
    }
}
