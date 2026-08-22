use agdb::{
    CountComparison, DbAnyTransaction, DbError, DbErrorType, DbValue, Query, QueryBuilder,
    QueryResult, Where, WhereLogicOperator,
};

use crate::error::Error;
use crate::kinds::{EdgeKind, NodeKind, TYPE_KEY, alias_of};
use crate::node_id::NodeId;
use crate::row::{ElementLookup, Row};
use crate::value::Value;

pub(crate) trait QueryReader {
    fn run<T: Query>(&self, query: T) -> Result<QueryResult, DbError>;
}

impl QueryReader for DbAnyTransaction<'_> {
    fn run<T: Query>(&self, query: T) -> Result<QueryResult, DbError> {
        self.exec(query)
    }
}

impl QueryReader for agdb::DbAnyTransactionMut<'_> {
    fn run<T: Query>(&self, query: T) -> Result<QueryResult, DbError> {
        self.exec(query)
    }
}

pub(crate) fn is_not_found(error: &DbError) -> bool {
    error.ty == DbErrorType::NotFound
}

pub(crate) fn resolve(
    reader: &impl QueryReader,
    kind: NodeKind,
    business_id: &str,
) -> Result<Option<NodeId>, Error> {
    match reader.run(
        QueryBuilder::select()
            .ids([alias_of(kind, business_id)])
            .query(),
    ) {
        Ok(result) => Ok(result
            .elements
            .first()
            .map(|element| NodeId::from_db(element.id))),
        Err(error) if is_not_found(&error) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn find_by_key(
    reader: &impl QueryReader,
    key: &str,
    value: &str,
) -> Result<Option<NodeId>, Error> {
    let result = reader.run(
        QueryBuilder::select()
            .values([key])
            .search()
            .index(key)
            .value(value)
            .query(),
    )?;
    Ok(result
        .elements
        .first()
        .map(|element| NodeId::from_db(element.id)))
}

pub(crate) fn read_nodes<T: Row>(
    reader: &impl QueryReader,
    ids: &[NodeId],
) -> Result<Vec<T>, Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let raw_ids: Vec<agdb::DbId> = ids.iter().map(|id| id.to_db()).collect();
    let result = reader
        .run(QueryBuilder::select().ids(raw_ids).query())
        .map_err(|error| {
            if is_not_found(&error) {
                let listed = ids
                    .iter()
                    .map(NodeId::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                Error::not_found(T::KIND, listed)
            } else {
                error.into()
            }
        })?;
    result
        .elements
        .iter()
        .map(|element| T::from_lookup(&ElementLookup::new(&element.values)))
        .collect()
}

pub(crate) fn all_nodes(reader: &impl QueryReader, kind: NodeKind) -> Result<Vec<NodeId>, Error> {
    let result =
        reader.run(kind_filter(QueryBuilder::search().elements().where_(), kind).query())?;
    Ok(result
        .elements
        .iter()
        .map(|element| NodeId::from_db(element.id))
        .collect())
}

fn kind_filter(
    where_: Where<agdb::SearchQuery>,
    kind: NodeKind,
) -> WhereLogicOperator<agdb::SearchQuery> {
    where_
        .key(TYPE_KEY.to_string())
        .value(DbValue::from(Value::Text(kind.key().to_string())))
}

pub(crate) fn outgoing(
    reader: &impl QueryReader,
    from: NodeId,
    edge_kind: EdgeKind,
) -> Result<Vec<NodeId>, Error> {
    let result = reader.run(
        QueryBuilder::search()
            .from(from.to_db())
            .where_()
            .distance(CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(TYPE_KEY)
            .value(edge_kind.key())
            .query(),
    )?;
    Ok(result
        .elements
        .iter()
        .map(|element| NodeId::from_db(element.to))
        .collect())
}

pub(crate) fn incoming(
    reader: &impl QueryReader,
    to: NodeId,
    edge_kind: EdgeKind,
) -> Result<Vec<NodeId>, Error> {
    let result = reader.run(
        QueryBuilder::search()
            .to(to.to_db())
            .where_()
            .distance(CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(TYPE_KEY)
            .value(edge_kind.key())
            .query(),
    )?;
    Ok(result
        .elements
        .iter()
        .map(|element| NodeId::from_db(element.from))
        .collect())
}

pub(crate) fn count_outgoing(
    reader: &impl QueryReader,
    from: NodeId,
    edge_kind: EdgeKind,
) -> Result<u64, Error> {
    Ok(outgoing(reader, from, edge_kind)?.len() as u64)
}

pub(crate) fn count_incoming(
    reader: &impl QueryReader,
    to: NodeId,
    edge_kind: EdgeKind,
) -> Result<u64, Error> {
    Ok(incoming(reader, to, edge_kind)?.len() as u64)
}
