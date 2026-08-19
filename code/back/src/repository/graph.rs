use std::collections::HashSet;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use agdb::{DbAny, DbError, DbErrorType, DbType, DbTypeMarker, Query, QueryBuilder};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::repository::schema::{KEY_TYPE, alias_of};

pub type DbHandle = Arc<RwLock<DbAny>>;

pub fn open(address: &str) -> anyhow::Result<DbHandle> {
    let database = match address.trim().to_ascii_lowercase().as_str() {
        "memory" | "mem" | ":memory:" | "in-memory" => DbAny::new_memory("nail_memory")?,
        path => {
            if path.is_empty() || path == "/" {
                anyhow::bail!("invalid db_path: {path:?} (use a file path or a memory indicator)");
            }
            if let Some(parent) = Path::new(path).parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent).map_err(|error| {
                    anyhow::anyhow!("create db_path parent {}: {error}", parent.display())
                })?;
            }
            DbAny::new_mapped(path)?
        }
    };
    Ok(Arc::new(RwLock::new(database)))
}

pub(crate) fn is_not_found(error: &DbError) -> bool {
    error.ty == DbErrorType::NotFound
}

pub(crate) fn resolve_node_id_sync(
    database: &DbAny,
    kind: &str,
    business_id: &str,
) -> Result<Option<agdb::DbId>, DbError> {
    let alias = alias_of(kind, business_id);
    match database.exec(QueryBuilder::select().ids([alias]).query()) {
        Ok(result) => Ok(result.elements.first().map(|element| element.id)),
        Err(error) if is_not_found(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn resolve_node_id_in_txn(
    transaction: &agdb::DbAnyTransactionMut,
    kind: &str,
    business_id: &str,
) -> Result<Option<agdb::DbId>, DbError> {
    let alias = alias_of(kind, business_id);
    match transaction.exec(QueryBuilder::select().ids([alias]).query()) {
        Ok(result) => Ok(result.elements.first().map(|element| element.id)),
        Err(error) if is_not_found(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn find_by_index_sync(
    database: &DbAny,
    index_key: &str,
    value: &str,
) -> Result<Vec<agdb::DbId>, DbError> {
    let result = database.exec(
        QueryBuilder::select()
            .values([agdb::DbValue::String(index_key.to_string())])
            .search()
            .index(index_key)
            .value(value)
            .query(),
    )?;
    Ok(result.elements.iter().map(|element| element.id).collect())
}

pub(crate) fn find_by_index_in_txn(
    transaction: &agdb::DbAnyTransactionMut,
    index_key: &str,
    value: &str,
) -> Result<Vec<agdb::DbId>, DbError> {
    let result = transaction.exec(
        QueryBuilder::select()
            .values([agdb::DbValue::String(index_key.to_string())])
            .search()
            .index(index_key)
            .value(value)
            .query(),
    )?;
    Ok(result.elements.iter().map(|element| element.id).collect())
}

pub(crate) fn read_rows_in_txn<T>(
    transaction: &agdb::DbAnyTransactionMut,
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
    let result = transaction.exec(QueryBuilder::select().values(keys).ids(search_ids).query())?;
    result.try_into()
}

pub(crate) fn read_node_in_txn<T>(
    transaction: &agdb::DbAnyTransactionMut,
    id: agdb::DbId,
) -> Result<Option<T>, DbError>
where
    T: DbType<ValueType = T> + DbTypeMarker,
{
    Ok(
        read_rows_in_txn::<T>(transaction, std::slice::from_ref(&id))?
            .into_iter()
            .next(),
    )
}

pub(crate) fn read_rows_sync<T>(database: &DbAny, ids: &[agdb::DbId]) -> Result<Vec<T>, DbError>
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
    let result = database.exec(QueryBuilder::select().values(keys).ids(search_ids).query())?;
    result.try_into()
}

pub(crate) fn read_node_sync<T>(database: &DbAny, id: agdb::DbId) -> Result<Option<T>, DbError>
where
    T: DbType<ValueType = T> + DbTypeMarker,
{
    Ok(read_rows_sync::<T>(database, std::slice::from_ref(&id))?
        .into_iter()
        .next())
}

pub(crate) trait GraphQuery {
    fn exec_query<T: Query>(&self, query: T) -> Result<agdb::QueryResult, agdb::DbError>;
}

impl GraphQuery for DbAny {
    fn exec_query<T: Query>(&self, query: T) -> Result<agdb::QueryResult, agdb::DbError> {
        self.exec(query)
    }
}

impl GraphQuery for agdb::DbAnyTransactionMut<'_> {
    fn exec_query<T: Query>(&self, query: T) -> Result<agdb::QueryResult, agdb::DbError> {
        self.exec(query)
    }
}

impl GraphQuery for RwLockReadGuard<'_, DbAny> {
    fn exec_query<T: Query>(&self, query: T) -> Result<agdb::QueryResult, agdb::DbError> {
        self.deref().exec(query)
    }
}

impl GraphQuery for RwLockWriteGuard<'_, DbAny> {
    fn exec_query<T: Query>(&self, query: T) -> Result<agdb::QueryResult, agdb::DbError> {
        self.deref().exec(query)
    }
}

pub(crate) fn resolve_node_id(
    executor: &impl GraphQuery,
    kind: &str,
    business_id: &str,
) -> Result<Option<agdb::DbId>, DbError> {
    let alias = alias_of(kind, business_id);
    match executor.exec_query(QueryBuilder::select().ids([alias]).query()) {
        Ok(result) => Ok(result.elements.first().map(|element| element.id)),
        Err(error) if is_not_found(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn read_rows<T>(
    executor: &impl GraphQuery,
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
    let result =
        executor.exec_query(QueryBuilder::select().values(keys).ids(search_ids).query())?;
    result.try_into()
}

pub(crate) fn read_node<T>(executor: &impl GraphQuery, id: agdb::DbId) -> Result<Option<T>, DbError>
where
    T: DbType<ValueType = T> + DbTypeMarker,
{
    Ok(read_rows::<T>(executor, std::slice::from_ref(&id))?
        .into_iter()
        .next())
}

pub(crate) fn outgoing_edges(
    executor: &impl GraphQuery,
    from: agdb::DbId,
    edge_type: &str,
) -> Result<Vec<agdb::DbElement>, DbError> {
    let result = executor.exec_query(
        QueryBuilder::search()
            .from(from)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(edge_type)
            .query(),
    )?;
    Ok(result.elements)
}

pub(crate) fn incoming_edges(
    executor: &impl GraphQuery,
    to: agdb::DbId,
    edge_type: &str,
) -> Result<Vec<agdb::DbElement>, DbError> {
    let result = executor.exec_query(
        QueryBuilder::search()
            .to(to)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(edge_type)
            .query(),
    )?;
    Ok(result.elements)
}

pub(crate) fn existing_index_keys(database: &DbAny) -> Result<HashSet<String>, DbError> {
    let result = database.exec(QueryBuilder::select().indexes().query())?;
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

pub(crate) fn insert_edge(
    transaction: &mut agdb::DbAnyTransactionMut,
    edge_type: &str,
    from: agdb::QueryId,
    to: agdb::QueryId,
) -> Result<(), DbError> {
    transaction.exec_mut(
        QueryBuilder::insert()
            .edges()
            .from(from)
            .to([to])
            .values([[(crate::repository::schema::KEY_TYPE, edge_type).into()]])
            .query(),
    )?;
    Ok(())
}
