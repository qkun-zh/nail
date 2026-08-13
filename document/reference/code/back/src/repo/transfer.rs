
use agdb::{DbError, QueryBuilder};

use crate::repo::authorization::ROLE_RECYCLER;
use crate::repo::db::{DbHandle, resolve_node_id_sync};
use crate::repo::types::{
    EDGE_USER_HOLD_ROLE, EDGE_USER_TO_ARTICLE, EDGE_USER_TO_COMMENT, ENTITY_TYPE_ROLE,
    ENTITY_TYPE_USER, IdRow, KEY_TYPE,
};

pub struct AccountTransferOutcome {
    pub transferred_article_edges: u64,
    pub transferred_comment_edges: u64,
}

pub async fn transfer_account_assets(
    db: &DbHandle,
    author_id: &str,
) -> Result<AccountTransferOutcome, DbError> {
    let target = pick_recycler_target(db, &[author_id])
        .await?
        .ok_or_else(|| no_recycler_error())?;
    let mut db = db.write().await;
    db.transaction_mut(|txn| -> Result<AccountTransferOutcome, DbError> {
        let recycler = crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_USER, &target)?
            .ok_or_else(|| no_recycler_error())?;
        let article_edges = repoint_from_user(txn, recycler, author_id, EDGE_USER_TO_ARTICLE)?;
        let comment_edges = repoint_from_user(txn, recycler, author_id, EDGE_USER_TO_COMMENT)?;
        if let Some(user_id) =
            crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_USER, author_id)?
        {
            txn.exec_mut(QueryBuilder::remove().ids([user_id]).query())?;
        }
        Ok(AccountTransferOutcome {
            transferred_article_edges: article_edges,
            transferred_comment_edges: comment_edges,
        })
    })
}

fn repoint_from_user(
    txn: &mut agdb::DbAnyTransactionMut,
    recycler: agdb::DbId,
    author_id: &str,
    edge_type: &str,
) -> Result<u64, DbError> {
    let Some(author) = crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_USER, author_id)?
    else {
        return Ok(0);
    };
    let edges = txn.exec(
        QueryBuilder::search()
            .from(author)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(edge_type)
            .query(),
    )?;
    let targets: Vec<agdb::DbId> = edges.elements.iter().map(|el| el.to).collect();
    if edges.elements.is_empty() {
        return Ok(0);
    }
    let edge_ids: Vec<agdb::DbId> = edges.elements.iter().map(|el| el.id).collect();
    txn.exec_mut(QueryBuilder::remove().ids(edge_ids).query())?;
    for target in &targets {
        txn.exec_mut(
            QueryBuilder::insert()
                .edges()
                .from(recycler)
                .to([*target])
                .values([[(KEY_TYPE, edge_type).into()]])
                .query(),
        )?;
    }
    Ok(targets.len() as u64)
}

#[derive(Debug)]
pub enum TargetTransferError {
    TargetNotFound,
    NoRecycler,
    Db(DbError),
}

impl From<DbError> for TargetTransferError {
    fn from(error: DbError) -> Self {
        TargetTransferError::Db(error)
    }
}

pub(crate) async fn transfer_target_ownership(
    db: &DbHandle,
    target_kind: &str,
    edge_type: &str,
    target_id: &str,
) -> Result<(), TargetTransferError> {
    let target = pick_recycler_target(db, &[]).await?.ok_or(TargetTransferError::NoRecycler)?;
    let mut db = db.write().await;
    db.transaction_mut(|txn| -> Result<(), TargetTransferError> {
        let recycler = crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_USER, &target)?
            .ok_or(TargetTransferError::NoRecycler)?;
        let target_node = crate::repo::db::resolve_node_id_in_txn(txn, target_kind, target_id)?
            .ok_or(TargetTransferError::TargetNotFound)?;
        let edges = txn.exec(
            QueryBuilder::search()
                .to(target_node)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(edge_type)
                .query(),
        )?;
        if edges.elements.is_empty() {
            return Ok(());
        }
        let edge_ids: Vec<agdb::DbId> = edges.elements.iter().map(|el| el.id).collect();
        txn.exec_mut(QueryBuilder::remove().ids(edge_ids).query())?;
        txn.exec_mut(
            QueryBuilder::insert()
                .edges()
                .from(recycler)
                .to([target_node])
                .values([[(KEY_TYPE, edge_type).into()]])
                .query(),
        )?;
        Ok(())
    })
}

pub async fn transfer_article_ownership(
    db: &DbHandle,
    article_id: &str,
) -> Result<(), TargetTransferError> {
    transfer_target_ownership(
        db,
        crate::repo::types::ENTITY_TYPE_ARTICLE,
        EDGE_USER_TO_ARTICLE,
        article_id,
    )
    .await
}

pub async fn transfer_comment_ownership(
    db: &DbHandle,
    comment_id: &str,
) -> Result<(), TargetTransferError> {
    transfer_target_ownership(
        db,
        crate::repo::types::ENTITY_TYPE_COMMENT,
        EDGE_USER_TO_COMMENT,
        comment_id,
    )
    .await
}

async fn users_holding_role(db: &DbHandle, role_name: &str) -> Result<Vec<String>, DbError> {
    let db = db.read().await;
    let Some(role_id) = resolve_node_id_sync(&db, ENTITY_TYPE_ROLE, role_name)? else {
        return Ok(Vec::new());
    };
    let edges = db.exec(
        QueryBuilder::search()
            .to(role_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_HOLD_ROLE)
            .query(),
    )?;
    let mut users = Vec::new();
    for edge in &edges.elements {
        if let Some(row) = crate::repo::db::read_node_sync::<IdRow>(&db, edge.from)? {
            users.push(row.id);
        }
    }
    Ok(users)
}

async fn pick_recycler_target(
    db: &DbHandle,
    exclude: &[&str],
) -> Result<Option<String>, DbError> {
    let recyclers = users_holding_role(db, ROLE_RECYCLER).await?;
    let db = db.read().await;
    let mut best: Option<(String, u64)> = None;
    for user_id in recyclers {
        if exclude.contains(&user_id.as_str()) {
            continue;
        }
        let Some(node) = resolve_node_id_sync(&db, ENTITY_TYPE_USER, &user_id)? else {
            continue;
        };
        let articles = count_edges_sync(&db, node, EDGE_USER_TO_ARTICLE)?;
        let comments = count_edges_sync(&db, node, EDGE_USER_TO_COMMENT)?;
        let total = articles + comments;
        let better = match &best {
            None => true,
            Some((best_id, best_total)) => {
                total < *best_total || (total == *best_total && user_id > *best_id)
            }
        };
        if better {
            best = Some((user_id.clone(), total));
        }
    }
    Ok(best.map(|(id, _)| id))
}

fn count_edges_sync(db: &agdb::DbAny, from: agdb::DbId, edge_type: &str) -> Result<u64, DbError> {
    Ok(db
        .exec(
            QueryBuilder::search()
                .from(from)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(edge_type)
                .query(),
        )?
        .elements
        .len() as u64)
}

fn no_recycler_error() -> DbError {
    DbError::query(
        agdb::DbErrorType::NotFound,
        "no recycler available (required role seed missing?)",
    )
}
