use database::{Database, EdgeKind, Error, NodeId, NodeKind};

use crate::repository::access::GraphRead;
use crate::repository::role::{ROLE_RECYCLER, users_holding_role};
use crate::repository::schema::IdRow;

pub struct AccountTransferOutcome {
    pub transferred_article_ids: Vec<String>,
}

#[derive(Debug)]
pub enum TransferError {
    NoRecycler,
    Db(Error),
}

impl From<Error> for TransferError {
    fn from(error: Error) -> Self {
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
    Db(Error),
}

impl From<Error> for TransferTargetError {
    fn from(error: Error) -> Self {
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

pub fn transfer_account_assets(
    db: &Database,
    author_id: &str,
) -> Result<AccountTransferOutcome, TransferError> {
    let target =
        pick_recycler_target(db, &[author_id.to_string()])?.ok_or(TransferError::NoRecycler)?;
    db.write(|scope| {
        let Some(recycler) = scope.resolve(NodeKind::User, &target)? else {
            return Ok(Err(TransferError::NoRecycler));
        };
        let article_ids = repoint_from_user(
            scope,
            recycler,
            author_id,
            EdgeKind::UserAuthorArticle,
            NodeKind::Article,
        )?;
        repoint_from_user(
            scope,
            recycler,
            author_id,
            EdgeKind::UserAuthorComment,
            NodeKind::Comment,
        )?;
        if let Some(user_node) = scope.resolve(NodeKind::User, author_id)? {
            scope.remove(&[user_node])?;
        }
        Ok(Ok(AccountTransferOutcome {
            transferred_article_ids: article_ids,
        }))
    })
    .map_err(TransferError::from)
    .and_then(std::convert::identity)
}

pub fn transfer_article(db: &Database, article_id: &str) -> Result<(), TransferTargetError> {
    transfer_target_ownership(
        db,
        NodeKind::Article,
        EdgeKind::UserAuthorArticle,
        article_id,
    )
}

pub fn transfer_comment(db: &Database, comment_id: &str) -> Result<(), TransferTargetError> {
    transfer_target_ownership(
        db,
        NodeKind::Comment,
        EdgeKind::UserAuthorComment,
        comment_id,
    )
}

fn transfer_target_ownership(
    db: &Database,
    target_kind: NodeKind,
    edge_kind: EdgeKind,
    target_id: &str,
) -> Result<(), TransferTargetError> {
    let mut exclude: Vec<String> = Vec::new();
    let found = db.read(|scope| {
        let Some(target_node) = scope.resolve(target_kind, target_id)? else {
            return Ok(false);
        };
        if let Some(owner) = scope.incoming(target_node, edge_kind)?.first()
            && let Some(row) = scope.read_node::<IdRow>(*owner)?
        {
            exclude.push(row.id);
        }
        Ok(true)
    })?;
    if !found {
        return Err(TransferTargetError::TargetMissing);
    }
    let target = pick_recycler_target(db, &exclude)?.ok_or(TransferTargetError::NoRecycler)?;
    db.write(|scope| {
        let Some(recycler) = scope.resolve(NodeKind::User, &target)? else {
            return Ok(Err(TransferTargetError::NoRecycler));
        };
        let Some(target_node) = scope.resolve(target_kind, target_id)? else {
            return Ok(Err(TransferTargetError::TargetMissing));
        };
        let Some(old_owner) = scope.incoming(target_node, edge_kind)?.first().copied() else {
            return Ok(Err(TransferTargetError::TargetOwnerMissing));
        };
        scope.remove_edge(old_owner, edge_kind, target_node)?;
        scope.insert_edge(
            NodeKind::User,
            recycler,
            edge_kind,
            target_kind,
            target_node,
        )?;
        Ok(Ok(()))
    })
    .map_err(TransferTargetError::from)
    .and_then(std::convert::identity)
}

fn repoint_from_user(
    scope: &mut database::WriteScope<'_, '_>,
    recycler: NodeId,
    author_id: &str,
    edge_kind: EdgeKind,
    target_kind: NodeKind,
) -> Result<Vec<String>, Error> {
    let Some(author) = scope.resolve(NodeKind::User, author_id)? else {
        return Ok(Vec::new());
    };
    let targets = scope.outgoing(author, edge_kind)?;
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    let target_ids = scope
        .scope_read_nodes::<IdRow>(&targets)?
        .into_iter()
        .map(|row| row.id)
        .collect();
    for target in &targets {
        scope.remove_edge(author, edge_kind, *target)?;
    }
    for target in &targets {
        scope.insert_edge(NodeKind::User, recycler, edge_kind, target_kind, *target)?;
    }
    Ok(target_ids)
}

fn pick_recycler_target(db: &Database, exclude: &[String]) -> Result<Option<String>, Error> {
    let recyclers = users_holding_role(db, ROLE_RECYCLER)?;
    db.read(|scope| {
        let mut best: Option<(String, u64)> = None;
        for user_id in recyclers {
            if exclude.contains(&user_id) {
                continue;
            }
            let Some(node) = scope.resolve(NodeKind::User, &user_id)? else {
                continue;
            };
            let articles = scope.count_outgoing(node, EdgeKind::UserAuthorArticle)?;
            let comments = scope.count_outgoing(node, EdgeKind::UserAuthorComment)?;
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
    })
}
