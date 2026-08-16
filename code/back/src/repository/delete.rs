use agdb::{DbError, QueryBuilder};

use crate::repository::graph::{
    DbHandle, read_node_in_txn, read_rows_sync, resolve_node_id_in_txn, resolve_node_id_sync,
};
use crate::repository::schema::{
    EDGE_ARTICLE_HOLD_VERSION, EDGE_COMMENT_ATTACH_VERSION, EDGE_COMMENT_REPLY_COMMENT,
    EDGE_USER_AUTHOR_ARTICLE, EDGE_USER_AUTHOR_COMMENT, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_COMMENT,
    ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, IdRow, KEY_LATEST_VERSION_ID, KEY_SOFT_DELETED,
    KEY_TYPE, VersionRow,
};
#[derive(Debug, Default)]
pub struct DeleteOutcome {
    pub removed_pdf_hashes: Vec<String>,
}

pub async fn delete_user(db: &DbHandle, user_id: &str) -> Result<DeleteOutcome, DbError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let mut outcome = DeleteOutcome::default();
        let Some(user) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_USER, user_id)? else {
            return Ok(outcome);
        };
        let article_edges = transaction.exec(
            QueryBuilder::search()
                .from(user)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_USER_AUTHOR_ARTICLE)
                .query(),
        )?;
        for edge in &article_edges.elements {
            delete_article_in_txn(transaction, edge.to, &mut outcome)?;
        }
        let comment_edges = transaction.exec(
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
            delete_comment_tree_in_txn(transaction, edge.to)?;
        }
        transaction.exec_mut(QueryBuilder::remove().ids([user]).query())?;
        Ok(outcome)
    })
}

pub async fn delete_article(db: &DbHandle, article_id: &str) -> Result<DeleteOutcome, DbError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let mut outcome = DeleteOutcome::default();
        let Some(article) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_ARTICLE, article_id)?
        else {
            return Ok(outcome);
        };
        delete_article_in_txn(transaction, article, &mut outcome)?;
        Ok(outcome)
    })
}

pub async fn delete_comment(db: &DbHandle, comment_id: &str) -> Result<DeleteOutcome, DbError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let outcome = DeleteOutcome::default();
        let Some(comment) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_COMMENT, comment_id)?
        else {
            return Ok(outcome);
        };
        delete_comment_tree_in_txn(transaction, comment)?;
        Ok(outcome)
    })
}

pub async fn delete_version(db: &DbHandle, version_id: &str) -> Result<DeleteOutcome, DbError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let mut outcome = DeleteOutcome::default();
        let Some(version) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_VERSION, version_id)?
        else {
            return Ok(outcome);
        };
        let article_id = transaction
            .exec(
                QueryBuilder::search()
                    .to(version)
                    .where_()
                    .distance(agdb::CountComparison::Equal(1))
                    .and()
                    .edge()
                    .and()
                    .key(KEY_TYPE)
                    .value(EDGE_ARTICLE_HOLD_VERSION)
                    .query(),
            )?
            .elements
            .first()
            .map(|edge| edge.from);
        delete_version_in_txn(transaction, version, &mut outcome)?;
        if let Some(article) = article_id {
            refresh_latest_version_in_txn(transaction, article)?;
        }
        Ok(outcome)
    })
}

pub fn has_soft_deleted_flag(guard: &agdb::DbAny, id: agdb::DbId) -> Result<bool, DbError> {
    let result = guard.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .ids([agdb::QueryId::from(id)])
            .and()
            .key(KEY_SOFT_DELETED)
            .value(agdb::Comparison::GreaterThan(agdb::DbValue::U64(0)))
            .query(),
    )?;
    Ok(!result.elements.is_empty())
}

pub(crate) fn content_path_soft_deleted_in_txn(
    transaction: &agdb::DbAnyTransactionMut,
    kind: &str,
    business_id: &str,
) -> Result<bool, DbError> {
    let mut current_kind = kind.to_string();
    let mut current_id = business_id.to_string();
    loop {
        let Some(node) = resolve_node_id_in_txn(transaction, &current_kind, &current_id)? else {
            return Ok(false);
        };
        if has_soft_deleted_flag_in_txn(transaction, node)? {
            return Ok(true);
        }
        match current_kind.as_str() {
            ENTITY_TYPE_COMMENT => {
                let parent =
                    next_comment_chain_node_in_txn(transaction, node, EDGE_COMMENT_REPLY_COMMENT)?;
                if let Some((next_kind, next_id)) = parent {
                    current_kind = next_kind;
                    current_id = next_id;
                    continue;
                }
                let Some((next_kind, next_id)) =
                    next_comment_chain_node_in_txn(transaction, node, EDGE_COMMENT_ATTACH_VERSION)?
                else {
                    return Ok(false);
                };
                current_kind = next_kind;
                current_id = next_id;
            }
            ENTITY_TYPE_VERSION => {
                let Some((next_kind, next_id)) =
                    incoming_node_id_in_txn(transaction, node, EDGE_ARTICLE_HOLD_VERSION)?
                else {
                    return Ok(false);
                };
                current_kind = next_kind;
                current_id = next_id;
            }
            _ => return Ok(false),
        }
    }
}

fn has_soft_deleted_flag_in_txn(
    transaction: &agdb::DbAnyTransactionMut,
    id: agdb::DbId,
) -> Result<bool, DbError> {
    let result = transaction.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .ids([agdb::QueryId::from(id)])
            .and()
            .key(KEY_SOFT_DELETED)
            .value(agdb::Comparison::GreaterThan(agdb::DbValue::U64(0)))
            .query(),
    )?;
    Ok(!result.elements.is_empty())
}

fn next_comment_chain_node_in_txn(
    transaction: &agdb::DbAnyTransactionMut,
    node: agdb::DbId,
    edge_type: &str,
) -> Result<Option<(String, String)>, DbError> {
    let edges = transaction.exec(
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
    let Some(edge) = edges.elements.first() else {
        return Ok(None);
    };
    let id = read_rows_sync_from_txn(transaction, &[edge.to])?
        .into_iter()
        .next()
        .map(|row| row.id)
        .unwrap_or_default();
    let next_kind = match edge_type {
        EDGE_COMMENT_REPLY_COMMENT => ENTITY_TYPE_COMMENT,
        EDGE_COMMENT_ATTACH_VERSION => ENTITY_TYPE_VERSION,
        EDGE_ARTICLE_HOLD_VERSION => ENTITY_TYPE_ARTICLE,
        _ => return Ok(None),
    };
    Ok(Some((next_kind.to_string(), id)))
}

fn incoming_node_id_in_txn(
    transaction: &agdb::DbAnyTransactionMut,
    node: agdb::DbId,
    edge_type: &str,
) -> Result<Option<(String, String)>, DbError> {
    let edges = transaction.exec(
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
    let Some(edge) = edges.elements.first() else {
        return Ok(None);
    };
    let id = read_rows_sync_from_txn(transaction, &[edge.from])?
        .into_iter()
        .next()
        .map(|row| row.id)
        .unwrap_or_default();
    let next_kind = match edge_type {
        EDGE_ARTICLE_HOLD_VERSION => ENTITY_TYPE_ARTICLE,
        EDGE_USER_AUTHOR_ARTICLE | EDGE_USER_AUTHOR_COMMENT => ENTITY_TYPE_USER,
        _ => return Ok(None),
    };
    Ok(Some((next_kind.to_string(), id)))
}

fn read_rows_sync_from_txn(
    transaction: &agdb::DbAnyTransactionMut,
    ids: &[agdb::DbId],
) -> Result<Vec<IdRow>, DbError> {
    crate::repository::graph::read_rows_in_txn::<IdRow>(transaction, ids)
}

pub(crate) fn content_path_soft_deleted_sync(
    guard: &agdb::DbAny,
    kind: &str,
    business_id: &str,
) -> Result<bool, DbError> {
    let mut current_kind = kind.to_string();
    let mut current_id = business_id.to_string();
    loop {
        let Some(node) = resolve_node_id_sync(guard, &current_kind, &current_id)? else {
            return Ok(false);
        };
        if has_soft_deleted_flag(guard, node)? {
            return Ok(true);
        }
        match current_kind.as_str() {
            ENTITY_TYPE_COMMENT => {
                let parent = next_comment_chain_node(guard, node, EDGE_COMMENT_REPLY_COMMENT)?;
                if let Some((next_kind, next_id)) = parent {
                    current_kind = next_kind;
                    current_id = next_id;
                    continue;
                }
                let Some((next_kind, next_id)) =
                    next_comment_chain_node(guard, node, EDGE_COMMENT_ATTACH_VERSION)?
                else {
                    return Ok(false);
                };
                current_kind = next_kind;
                current_id = next_id;
            }
            ENTITY_TYPE_VERSION => {
                let Some((next_kind, next_id)) =
                    incoming_node_id(guard, node, EDGE_ARTICLE_HOLD_VERSION)?
                else {
                    return Ok(false);
                };
                current_kind = next_kind;
                current_id = next_id;
            }
            _ => return Ok(false),
        }
    }
}

fn next_comment_chain_node(
    guard: &agdb::DbAny,
    node: agdb::DbId,
    edge_type: &str,
) -> Result<Option<(String, String)>, DbError> {
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
    let Some(edge) = edges.elements.first() else {
        return Ok(None);
    };
    let id = read_rows_sync::<IdRow>(guard, &[edge.to])?
        .into_iter()
        .next()
        .map(|row| row.id)
        .unwrap_or_default();
    let next_kind = match edge_type {
        EDGE_COMMENT_REPLY_COMMENT => ENTITY_TYPE_COMMENT,
        EDGE_COMMENT_ATTACH_VERSION => ENTITY_TYPE_VERSION,
        EDGE_ARTICLE_HOLD_VERSION => ENTITY_TYPE_ARTICLE,
        _ => return Ok(None),
    };
    Ok(Some((next_kind.to_string(), id)))
}

fn incoming_node_id(
    guard: &agdb::DbAny,
    node: agdb::DbId,
    edge_type: &str,
) -> Result<Option<(String, String)>, DbError> {
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
    let Some(edge) = edges.elements.first() else {
        return Ok(None);
    };
    let id = read_rows_sync::<IdRow>(guard, &[edge.from])?
        .into_iter()
        .next()
        .map(|row| row.id)
        .unwrap_or_default();
    let next_kind = match edge_type {
        EDGE_ARTICLE_HOLD_VERSION => ENTITY_TYPE_ARTICLE,
        EDGE_USER_AUTHOR_ARTICLE | EDGE_USER_AUTHOR_COMMENT => ENTITY_TYPE_USER,
        _ => return Ok(None),
    };
    Ok(Some((next_kind.to_string(), id)))
}

pub async fn soft_delete_article(db: &DbHandle, article_id: &str) -> Result<(), DbError> {
    adjust_soft_delete_count(db, ENTITY_TYPE_ARTICLE, article_id, 1).await
}

pub async fn soft_delete_version(db: &DbHandle, version_id: &str) -> Result<(), DbError> {
    adjust_soft_delete_count(db, ENTITY_TYPE_VERSION, version_id, 1).await
}

pub async fn soft_delete_comment(db: &DbHandle, comment_id: &str) -> Result<(), DbError> {
    adjust_soft_delete_count(db, ENTITY_TYPE_COMMENT, comment_id, 1).await
}

#[cfg_attr(not(test), allow(dead_code))]
pub async fn clear_soft_deleted_flag(db: &DbHandle, business_id: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let Some((kind, id)) = resolve_any_node_id_sync(&guard, business_id)? else {
        return Ok(());
    };
    guard.transaction_mut(|transaction| {
        adjust_soft_delete_count_in_txn(transaction, kind.as_str(), id, -1)
    })?;
    Ok(())
}

fn resolve_any_node_id_sync(
    guard: &agdb::DbAny,
    business_id: &str,
) -> Result<Option<(String, agdb::DbId)>, DbError> {
    for kind in [ENTITY_TYPE_ARTICLE, ENTITY_TYPE_VERSION, ENTITY_TYPE_COMMENT] {
        if let Some(id) = resolve_node_id_sync(guard, kind, business_id)? {
            return Ok(Some((kind.to_string(), id)));
        }
    }
    Ok(None)
}

async fn adjust_soft_delete_count(
    db: &DbHandle,
    entity_type: &str,
    business_id: &str,
    delta: i64,
) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let Some(id) = resolve_node_id_sync(&guard, entity_type, business_id)? else {
        return Ok(());
    };
    guard.transaction_mut(|transaction| {
        adjust_soft_delete_count_in_txn(transaction, entity_type, id, delta)
    })?;
    Ok(())
}

fn adjust_soft_delete_count_in_txn(
    transaction: &mut agdb::DbAnyTransactionMut,
    kind: &str,
    id: agdb::DbId,
    delta: i64,
) -> Result<(), DbError> {
    match kind {
        ENTITY_TYPE_ARTICLE => adjust_article_subtree_in_txn(transaction, id, delta),
        ENTITY_TYPE_VERSION => adjust_version_subtree_in_txn(transaction, id, delta),
        ENTITY_TYPE_COMMENT => adjust_comment_tree_in_txn(transaction, id, delta),
        _ => Ok(()),
    }
}

fn adjust_article_subtree_in_txn(
    transaction: &mut agdb::DbAnyTransactionMut,
    article: agdb::DbId,
    delta: i64,
) -> Result<(), DbError> {
    let version_edges = transaction.exec(
        QueryBuilder::search()
            .from(article)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_HOLD_VERSION)
            .query(),
    )?;
    for edge in &version_edges.elements {
        adjust_version_subtree_in_txn(transaction, edge.to, delta)?;
    }
    adjust_node_soft_delete_count_in_txn(transaction, article, delta)?;
    Ok(())
}

fn adjust_version_subtree_in_txn(
    transaction: &mut agdb::DbAnyTransactionMut,
    version: agdb::DbId,
    delta: i64,
) -> Result<(), DbError> {
    let comment_edges = transaction.exec(
        QueryBuilder::search()
            .to(version)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_COMMENT_ATTACH_VERSION)
            .query(),
    )?;
    for edge in &comment_edges.elements {
        adjust_comment_tree_in_txn(transaction, edge.from, delta)?;
    }
    adjust_node_soft_delete_count_in_txn(transaction, version, delta)?;
    Ok(())
}

fn adjust_comment_tree_in_txn(
    transaction: &mut agdb::DbAnyTransactionMut,
    comment: agdb::DbId,
    delta: i64,
) -> Result<(), DbError> {
    let reply_edges = transaction.exec(
        QueryBuilder::search()
            .to(comment)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_COMMENT_REPLY_COMMENT)
            .query(),
    )?;
    for edge in &reply_edges.elements {
        adjust_comment_tree_in_txn(transaction, edge.from, delta)?;
    }
    adjust_node_soft_delete_count_in_txn(transaction, comment, delta)?;
    Ok(())
}

fn adjust_node_soft_delete_count_in_txn(
    transaction: &mut agdb::DbAnyTransactionMut,
    id: agdb::DbId,
    delta: i64,
) -> Result<(), DbError> {
    let current = soft_delete_count_in_txn(transaction, id)?;
    let next = i64::try_from(current).unwrap_or(i64::MAX).saturating_add(delta);
    if next <= 0 {
        transaction.exec_mut(
            QueryBuilder::remove()
                .values([KEY_SOFT_DELETED])
                .ids([id])
                .query(),
        )?;
        return Ok(());
    }
    transaction.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .ids([id])
            .values([[(KEY_SOFT_DELETED, u64::try_from(next).unwrap_or(u64::MAX)).into()]])
            .query(),
    )?;
    Ok(())
}

fn soft_delete_count_in_txn(
    transaction: &agdb::DbAnyTransactionMut,
    id: agdb::DbId,
) -> Result<u64, DbError> {
    let result = match transaction.exec(
        QueryBuilder::select()
            .values([KEY_SOFT_DELETED])
            .ids([agdb::QueryId::from(id)])
            .query(),
    ) {
        Ok(result) => result,
        Err(error) if crate::repository::graph::is_not_found(&error) => return Ok(0),
        Err(error) => return Err(error),
    };
    let Some(element) = result.elements.first() else {
        return Ok(0);
    };
    let Some(key_value) = element.values.iter().find(|value| value.key == KEY_SOFT_DELETED.into())
    else {
        return Ok(0);
    };
    Ok(key_value.value.to_u64().unwrap_or(0))
}

fn delete_article_in_txn(
    transaction: &mut agdb::DbAnyTransactionMut,
    article: agdb::DbId,
    outcome: &mut DeleteOutcome,
) -> Result<(), DbError> {
    let version_edges = transaction.exec(
        QueryBuilder::search()
            .from(article)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_HOLD_VERSION)
            .query(),
    )?;
    for edge in &version_edges.elements {
        delete_version_in_txn(transaction, edge.to, outcome)?;
    }
    transaction.exec_mut(QueryBuilder::remove().ids([article]).query())?;
    Ok(())
}

fn delete_version_in_txn(
    transaction: &mut agdb::DbAnyTransactionMut,
    version: agdb::DbId,
    outcome: &mut DeleteOutcome,
) -> Result<(), DbError> {
    let comment_edges = transaction.exec(
        QueryBuilder::search()
            .to(version)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_COMMENT_ATTACH_VERSION)
            .query(),
    )?;
    for edge in &comment_edges.elements {
        delete_comment_tree_in_txn(transaction, edge.from)?;
    }
    if let Some(row) = read_node_in_txn::<VersionRow>(transaction, version)? {
        outcome.removed_pdf_hashes.push(row.content_hash);
    }
    transaction.exec_mut(QueryBuilder::remove().ids([version]).query())?;
    Ok(())
}

fn delete_comment_tree_in_txn(
    transaction: &mut agdb::DbAnyTransactionMut,
    comment: agdb::DbId,
) -> Result<(), DbError> {
    let reply_edges = transaction.exec(
        QueryBuilder::search()
            .to(comment)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_COMMENT_REPLY_COMMENT)
            .query(),
    )?;
    for edge in &reply_edges.elements {
        delete_comment_tree_in_txn(transaction, edge.from)?;
    }
    transaction.exec_mut(QueryBuilder::remove().ids([comment]).query())?;
    Ok(())
}

fn refresh_latest_version_in_txn(
    transaction: &mut agdb::DbAnyTransactionMut,
    article: agdb::DbId,
) -> Result<(), DbError> {
    let version_edges = transaction.exec(
        QueryBuilder::search()
            .from(article)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_HOLD_VERSION)
            .query(),
    )?;
    let latest_id = version_edges
        .elements
        .iter()
        .filter_map(|edge| read_node_in_txn::<VersionRow>(transaction, edge.to).transpose())
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|row| row.id)
        .max()
        .unwrap_or_default();
    transaction.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .ids([article])
            .values([[(KEY_LATEST_VERSION_ID, latest_id).into()]])
            .query(),
    )?;
    Ok(())
}
