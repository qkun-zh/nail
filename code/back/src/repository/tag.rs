use agdb::{DbAnyTransactionMut, DbError, QueryBuilder};
use nail_common::tag::TagRef;

use crate::repository::graph::DbHandle;
use crate::repository::graph::{
    find_by_index, incoming_edges, insert_edge, outgoing_edges, read_node, resolve_node_id,
};
use crate::repository::schema::{
    EDGE_ARTICLE_APPLY_TAG, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_TAG, IdRow, KEY_TAG_NAME, KEY_TYPE,
    TagRow, alias_of,
};

pub fn create_tag_in_txn(
    transaction: &mut DbAnyTransactionMut,
    name: &str,
) -> Result<TagRef, DbError> {
    if let Some(existing_id) = find_tag_id_by_name_in_txn(transaction, name)? {
        return Ok(TagRef {
            id: existing_id,
            name: name.to_string(),
        });
    }
    let tag_id = uuid::Uuid::now_v7().to_string();
    transaction.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .aliases([alias_of(ENTITY_TYPE_TAG, &tag_id)])
            .values(TagRow {
                db_id: None,
                entity_type: ENTITY_TYPE_TAG.to_string(),
                id: tag_id.clone(),
                tag_name: name.to_string(),
            })
            .query(),
    )?;
    Ok(TagRef {
        id: tag_id,
        name: name.to_string(),
    })
}

fn find_tag_id_by_name_in_txn(
    transaction: &DbAnyTransactionMut,
    name: &str,
) -> Result<Option<String>, DbError> {
    let ids = find_by_index(transaction, KEY_TAG_NAME, name)?;
    let Some(id) = ids.first() else {
        return Ok(None);
    };
    Ok(read_node::<IdRow>(transaction, *id)?.map(|row| row.id))
}

pub struct TagViewRow {
    pub id: String,
    pub tag_name: String,
}

pub async fn create_tag(db: &DbHandle, name: &str) -> Result<String, DbError> {
    let mut guard = db.write().await;
    if !find_by_index(&guard, KEY_TAG_NAME, name)?.is_empty() {
        return Ok(name.to_string());
    }
    let tag_id = uuid::Uuid::now_v7().to_string();
    guard.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .aliases([alias_of(ENTITY_TYPE_TAG, &tag_id)])
            .values(TagRow {
                db_id: None,
                entity_type: ENTITY_TYPE_TAG.to_string(),
                id: tag_id.clone(),
                tag_name: name.to_string(),
            })
            .query(),
    )?;
    Ok(tag_id)
}

pub async fn read_tag_by_id(db: &DbHandle, tag_id: &str) -> Result<Option<TagViewRow>, DbError> {
    let guard = db.read().await;
    let Some(db_id) = resolve_node_id(&guard, ENTITY_TYPE_TAG, tag_id)? else {
        return Ok(None);
    };
    let row = read_node::<TagRow>(&guard, db_id)?;
    Ok(row.map(|r| TagViewRow {
        id: r.id,
        tag_name: r.tag_name,
    }))
}

pub async fn read_tag_by_name(db: &DbHandle, name: &str) -> Result<Option<TagViewRow>, DbError> {
    let guard = db.read().await;
    let ids = find_by_index(&guard, KEY_TAG_NAME, name)?;
    let Some(id) = ids.first() else {
        return Ok(None);
    };
    let row = read_node::<TagRow>(&guard, *id)?;
    Ok(row.map(|r| TagViewRow {
        id: r.id,
        tag_name: r.tag_name,
    }))
}

pub async fn read_tags(db: &DbHandle) -> Result<Vec<TagViewRow>, DbError> {
    let guard = db.read().await;
    let result = guard.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_TAG)
            .query(),
    )?;
    let mut tags = Vec::new();
    for element in &result.elements {
        if let Some(row) = read_node::<TagRow>(&guard, element.id)? {
            tags.push(TagViewRow {
                id: row.id,
                tag_name: row.tag_name,
            });
        }
    }
    tags.sort_by(|left, right| left.tag_name.cmp(&right.tag_name));
    Ok(tags)
}

pub async fn update_tag(db: &DbHandle, tag_id: &str, new_name: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let Some(db_id) = resolve_node_id(&guard, ENTITY_TYPE_TAG, tag_id)? else {
        return Ok(());
    };
    guard.exec_mut(
        QueryBuilder::insert()
            .values([[(KEY_TAG_NAME, agdb::DbValue::String(new_name.to_string())).into()]])
            .ids([db_id])
            .query(),
    )?;
    Ok(())
}

pub async fn delete_tag(db: &DbHandle, tag_id: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let Some(db_id) = resolve_node_id(&guard, ENTITY_TYPE_TAG, tag_id)? else {
        return Ok(());
    };
    let edges = incoming_edges(&guard, db_id, EDGE_ARTICLE_APPLY_TAG)?;
    let edge_ids: Vec<agdb::DbId> = edges.iter().map(|edge| edge.id).collect();
    if !edge_ids.is_empty() {
        guard.exec_mut(QueryBuilder::remove().ids(edge_ids).query())?;
    }
    guard.exec_mut(QueryBuilder::remove().ids([db_id]).query())?;
    Ok(())
}

pub async fn count_tag_articles(db: &DbHandle, tag_id: &str) -> Result<u64, DbError> {
    let guard = db.read().await;
    let Some(db_id) = resolve_node_id(&guard, ENTITY_TYPE_TAG, tag_id)? else {
        return Ok(0);
    };
    let edges = incoming_edges(&guard, db_id, EDGE_ARTICLE_APPLY_TAG)?;
    Ok(edges.len() as u64)
}

pub async fn apply_tag_to_article(
    db: &DbHandle,
    article_id: &str,
    tag_id: &str,
) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let Some(article_db_id) = resolve_node_id(&guard, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(());
    };
    let Some(tag_db_id) = resolve_node_id(&guard, ENTITY_TYPE_TAG, tag_id)? else {
        return Ok(());
    };
    let edges = outgoing_edges(&guard, article_db_id, EDGE_ARTICLE_APPLY_TAG)?;
    if !edges.iter().any(|edge| edge.to == tag_db_id) {
        insert_edge(
            &mut guard,
            EDGE_ARTICLE_APPLY_TAG,
            article_db_id.into(),
            tag_db_id.into(),
        )?;
    }
    Ok(())
}

pub async fn unapply_tag_from_article(
    db: &DbHandle,
    article_id: &str,
    tag_id: &str,
) -> Result<(), DbError> {
    let mut guard = db.write().await;
    let Some(article_db_id) = resolve_node_id(&guard, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(());
    };
    let Some(tag_db_id) = resolve_node_id(&guard, ENTITY_TYPE_TAG, tag_id)? else {
        return Ok(());
    };
    let edges = outgoing_edges(&guard, article_db_id, EDGE_ARTICLE_APPLY_TAG)?;
    if let Some(edge) = edges.iter().find(|edge| edge.to == tag_db_id) {
        guard.exec_mut(QueryBuilder::remove().ids([edge.id]).query())?;
    }
    Ok(())
}

#[cfg(test)]
pub async fn read_tag_articles(db: &DbHandle, tag_id: &str) -> Result<Vec<String>, DbError> {
    let guard = db.read().await;
    let Some(db_id) = resolve_node_id(&guard, ENTITY_TYPE_TAG, tag_id)? else {
        return Ok(Vec::new());
    };
    let edges = incoming_edges(&guard, db_id, EDGE_ARTICLE_APPLY_TAG)?;
    let mut article_ids = Vec::new();
    for edge in &edges {
        if let Some(row) = read_node::<IdRow>(&guard, edge.from)? {
            article_ids.push(row.id);
        }
    }
    Ok(article_ids)
}
