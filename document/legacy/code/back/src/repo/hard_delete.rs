
use agdb::{DbError, QueryBuilder};

use crate::repo::db::DbHandle;
use crate::repo::types::{
    EDGE_ARTICLE_TO_VERSION, EDGE_COMMENT_TO_COMMENT, EDGE_COMMENT_TO_VERSION,
    EDGE_USER_TO_ARTICLE, EDGE_USER_TO_COMMENT, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_COMMENT,
    ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, VersionRow, KEY_TYPE,
};

#[derive(Debug, Default)]
pub struct HardDeleteOutcome {
    pub removed_pdf_hashes: Vec<String>,
}

pub async fn hard_delete_user(
    db: &DbHandle,
    user_id: &str,
) -> Result<HardDeleteOutcome, DbError> {
    let mut db = db.write().await;
    db.transaction_mut(|txn| -> Result<HardDeleteOutcome, DbError> {
        let mut outcome = HardDeleteOutcome::default();
        let Some(user) = crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_USER, user_id)?
        else {
            return Ok(outcome);
        };
        let article_edges = txn.exec(
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
            delete_article_in_txn(txn, edge.to, &mut outcome)?;
        }
        let comment_edges = txn.exec(
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
            delete_comment_tree_in_txn(txn, edge.to)?;
        }
        txn.exec_mut(QueryBuilder::remove().ids([user]).query())?;
        Ok(outcome)
    })
}

pub async fn hard_delete_article(
    db: &DbHandle,
    article_id: &str,
) -> Result<HardDeleteOutcome, DbError> {
    let mut db = db.write().await;
    db.transaction_mut(|txn| -> Result<HardDeleteOutcome, DbError> {
        let mut outcome = HardDeleteOutcome::default();
        let Some(article) =
            crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_ARTICLE, article_id)?
        else {
            return Ok(outcome);
        };
        delete_article_in_txn(txn, article, &mut outcome)?;
        Ok(outcome)
    })
}

pub async fn hard_delete_version(
    db: &DbHandle,
    version_id: &str,
) -> Result<HardDeleteOutcome, DbError> {
    let mut db = db.write().await;
    db.transaction_mut(|txn| -> Result<HardDeleteOutcome, DbError> {
        let mut outcome = HardDeleteOutcome::default();
        let Some(version) =
            crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_VERSION, version_id)?
        else {
            return Ok(outcome);
        };
        let article_id = txn
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
            .map(|el| el.from);
        delete_version_in_txn(txn, version, &mut outcome)?;
        if let Some(article) = article_id {
            refresh_latest_version_in_txn(txn, article)?;
        }
        Ok(outcome)
    })
}

pub async fn hard_delete_comment(
    db: &DbHandle,
    comment_id: &str,
) -> Result<(), DbError> {
    let mut db = db.write().await;
    db.transaction_mut(|txn| -> Result<(), DbError> {
        let Some(comment) =
            crate::repo::db::resolve_node_id_in_txn(txn, ENTITY_TYPE_COMMENT, comment_id)?
        else {
            return Ok(());
        };
        delete_comment_tree_in_txn(txn, comment)?;
        Ok(())
    })
}

fn delete_article_in_txn(
    txn: &mut agdb::DbAnyTransactionMut,
    article: agdb::DbId,
    outcome: &mut HardDeleteOutcome,
) -> Result<(), DbError> {
    let version_edges = txn.exec(
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
        delete_version_in_txn(txn, edge.to, outcome)?;
    }
    txn.exec_mut(QueryBuilder::remove().ids([article]).query())?;
    Ok(())
}

fn delete_version_in_txn(
    txn: &mut agdb::DbAnyTransactionMut,
    version: agdb::DbId,
    outcome: &mut HardDeleteOutcome,
) -> Result<(), DbError> {
    let comment_edges = txn.exec(
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
        delete_comment_tree_in_txn(txn, edge.from)?;
    }
    if let Some(row) = crate::repo::db::read_node_in_txn::<VersionRow>(txn, version)? {
        outcome.removed_pdf_hashes.push(row.content_hash);
    }
    txn.exec_mut(QueryBuilder::remove().ids([version]).query())?;
    Ok(())
}

fn delete_comment_tree_in_txn(
    txn: &mut agdb::DbAnyTransactionMut,
    comment: agdb::DbId,
) -> Result<(), DbError> {
    let reply_edges = txn.exec(
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
        delete_comment_tree_in_txn(txn, edge.to)?;
    }
    txn.exec_mut(QueryBuilder::remove().ids([comment]).query())?;
    Ok(())
}

fn refresh_latest_version_in_txn(
    txn: &mut agdb::DbAnyTransactionMut,
    article: agdb::DbId,
) -> Result<(), DbError> {
    let version_edges = txn.exec(
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
    let mut latest: Option<agdb::DbId> = None;
    let mut latest_id: Option<String> = None;
    for edge in &version_edges.elements {
        if let Some(row) = crate::repo::db::read_node_in_txn::<VersionRow>(txn, edge.to)? {
            if latest_id.as_deref().map_or(true, |cur| row.id.as_str() > cur) {
                latest = Some(edge.to);
                latest_id = Some(row.id);
            }
        }
    }
    let value = match (latest, latest_id) {
        (Some(node), Some(business_id)) => {
            let _ = node;
            Some(business_id)
        }
        _ => None,
    };
    let value = agdb::DbValue::String(value.unwrap_or_default());
    txn.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .ids([article])
            .values([[(crate::repo::types::KEY_LATEST_VERSION_ID, value).into()]])
            .query(),
    )?;
    Ok(())
}
