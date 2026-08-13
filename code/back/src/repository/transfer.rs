use agdb::{DbError, QueryBuilder};

use crate::repository::graph::{DbHandle, resolve_node_id_in_txn, resolve_node_id_sync};
use crate::repository::role::{ROLE_RECYCLER, users_holding_role};
use crate::repository::schema::{
    EDGE_USER_TO_ARTICLE, EDGE_USER_TO_COMMENT, ENTITY_TYPE_USER, KEY_TYPE,
};

pub struct AccountTransferOutcome {
    pub transferred_article_edges: u64,
    pub transferred_comment_edges: u64,
}

#[derive(Debug)]
pub enum TransferError {
    NoRecycler,
    Db(DbError),
}

impl From<DbError> for TransferError {
    fn from(error: DbError) -> Self {
        Self::Db(error)
    }
}

pub async fn transfer_account_assets(
    db: &DbHandle,
    author_id: &str,
) -> Result<AccountTransferOutcome, TransferError> {
    let target = pick_recycler_target(db, &[author_id])
        .await?
        .ok_or(TransferError::NoRecycler)?;
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let recycler = resolve_node_id_in_txn(transaction, ENTITY_TYPE_USER, &target)?
            .ok_or(TransferError::NoRecycler)?;
        let article_edges =
            repoint_from_user(transaction, recycler, author_id, EDGE_USER_TO_ARTICLE)?;
        let comment_edges =
            repoint_from_user(transaction, recycler, author_id, EDGE_USER_TO_COMMENT)?;
        if let Some(user_node) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_USER, author_id)? {
            transaction.exec_mut(QueryBuilder::remove().ids([user_node]).query())?;
        }
        Ok(AccountTransferOutcome {
            transferred_article_edges: article_edges,
            transferred_comment_edges: comment_edges,
        })
    })
}

fn repoint_from_user(
    transaction: &mut agdb::DbAnyTransactionMut,
    recycler: agdb::DbId,
    author_id: &str,
    edge_type: &str,
) -> Result<u64, DbError> {
    let Some(author) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_USER, author_id)? else {
        return Ok(0);
    };
    let edges = transaction.exec(
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
    let targets: Vec<agdb::DbId> = edges.elements.iter().map(|element| element.to).collect();
    if edges.elements.is_empty() {
        return Ok(0);
    }
    let edge_ids: Vec<agdb::DbId> = edges.elements.iter().map(|element| element.id).collect();
    transaction.exec_mut(QueryBuilder::remove().ids(edge_ids).query())?;
    for target in &targets {
        transaction.exec_mut(
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

async fn pick_recycler_target(
    db: &DbHandle,
    exclude: &[&str],
) -> Result<Option<String>, DbError> {
    let recyclers = users_holding_role(db, ROLE_RECYCLER).await?;
    let guard = db.read().await;
    let mut best: Option<(String, u64)> = None;
    for user_id in recyclers {
        if exclude.contains(&user_id.as_str()) {
            continue;
        }
        let Some(node) = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, &user_id)? else {
            continue;
        };
        let articles = count_edges_sync(&guard, node, EDGE_USER_TO_ARTICLE)?;
        let comments = count_edges_sync(&guard, node, EDGE_USER_TO_COMMENT)?;
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
    Ok(best.map(|(user_id, _)| user_id))
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
