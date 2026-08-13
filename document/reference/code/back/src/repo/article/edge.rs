
use agdb::{DbAnyTransactionMut, DbError, QueryBuilder};

use crate::repo::db::{DbHandle, read_node_sync, resolve_node_id_sync};
use crate::repo::types::{
    EDGE_ARTICLE_TO_VERSION, EDGE_USER_TO_ARTICLE, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_VERSION, IdRow,
    KEY_TYPE,
};

#[allow(dead_code)]
pub async fn find_article_author_id(
    db: &DbHandle,
    article_id: &str,
) -> Result<Option<String>, DbError> {
    let db = db.read().await;
    let Some(article) = resolve_node_id_sync(&db, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(None);
    };
    let edges = db.exec(
        QueryBuilder::search()
            .to(article)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_TO_ARTICLE)
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

pub async fn version_belongs_to_article(
    db: &DbHandle,
    version_id: &str,
    article_id: &str,
) -> Result<bool, DbError> {
    let db = db.read().await;
    let Some(article) = resolve_node_id_sync(&db, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(false);
    };
    let Some(version) = resolve_node_id_sync(&db, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(false);
    };
    let edges = db.exec(
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
    Ok(edges.elements.iter().any(|el| el.to == version))
}

pub async fn find_article_id_by_version(
    db: &DbHandle,
    version_id: &str,
) -> Result<Option<String>, DbError> {
    let db = db.read().await;
    let Some(version) = resolve_node_id_sync(&db, ENTITY_TYPE_VERSION, version_id)? else {
        return Ok(None);
    };
    let edges = db.exec(
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
    )?;
    Ok(edges
        .elements
        .first()
        .map(|el| el.from)
        .map(|article| read_node_sync::<IdRow>(&db, article).map(|r| r.map(|row| row.id)))
        .transpose()?
        .flatten())
}

pub(crate) fn relate(
    txn: &mut DbAnyTransactionMut,
    edge_type: &str,
    from: agdb::QueryId,
    to: agdb::QueryId,
) -> Result<(), DbError> {
    txn.exec_mut(
        QueryBuilder::insert()
            .edges()
            .from(from)
            .to([to])
            .values([[(KEY_TYPE, edge_type).into()]])
            .query(),
    )?;
    Ok(())
}
