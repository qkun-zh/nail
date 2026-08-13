use agdb::{DbAnyTransactionMut, DbError, QueryBuilder};
use nail_common::tag::TagRef;

use crate::repository::graph::{find_by_index_in_txn, read_node_in_txn};
use crate::repository::schema::{ENTITY_TYPE_TAG, IdRow, KEY_TAG_NAME, TagRow, alias_of};

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
    let ids = find_by_index_in_txn(transaction, KEY_TAG_NAME, name)?;
    let Some(id) = ids.first() else {
        return Ok(None);
    };
    Ok(read_node_in_txn::<IdRow>(transaction, *id)?.map(|row| row.id))
}
