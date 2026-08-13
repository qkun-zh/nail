use agdb::{DbError, QueryBuilder};

use crate::repository::graph::{DbHandle, resolve_node_id_in_txn};
use crate::repository::schema::ENTITY_TYPE_USER;

pub async fn hard_delete_user(db: &DbHandle, user_id: &str) -> Result<(), DbError> {
    let mut guard = db.write().await;
    guard.transaction_mut(|transaction| {
        if let Some(user_node) = resolve_node_id_in_txn(transaction, ENTITY_TYPE_USER, user_id)? {
            transaction.exec_mut(QueryBuilder::remove().ids([user_node]).query())?;
        }
        Ok(())
    })
}
