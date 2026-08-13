use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use agdb::{DbAny, DbError, DbErrorType, DbType, DbTypeMarker, QueryBuilder};
use tokio::sync::RwLock;

use crate::repository::schema::alias_of;

pub type DbHandle = Arc<RwLock<DbAny>>;

pub async fn open(address: &str) -> anyhow::Result<DbHandle> {
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
                    .map_err(|error| anyhow::anyhow!("create db_path parent {parent:?}: {error}"))?;
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
    let alias = alias_of(kind, business_id);
    match db.exec(QueryBuilder::select().ids([alias]).query()) {
        Ok(result) => Ok(result.elements.first().map(|element| element.id)),
        Err(error) if is_not_found(&error) => Ok(None),
        Err(error) => Err(error),
    }
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
    Ok(result.elements.iter().map(|element| element.id).collect())
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
    Ok(result.try_into()?)
}

pub(crate) fn read_node_sync<T>(db: &DbAny, id: agdb::DbId) -> Result<Option<T>, DbError>
where
    T: DbType<ValueType = T> + DbTypeMarker,
{
    Ok(read_rows_sync::<T>(db, std::slice::from_ref(&id))?
        .into_iter()
        .next())
}

pub(crate) fn existing_index_keys(db: &DbAny) -> Result<HashSet<String>, DbError> {
    let result = db.exec(QueryBuilder::select().indexes().query())?;
    let mut keys = HashSet::new();
    if let Some(element) = result.elements.first() {
        for key_value in &element.values {
            if let Ok(key) = key_value.key.string() {
                keys.insert(key.clone());
            }
        }
    }
    Ok(keys)
}
