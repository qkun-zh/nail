
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use agdb::{DbAny, DbError, DbErrorType, DbType, DbTypeMarker, QueryBuilder};
use tokio::sync::RwLock;

pub type DbHandle = Arc<RwLock<DbAny>>;

pub async fn new(address: &str) -> anyhow::Result<DbHandle> {
    let db = match address.trim().to_ascii_lowercase().as_str() {
        "memory" | "mem" | ":memory:" | "in-memory" => DbAny::new_memory("nail_memory")?,
        path => {
            if path.is_empty() || path == "/" {
                anyhow::bail!(
                    "invalid db_path: {path:?} (use a file path or a known memory indicator)"
                );
            }
            if let Some(parent) = Path::new(path).parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow::anyhow!("create db_path parent {parent:?}: {e}"))?;
            }
            DbAny::new_mapped(path)?
        }
    };
    Ok(Arc::new(RwLock::new(db)))
}

pub(crate) fn is_not_found(error: &DbError) -> bool {
    error.ty == DbErrorType::NotFound
}

pub(crate) fn resolve_node_id_sync(
    db: &DbAny,
    kind: &str,
    business_id: &str,
) -> Result<Option<agdb::DbId>, DbError> {
    let alias = crate::repo::types::alias_of(kind, business_id);
    match db.exec(QueryBuilder::select().ids([alias]).query()) {
        Ok(result) => Ok(result.elements.first().map(|el| el.id)),
        Err(error) if is_not_found(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn resolve_node_ids_sync(
    db: &DbAny,
    kind: &str,
    business_ids: &[String],
) -> Result<Vec<Option<agdb::DbId>>, DbError> {
    if business_ids.is_empty() {
        return Ok(Vec::new());
    }
    let aliases: Vec<agdb::QueryId> = business_ids
        .iter()
        .map(|id| agdb::QueryId::Alias(crate::repo::types::alias_of(kind, id)))
        .collect();
    let search_ids = QueryBuilder::search()
        .elements()
        .where_()
        .ids(agdb::QueryIds::Ids(aliases))
        .query();
    let result = db.exec(
        QueryBuilder::select()
            .values([agdb::DbValue::String(
                crate::repo::types::KEY_ID.to_string(),
            )])
            .ids(search_ids)
            .query(),
    )?;
    let mut by_business: std::collections::HashMap<String, agdb::DbId> = result
        .elements
        .iter()
        .filter_map(|el| {
            el.values.iter().find_map(|kv| {
                if kv.key == agdb::DbValue::String(crate::repo::types::KEY_ID.to_string()) {
                    kv.value.string().ok().map(|v| (v.clone(), el.id))
                } else {
                    None
                }
            })
        })
        .collect();
    Ok(business_ids
        .iter()
        .map(|id| by_business.remove(id))
        .collect())
}

pub(crate) fn find_by_index_sync(
    db: &DbAny,
    index_key: &str,
    value: &str,
) -> Result<Vec<agdb::DbId>, DbError> {
    let result = db.exec(
        QueryBuilder::select()
            .values([agdb::DbValue::String(index_key.to_string())])
            .search()
            .index(index_key)
            .value(value)
            .query(),
    )?;
    Ok(result.elements.iter().map(|el| el.id).collect())
}

pub(crate) fn read_rows_sync<T>(db: &DbAny, ids: &[agdb::DbId]) -> Result<Vec<T>, DbError>
where
    T: DbType<ValueType = T> + DbTypeMarker,
{
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let keys = T::db_keys();
    let search_ids = QueryBuilder::search()
        .elements()
        .where_()
        .ids(ids.to_vec())
        .query();
    let result = db.exec(QueryBuilder::select().values(keys).ids(search_ids).query())?;
    let ids_in_result: Vec<u64> = result.elements.iter().map(|e| e.id.as_index()).collect();
    let rows: Vec<T> = result.try_into()?;
    let mut by_index: BTreeMap<u64, T> = ids_in_result.into_iter().zip(rows).collect();
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(row) = by_index.remove(&id.as_index()) {
            out.push(row);
        }
    }
    Ok(out)
}

pub(crate) fn read_node_sync<T>(db: &DbAny, id: agdb::DbId) -> Result<Option<T>, DbError>
where
    T: DbType<ValueType = T> + DbTypeMarker,
{
    Ok(read_rows_sync::<T>(db, std::slice::from_ref(&id))?
        .into_iter()
        .next())
}

pub(crate) fn existing_index_keys(
    db: &DbAny,
) -> Result<std::collections::HashSet<String>, DbError> {
    let result = db.exec(QueryBuilder::select().indexes().query())?;
    let mut keys = std::collections::HashSet::new();
    if let Some(element) = result.elements.first() {
        for kv in &element.values {
            if let Ok(key) = kv.key.string() {
                keys.insert(key.clone());
            }
        }
    }
    Ok(keys)
}


pub(crate) fn resolve_node_id_in_txn(
    txn: &agdb::DbAnyTransactionMut,
    kind: &str,
    business_id: &str,
) -> Result<Option<agdb::DbId>, DbError> {
    let alias = crate::repo::types::alias_of(kind, business_id);
    match txn.exec(QueryBuilder::select().ids([alias]).query()) {
        Ok(result) => Ok(result.elements.first().map(|el| el.id)),
        Err(error) if is_not_found(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn find_by_index_in_txn(
    txn: &agdb::DbAnyTransactionMut,
    index_key: &str,
    value: &str,
) -> Result<Vec<agdb::DbId>, DbError> {
    let result = txn.exec(
        QueryBuilder::select()
            .values([agdb::DbValue::String(index_key.to_string())])
            .search()
            .index(index_key)
            .value(value)
            .query(),
    )?;
    Ok(result.elements.iter().map(|el| el.id).collect())
}

pub(crate) fn read_rows_in_txn<T>(
    txn: &agdb::DbAnyTransactionMut,
    ids: &[agdb::DbId],
) -> Result<Vec<T>, DbError>
where
    T: DbType<ValueType = T> + DbTypeMarker,
{
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let keys = T::db_keys();
    let search_ids = QueryBuilder::search()
        .elements()
        .where_()
        .ids(ids.to_vec())
        .query();
    let result = txn.exec(QueryBuilder::select().values(keys).ids(search_ids).query())?;
    let ids_in_result: Vec<u64> = result.elements.iter().map(|e| e.id.as_index()).collect();
    let rows: Vec<T> = result.try_into()?;
    let mut by_index: BTreeMap<u64, T> = ids_in_result.into_iter().zip(rows).collect();
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(row) = by_index.remove(&id.as_index()) {
            out.push(row);
        }
    }
    Ok(out)
}

pub(crate) fn read_node_in_txn<T>(
    txn: &agdb::DbAnyTransactionMut,
    id: agdb::DbId,
) -> Result<Option<T>, DbError>
where
    T: DbType<ValueType = T> + DbTypeMarker,
{
    Ok(read_rows_in_txn::<T>(txn, std::slice::from_ref(&id))?
        .into_iter()
        .next())
}
