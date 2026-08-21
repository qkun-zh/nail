use std::convert::TryFrom;

use agdb::{
    Comparison, CountComparison, DbAnyTransaction, DbError, DbErrorType, DbValue, Query,
    QueryBuilder, QueryResult, Where, WhereLogicOperator,
};

use crate::condition::{Condition, Order};
use crate::error::Error;
use crate::kinds::{EdgeKind, NodeKind, TYPE_KEY, alias_of};
use crate::node_id::NodeId;
use crate::row::{ElementLookup, Row, ValueLookup};
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
        reader.run(kind_filter(QueryBuilder::search().elements().where_(), kind, None).query())?;
    Ok(result
        .elements
        .iter()
        .map(|element| NodeId::from_db(element.id))
        .collect())
}

pub(crate) fn scan_nodes(
    reader: &impl QueryReader,
    kind: NodeKind,
    condition: Option<&Condition>,
    order: &Order,
    offset: u64,
    limit: u64,
) -> Result<Vec<NodeId>, Error> {
    let builder = QueryBuilder::search()
        .elements()
        .order_by([db_key_order(order)])
        .offset(offset)
        .limit(limit)
        .where_();
    let result = reader.run(kind_filter(builder, kind, condition).query())?;
    Ok(result
        .elements
        .iter()
        .map(|element| NodeId::from_db(element.id))
        .collect())
}

pub(crate) fn count_nodes(
    reader: &impl QueryReader,
    kind: NodeKind,
    condition: Option<&Condition>,
) -> Result<u64, Error> {
    let result = reader
        .run(kind_filter(QueryBuilder::search().elements().where_(), kind, condition).query())?;
    Ok(result.elements.len() as u64)
}

fn kind_filter(
    where_: Where<agdb::SearchQuery>,
    kind: NodeKind,
    condition: Option<&Condition>,
) -> WhereLogicOperator<agdb::SearchQuery> {
    let mut filters = vec![type_condition(kind)];
    if let Some(condition) = condition {
        filters.push(condition.clone());
    }
    apply_all(where_, &filters)
}

fn type_condition(kind: NodeKind) -> Condition {
    Condition::KeyEquals(TYPE_KEY.to_string(), Value::Text(kind.key().to_string()))
}

fn apply_all(
    where_: Where<agdb::SearchQuery>,
    filters: &[Condition],
) -> WhereLogicOperator<agdb::SearchQuery> {
    let mut operator = apply_condition(where_, &filters[0]);
    for filter in &filters[1..] {
        operator = apply_condition(operator.and(), filter);
    }
    operator
}

fn apply_condition(
    where_: Where<agdb::SearchQuery>,
    condition: &Condition,
) -> WhereLogicOperator<agdb::SearchQuery> {
    match condition {
        Condition::KeyEquals(key, value) => {
            where_.key(key.clone()).value(DbValue::from(value.clone()))
        }
        Condition::KeyGreaterThan(key, value) => where_
            .key(key.clone())
            .value(Comparison::GreaterThan(DbValue::from(value.clone()))),
        Condition::KeyNotExists(key) => where_.not().keys(key.as_str()),
        Condition::All(items) => apply_all(where_, items),
    }
}

fn db_key_order(order: &Order) -> agdb::DbKeyOrder {
    let key = DbValue::from(order.key.clone());
    if order.ascending {
        agdb::DbKeyOrder::Asc(key)
    } else {
        agdb::DbKeyOrder::Desc(key)
    }
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

pub(crate) fn read_value<T>(
    reader: &impl QueryReader,
    kind: NodeKind,
    id: NodeId,
    key: &str,
) -> Result<Option<T>, Error>
where
    T: TryFrom<Value, Error = Value>,
{
    let result = reader
        .run(QueryBuilder::select().ids([id.to_db()]).query())
        .map_err(|error| {
            if is_not_found(&error) {
                Error::not_found(kind, id.to_string())
            } else {
                error.into()
            }
        })?;
    let element = result
        .elements
        .first()
        .ok_or_else(|| Error::not_found(kind, id.to_string()))?;
    let lookup = ElementLookup::new(&element.values);
    match lookup.get(key) {
        Some(value) => T::try_from(value).map(Some).map_err(|_| {
            Error::Invalid(format!("key {key} does not convert to the requested type"))
        }),
        None => Ok(None),
    }
}
