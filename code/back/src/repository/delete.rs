use agdb::{DbError, QueryBuilder};

use crate::repository::graph::{
    DbHandle, read_node_in_txn, resolve_node_id_in_txn, resolve_node_id_sync,
};
use crate::repository::schema::{
    EDGE_ARTICLE_HOLD_VERSION, EDGE_COMMENT_ATTACH_VERSION, EDGE_COMMENT_REPLY_COMMENT,
    EDGE_USER_AUTHOR_ARTICLE, EDGE_USER_AUTHOR_COMMENT, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_COMMENT,
    ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, KEY_LATEST_VERSION_ID, KEY_SOFT_DELETED, KEY_TYPE,
    VersionRow,
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
            .keys(KEY_SOFT_DELETED)
            .query(),
    )?;
    Ok(!result.elements.is_empty())
}

pub async fn soft_delete_article(db: &DbHandle, article_id: &str) -> Result<(), DbError> {
    set_soft_deleted_flag(db, ENTITY_TYPE_ARTICLE, article_id).await
}

pub async fn soft_delete_version(db: &DbHandle, version_id: &str) -> Result<(), DbError> {
    set_soft_deleted_flag(db, ENTITY_TYPE_VERSION, version_id).await
}

pub async fn soft_delete_comment(db: &DbHandle, comment_id: &str) -> Result<(), DbError> {
    set_soft_deleted_flag(db, ENTITY_TYPE_COMMENT, comment_id).await
}

async fn set_soft_deleted_flag(
    db: &DbHandle,
    entity_type: &str,
    business_id: &str,
) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let Some(id) = resolve_node_id_sync(&guard, entity_type, business_id)? else {
        return Ok(());
    };
    guard.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .ids([id])
            .values([[(KEY_SOFT_DELETED, 1).into()]])
            .query(),
    )?;
    Ok(())
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
