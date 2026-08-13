use agdb::{DbError, QueryBuilder};

use crate::repository::graph::{DbHandle, read_node_in_txn, resolve_node_id_in_txn};
use crate::repository::schema::{
    EDGE_ARTICLE_TO_VERSION, EDGE_COMMENT_TO_COMMENT, EDGE_COMMENT_TO_VERSION,
    EDGE_USER_TO_ARTICLE, EDGE_USER_TO_COMMENT, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_COMMENT,
    ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, KEY_LATEST_VERSION_ID, KEY_TYPE, VersionRow,
};

#[derive(Debug, Default)]
pub struct HardDeleteOutcome {
    pub removed_pdf_hashes: Vec<String>,
}

pub async fn hard_delete_user(db: &DbHandle, user_id: &str) -> Result<HardDeleteOutcome, DbError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let mut outcome = HardDeleteOutcome::default();
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
                .value(EDGE_USER_TO_ARTICLE)
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
                .value(EDGE_USER_TO_COMMENT)
                .query(),
        )?;
        for edge in &comment_edges.elements {
            delete_comment_tree_in_txn(transaction, edge.to)?;
        }
        transaction.exec_mut(QueryBuilder::remove().ids([user]).query())?;
        Ok(outcome)
    })
}

pub async fn hard_delete_article(
    db: &DbHandle,
    article_id: &str,
) -> Result<HardDeleteOutcome, DbError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let mut outcome = HardDeleteOutcome::default();
        let Some(article) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_ARTICLE, article_id)?
        else {
            return Ok(outcome);
        };
        delete_article_in_txn(transaction, article, &mut outcome)?;
        Ok(outcome)
    })
}

pub async fn hard_delete_version(
    db: &DbHandle,
    version_id: &str,
) -> Result<HardDeleteOutcome, DbError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        let mut outcome = HardDeleteOutcome::default();
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
                    .value(EDGE_ARTICLE_TO_VERSION)
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

fn delete_article_in_txn(
    transaction: &mut agdb::DbAnyTransactionMut,
    article: agdb::DbId,
    outcome: &mut HardDeleteOutcome,
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
            .value(EDGE_ARTICLE_TO_VERSION)
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
    outcome: &mut HardDeleteOutcome,
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
            .value(EDGE_COMMENT_TO_VERSION)
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
    for edge in &reply_edges.elements {
        delete_comment_tree_in_txn(transaction, edge.to)?;
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
            .value(EDGE_ARTICLE_TO_VERSION)
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
