
use agdb::{DbError, QueryBuilder};
use serde_json::Value;

use crate::repo::article::relate;
use crate::repo::db::{DbHandle, read_node_sync, resolve_node_id_sync};
use crate::repo::types::{
    CommentRow, EDGE_COMMENT_TO_COMMENT, EDGE_COMMENT_TO_VERSION, EDGE_USER_TO_COMMENT,
    ENTITY_TYPE_COMMENT, ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, IdRow, KEY_TYPE, alias_of,
};

#[derive(Debug)]
pub enum CreateCommentError {
    TargetNotFound,
    CommentIdExists,
    CommentTreeTooDeep,
    Db(DbError),
}

impl From<DbError> for CreateCommentError {
    fn from(error: DbError) -> Self {
        CreateCommentError::Db(error)
    }
}

pub async fn find_comment_author_id(
    db: &DbHandle,
    comment_id: &str,
) -> Result<Option<String>, DbError> {
    let db = db.read().await;
    let Some(comment) = resolve_node_id_sync(&db, ENTITY_TYPE_COMMENT, comment_id)? else {
        return Ok(None);
    };
    let edges = db.exec(
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
    Ok(edges
        .elements
        .first()
        .map(|el| el.from)
        .map(|user| read_node_sync::<IdRow>(&db, user).map(|r| r.map(|row| row.id)))
        .transpose()?
        .flatten())
}

pub use crate::repo::transfer::transfer_comment_ownership;

pub async fn create_top_level_comment(
    db: &DbHandle,
    comment_id: &str,
    user_id: &str,
    version_id: &str,
    content: &str,
) -> Result<(), CreateCommentError> {
    let mut db = db.write().await;
    db.transaction_mut(|txn| -> Result<(), CreateCommentError> {
        let user = crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_USER, user_id)?
            .ok_or(CreateCommentError::TargetNotFound)?;
        let version =
            crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_VERSION, version_id)?
                .ok_or(CreateCommentError::TargetNotFound)?;
        if crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_COMMENT, comment_id)?.is_some()
        {
            return Err(CreateCommentError::CommentIdExists);
        }
        let comment_alias = alias_of(ENTITY_TYPE_COMMENT, comment_id);
        txn.exec_mut(
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
        relate(
            txn,
            EDGE_USER_TO_COMMENT,
            user.into(),
            comment_alias.clone().into(),
        )?;
        relate(
            txn,
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
    let mut db = db.write().await;
    db.transaction_mut(|txn| -> Result<(), CreateCommentError> {
        let user = crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_USER, user_id)?
            .ok_or(CreateCommentError::TargetNotFound)?;
        let parent =
            crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_COMMENT, parent_comment_id)?
                .ok_or(CreateCommentError::TargetNotFound)?;
        if parent_chain_depth_in_txn(txn, parent_comment_id, max_tree_depth)? >= max_tree_depth {
            return Err(CreateCommentError::CommentTreeTooDeep);
        }
        if crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_COMMENT, comment_id)?.is_some()
        {
            return Err(CreateCommentError::CommentIdExists);
        }
        let comment_alias = alias_of(ENTITY_TYPE_COMMENT, comment_id);
        txn.exec_mut(
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
        relate(
            txn,
            EDGE_USER_TO_COMMENT,
            user.into(),
            comment_alias.clone().into(),
        )?;
        relate(
            txn,
            EDGE_COMMENT_TO_COMMENT,
            comment_alias.into(),
            parent.into(),
        )?;
        Ok(())
    })
}

fn parent_chain_depth_in_txn(
    txn: &agdb::DbAnyTransactionMut,
    comment_id: &str,
    max_tree_depth: usize,
) -> Result<usize, DbError> {
    let mut depth = 0usize;
    let mut current = comment_id.to_string();
    loop {
        let Some(current_id) =
            crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_COMMENT, &current)?
        else {
            return Ok(depth);
        };
        let edges = txn.exec(
            QueryBuilder::search()
                .from(current_id)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_COMMENT_TO_COMMENT)
                .query(),
        )?;
        let Some(parent_id) = edges.elements.first().map(|el| el.to) else {
            return Ok(depth);
        };
        let Some(parent_business_id) =
            crate::repo::db::read_node_in_txn::<IdRow>(txn, parent_id)?.map(|r| r.id)
        else {
            return Ok(depth);
        };
        current = parent_business_id;
        depth += 1;
        if depth > max_tree_depth {
            return Ok(depth);
        }
    }
}

#[allow(dead_code)]
pub async fn read_comments_by_version(
    db: &DbHandle,
    version_id: &str,
    max_tree_depth: usize,
) -> Result<Vec<Value>, DbError> {
    let db = db.read().await;
    let mut out: Vec<Value> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let Some(version) = resolve_node_id_sync(&db, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(out);
    };
    let top_edges = db.exec(
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
    let mut layer_ids: Vec<String> = top_edges
        .elements
        .iter()
        .filter_map(|el| {
            read_node_sync::<IdRow>(&db, el.from)
                .ok()
                .flatten()
                .map(|row| row.id)
        })
        .collect();
    layer_ids.sort_by(|a, b| b.cmp(a));
    let layer_rows = read_comment_rows(&db, &layer_ids).await?;
    for row in &layer_rows {
        if let Some(id) = row.get("comment_id").and_then(|v| v.as_str()) {
            seen.insert(id.to_string());
        }
    }
    out.extend(layer_rows);

    let mut depth = 0usize;
    let mut parents: Vec<String> = layer_ids;
    while !parents.is_empty() {
        if depth > max_tree_depth {
            tracing::error!(
                target: "comment",
                version_id = %version_id,
                max_depth = max_tree_depth,
                "comment tree exceeds depth cap (cycle or runaway nesting); truncating"
            );
            break;
        }
        depth += 1;
        let mut kids: Vec<String> = Vec::new();
        for parent_id in &parents {
            let Some(parent) = resolve_node_id_sync(&db, ENTITY_TYPE_COMMENT, parent_id)? else {
                continue;
            };
            let reply_edges = db.exec(
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
                if let Some(kid_id) = read_node_sync::<IdRow>(&db, edge.from)?.map(|r| r.id) {
                    if seen.insert(kid_id.clone()) {
                        kids.push(kid_id);
                    }
                }
            }
        }
        kids.sort();
        let rows = read_comment_rows(&db, &kids).await?;
        out.extend(rows);
        parents = kids;
    }
    Ok(out)
}

pub async fn read_comments_page_by_version(
    db: &DbHandle,
    version_id: &str,
    max_tree_depth: usize,
    limit: u64,
    offset: u64,
) -> Result<(Vec<Value>, u64), DbError> {
    let db = db.read().await;
    let mut out: Vec<Value> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let Some(version) = resolve_node_id_sync(&db, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok((out, 0));
    };
    let top_edges = db.exec(
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
        .filter_map(|el| {
            read_node_sync::<IdRow>(&db, el.from)
                .ok()
                .flatten()
                .map(|row| row.id)
        })
        .collect();
    top_ids.sort_by(|a, b| b.cmp(a));
    let total = top_ids.len() as u64;
    let page_ids: Vec<String> = top_ids
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();
    if page_ids.is_empty() {
        return Ok((out, total));
    }

    let layer_rows = read_comment_rows(&db, &page_ids).await?;
    for row in &layer_rows {
        if let Some(id) = row.get("comment_id").and_then(|v| v.as_str()) {
            seen.insert(id.to_string());
        }
    }
    out.extend(layer_rows);

    let mut depth = 0usize;
    let mut parents: Vec<String> = page_ids;
    while !parents.is_empty() {
        if depth > max_tree_depth {
            tracing::error!(
                target: "comment",
                version_id = %version_id,
                max_depth = max_tree_depth,
                "comment tree exceeds depth cap (cycle or runaway nesting); truncating"
            );
            break;
        }
        depth += 1;
        let mut kids: Vec<String> = Vec::new();
        for parent_id in &parents {
            let Some(parent) = resolve_node_id_sync(&db, ENTITY_TYPE_COMMENT, parent_id)? else {
                continue;
            };
            let reply_edges = db.exec(
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
                if let Some(kid_id) = read_node_sync::<IdRow>(&db, edge.from)?.map(|r| r.id) {
                    if seen.insert(kid_id.clone()) {
                        kids.push(kid_id);
                    }
                }
            }
        }
        kids.sort();
        let rows = read_comment_rows(&db, &kids).await?;
        out.extend(rows);
        parents = kids;
    }
    Ok((out, total))
}

async fn read_comment_rows(
    db: &agdb::DbAny,
    comment_ids: &[String],
) -> Result<Vec<Value>, DbError> {
    let mut rows = Vec::with_capacity(comment_ids.len());
    for comment_id in comment_ids {
        let Some(comment) = resolve_node_id_sync(db, ENTITY_TYPE_COMMENT, comment_id)? else {
            continue;
        };
        let content = read_node_sync::<CommentRow>(db, comment)?
            .map(|r| r.content)
            .unwrap_or_default();
        let author_edges = db.exec(
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
        let author = author_edges
            .elements
            .first()
            .map(|el| read_node_sync::<IdRow>(db, el.from).map(|r| r.map(|row| row.id)))
            .transpose()?
            .flatten()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null);
        let parent_edges = db.exec(
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
        let parent = parent_edges
            .elements
            .first()
            .map(|el| read_node_sync::<IdRow>(db, el.to).map(|r| r.map(|row| row.id)))
            .transpose()?
            .flatten()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null);
        rows.push(serde_json::json!({
            "comment_id": comment_id,
            "content": content,
            "author": author,
            "parent": parent,
        }));
    }
    Ok(rows)
}

pub async fn update_comment_content(
    db: &DbHandle,
    comment_id: &str,
    content: &str,
) -> Result<Option<String>, DbError> {
    let mut db = db.write().await;
    let Some(id) = resolve_node_id_sync(&db, ENTITY_TYPE_COMMENT, comment_id)? else {
        return Ok(None);
    };
    db.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .ids([id])
            .values([[(crate::repo::types::KEY_COMMENT_CONTENT, content).into()]])
            .query(),
    )?;
    Ok(Some(content.to_string()))
}
