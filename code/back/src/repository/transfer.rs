use agdb::{DbError, QueryBuilder};

use crate::repository::graph::{
    DbHandle, edge_count, incoming_edges, insert_edge, outgoing_edges, read_node, read_rows,
    resolve_node_id,
};
use crate::repository::role::{ROLE_RECYCLER, users_holding_role};
use crate::repository::schema::{
    EDGE_USER_AUTHOR_ARTICLE, EDGE_USER_AUTHOR_COMMENT, ENTITY_TYPE_COMMENT, ENTITY_TYPE_USER,
    IdRow,
};

pub struct AccountTransferOutcome {
    pub transferred_article_ids: Vec<String>,
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

impl std::fmt::Display for TransferError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRecycler => formatter.write_str("no recycler available"),
            Self::Db(error) => write!(formatter, "database query failed: {error}"),
        }
    }
}

impl std::error::Error for TransferError {}

#[derive(Debug)]
pub enum TransferTargetError {
    TargetMissing,
    TargetOwnerMissing,
    NoRecycler,
    Db(DbError),
}

impl From<DbError> for TransferTargetError {
    fn from(error: DbError) -> Self {
        Self::Db(error)
    }
}

impl std::fmt::Display for TransferTargetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetMissing => formatter.write_str("transfer target not found"),
            Self::TargetOwnerMissing => formatter.write_str("transfer target has no owner edge"),
            Self::NoRecycler => formatter.write_str("no recycler available"),
            Self::Db(error) => write!(formatter, "database query failed: {error}"),
        }
    }
}

impl std::error::Error for TransferTargetError {}

pub async fn transfer_account_assets(
    db: &DbHandle,
    author_id: &str,
) -> Result<AccountTransferOutcome, TransferError> {
    let target = pick_recycler_target(db, &[author_id.to_string()])
        .await?
        .ok_or(TransferError::NoRecycler)?;
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let recycler = resolve_node_id(transaction, ENTITY_TYPE_USER, &target)?
            .ok_or(TransferError::NoRecycler)?;
        let article_ids =
            repoint_from_user(transaction, recycler, author_id, EDGE_USER_AUTHOR_ARTICLE)?;
        repoint_from_user(transaction, recycler, author_id, EDGE_USER_AUTHOR_COMMENT)?;
        if let Some(user_node) = resolve_node_id(transaction, ENTITY_TYPE_USER, author_id)? {
            transaction.exec_mut(QueryBuilder::remove().ids([user_node]).query())?;
        }
        Ok(AccountTransferOutcome {
            transferred_article_ids: article_ids,
        })
    })
}

pub async fn transfer_article(db: &DbHandle, article_id: &str) -> Result<(), TransferTargetError> {
    transfer_target_ownership(
        db,
        crate::repository::schema::ENTITY_TYPE_ARTICLE,
        EDGE_USER_AUTHOR_ARTICLE,
        article_id,
    )
    .await
}

pub async fn transfer_comment(db: &DbHandle, comment_id: &str) -> Result<(), TransferTargetError> {
    transfer_target_ownership(
        db,
        ENTITY_TYPE_COMMENT,
        EDGE_USER_AUTHOR_COMMENT,
        comment_id,
    )
    .await
}

async fn transfer_target_ownership(
    db: &DbHandle,
    target_kind: &str,
    edge_type: &str,
    target_id: &str,
) -> Result<(), TransferTargetError> {
    let mut exclude: Vec<String> = Vec::new();
    {
        let guard = db.read().await;
        let Some(target_node) = resolve_node_id(&guard, target_kind, target_id)? else {
            return Err(TransferTargetError::TargetMissing);
        };
        let owner_edges = incoming_edges(&guard, target_node, edge_type)?;
        if let Some(edge) = owner_edges.first()
            && let Some(row) = read_node::<IdRow>(&guard, edge.from)?
        {
            exclude.push(row.id);
        }
    }
    let target = pick_recycler_target(db, &exclude)
        .await?
        .ok_or(TransferTargetError::NoRecycler)?;
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let recycler = resolve_node_id(transaction, ENTITY_TYPE_USER, &target)?
            .ok_or(TransferTargetError::NoRecycler)?;
        let target_node = resolve_node_id(transaction, target_kind, target_id)?
            .ok_or(TransferTargetError::TargetMissing)?;
        let edges = incoming_edges(transaction, target_node, edge_type)?;
        let edge = edges
            .first()
            .ok_or(TransferTargetError::TargetOwnerMissing)?;
        let edge_ids = [edge.id];
        transaction.exec_mut(QueryBuilder::remove().ids(edge_ids).query())?;
        insert_edge(transaction, edge_type, recycler.into(), target_node.into())?;
        Ok(())
    })
}

fn repoint_from_user(
    transaction: &mut agdb::DbAnyTransactionMut,
    recycler: agdb::DbId,
    author_id: &str,
    edge_type: &str,
) -> Result<Vec<String>, DbError> {
    let Some(author) = resolve_node_id(transaction, ENTITY_TYPE_USER, author_id)? else {
        return Ok(Vec::new());
    };
    let edges = outgoing_edges(transaction, author, edge_type)?;
    let targets: Vec<agdb::DbId> = edges.iter().map(|element| element.to).collect();
    if edges.is_empty() {
        return Ok(Vec::new());
    }
    let target_ids = read_rows::<IdRow>(transaction, &targets)?
        .into_iter()
        .map(|row| row.id)
        .collect();
    let edge_ids: Vec<agdb::DbId> = edges.iter().map(|element| element.id).collect();
    transaction.exec_mut(QueryBuilder::remove().ids(edge_ids).query())?;
    for target in &targets {
        insert_edge(transaction, edge_type, recycler.into(), (*target).into())?;
    }
    Ok(target_ids)
}

async fn pick_recycler_target(
    db: &DbHandle,
    exclude: &[String],
) -> Result<Option<String>, DbError> {
    let recyclers = users_holding_role(db, ROLE_RECYCLER).await?;
    let guard = db.read().await;
    let mut best: Option<(String, u64)> = None;
    for user_id in recyclers {
        if exclude.contains(&user_id) {
            continue;
        }
        let Some(node) = resolve_node_id(&guard, ENTITY_TYPE_USER, &user_id)? else {
            continue;
        };
        let articles = edge_count(&guard, node, EDGE_USER_AUTHOR_ARTICLE)?;
        let comments = edge_count(&guard, node, EDGE_USER_AUTHOR_COMMENT)?;
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
