
use agdb::{DbAny, DbAnyTransactionMut, DbError, QueryBuilder};

use common::tag::TagRef;

use crate::repo::db::{find_by_index_in_txn, find_by_index_sync, read_node_in_txn};
use crate::repo::types::{ENTITY_TYPE_TAG, IdRow, KEY_TAG_NAME, TagRow, alias_of};

pub fn get_or_create_tag_in_txn(
    txn: &mut DbAnyTransactionMut,
    name: &str,
) -> Result<TagRef, DbError> {
    if let Some(existing_id) = find_tag_id_by_name_in_txn(txn, name)? {
        return Ok(TagRef {
            id: existing_id,
            name: name.to_string(),
        });
    }
    let tag_id = uuid::Uuid::now_v7().to_string();
    let alias = alias_of(ENTITY_TYPE_TAG, &tag_id);
    txn.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .aliases([alias])
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

#[allow(dead_code)]
pub(crate) fn get_or_create_tag_sync(db: &mut DbAny, name: &str) -> Result<agdb::DbId, DbError> {
    if let Some(id) = find_tag_id_by_name_sync(db, name)? {
        return Ok(id);
    }
    let tag_id = uuid::Uuid::now_v7().to_string();
    let alias = alias_of(ENTITY_TYPE_TAG, &tag_id);
    let result = db.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .aliases([alias])
            .values(TagRow {
                db_id: None,
                entity_type: ENTITY_TYPE_TAG.to_string(),
                id: tag_id,
                tag_name: name.to_string(),
            })
            .query(),
    )?;
    result
        .elements
        .first()
        .map(|el| el.id)
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, "inserted tag id missing"))
}

fn find_tag_id_by_name_in_txn(
    txn: &DbAnyTransactionMut,
    name: &str,
) -> Result<Option<String>, DbError> {
    let ids = find_by_index_in_txn(txn, KEY_TAG_NAME, name)?;
    let Some(id) = ids.first() else {
        return Ok(None);
    };
    Ok(read_node_in_txn::<IdRow>(txn, *id)?.map(|r| r.id))
}

#[allow(dead_code)]
fn find_tag_id_by_name_sync(db: &DbAny, name: &str) -> Result<Option<agdb::DbId>, DbError> {
    Ok(find_by_index_sync(db, KEY_TAG_NAME, name)?.first().copied())
}

#[cfg(test)]
pub async fn find_tag_ids_by_names_contains(
    db: &crate::repo::DbHandle,
    names: &[String],
) -> Result<Vec<(String, String)>, DbError> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let db = db.read().await;
    let all = db.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(crate::repo::types::KEY_TYPE)
            .value(ENTITY_TYPE_TAG)
            .query(),
    )?;
    let mut out = Vec::new();
    for el in &all.elements {
        let Some(name) = crate::repo::db::read_node_sync::<crate::repo::types::TagRow>(&db, el.id)?
            .map(|r| r.tag_name)
        else {
            continue;
        };
        if names.iter().any(|n| name.contains(n.as_str())) {
            let Some(id) = crate::repo::db::read_node_sync::<IdRow>(&db, el.id)?.map(|r| r.id)
            else {
                continue;
            };
            out.push((name, id));
        }
    }
    Ok(out)
}

#[cfg(test)]
pub async fn read_article_tags(
    db: &crate::repo::DbHandle,
    article_id: &str,
) -> Result<Vec<serde_json::Value>, DbError> {
    let db = db.read().await;
    let edges = db.exec(
        QueryBuilder::search()
            .from(alias_of(
                crate::repo::types::ENTITY_TYPE_ARTICLE,
                article_id,
            ))
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(crate::repo::types::KEY_TYPE)
            .value(crate::repo::types::EDGE_ARTICLE_TO_TAG)
            .query(),
    )?;
    let mut tags: Vec<(u64, String, String)> = Vec::with_capacity(edges.elements.len());
    for el in &edges.elements {
        if let (Some(idrow), Some(name)) = (
            crate::repo::db::read_node_sync::<IdRow>(&db, el.to)?,
            crate::repo::db::read_node_sync::<crate::repo::types::TagRow>(&db, el.to)?
                .map(|r| r.tag_name),
        ) {
            tags.push((el.to.as_index(), idrow.id, name));
        }
    }
    tags.sort_by_key(|(index, _, _)| *index);
    Ok(tags
        .into_iter()
        .map(|(_, id, name)| serde_json::json!({ "id": id, "name": name }))
        .collect())
}

#[cfg(test)]
pub async fn read_tags_by_articles(
    db: &crate::repo::DbHandle,
    article_ids: &[String],
) -> Result<Vec<(String, Vec<serde_json::Value>)>, DbError> {
    let mut out = Vec::with_capacity(article_ids.len());
    for article_id in article_ids {
        let tags = read_article_tags(db, article_id).await?;
        out.push((article_id.clone(), tags));
    }
    Ok(out)
}
