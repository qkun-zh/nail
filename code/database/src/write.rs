use std::collections::HashSet;

use agdb::{DbAnyTransactionMut, DbId, QueryBuilder};

use crate::condition::{Condition, Order};
use crate::error::Error;
use crate::kinds::{EdgeKind, NodeKind, TYPE_KEY, alias_of};
use crate::node_id::NodeId;
use crate::read::{
    all_nodes, count_incoming, count_nodes, count_outgoing, incoming, is_not_found, outgoing,
    read_nodes, read_value, resolve, scan_nodes,
};
use crate::row::Row;
use crate::value::Value;

/// Writable view of the database inside a `Database::write` closure. Has
/// every read method plus the write primitives; commits on `Ok`, rolls back
/// on `Err`.
pub struct WriteScope<'db, 'txn> {
    txn: &'txn mut DbAnyTransactionMut<'db>,
}

impl<'db, 'txn> WriteScope<'db, 'txn> {
    pub(crate) fn new(txn: &'txn mut DbAnyTransactionMut<'db>) -> Self {
        Self { txn }
    }

    fn reader(&self) -> &DbAnyTransactionMut<'db> {
        self.txn
    }

    fn row_key_values<T: Row>(row: &T) -> Vec<agdb::DbKeyValue> {
        let mut values = Vec::with_capacity(row.to_row().len() + 1);
        values.push((TYPE_KEY, T::KIND.key().to_string()).into());
        for (key, value) in row.to_row() {
            values.push((key.as_str(), agdb::DbValue::from(value)).into());
        }
        values
    }

    /// Inserts a node or replaces an existing one with the same business id.
    /// Full-row replace semantics: keys present on the stored node but
    /// absent from the new row are cleared.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the underlying queries fail.
    pub fn insert_node<T: Row>(&mut self, row: &T) -> Result<NodeId, Error> {
        let alias = alias_of(T::KIND, row.business_id());
        let existing = resolve(self.reader(), T::KIND, row.business_id())?;
        if let Some(node) = existing {
            self.replace_row(node, row)?;
            return Ok(node);
        }
        let result = self.txn.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases([alias])
                .values([Self::row_key_values(row)])
                .query(),
        )?;
        result
            .elements
            .first()
            .map(|element| NodeId::from_db(element.id))
            .ok_or_else(|| Error::Storage("insert_node returned no element".to_string()))
    }

    fn replace_row<T: Row>(&mut self, node: NodeId, row: &T) -> Result<(), Error> {
        let mut fresh_keys: HashSet<String> = HashSet::new();
        fresh_keys.insert(TYPE_KEY.to_string());
        for (key, _) in row.to_row() {
            fresh_keys.insert(key);
        }
        let current = self
            .txn
            .exec(QueryBuilder::select().ids([node.to_db()]).query())?;
        let current_keys: Vec<String> = current
            .elements
            .first()
            .map(|element| {
                element
                    .values
                    .iter()
                    .filter_map(|pair| pair.key.string().ok())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let stale: Vec<&str> = current_keys
            .iter()
            .map(String::as_str)
            .filter(|key| !fresh_keys.contains(*key))
            .collect();
        if !stale.is_empty() {
            self.txn.exec_mut(
                QueryBuilder::remove()
                    .values(stale)
                    .ids([node.to_db()])
                    .query(),
            )?;
        }
        self.txn.exec_mut(
            QueryBuilder::insert()
                .values([Self::row_key_values(row)])
                .ids([node.to_db()])
                .query(),
        )?;
        Ok(())
    }

    /// Inserts many nodes of one kind in a single query. Intended for
    /// seeding; does not clear stale keys on alias collisions.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the insert fails.
    pub fn insert_nodes<T: Row>(&mut self, rows: &[T]) -> Result<Vec<NodeId>, Error> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let aliases: Vec<String> = rows
            .iter()
            .map(|row| alias_of(T::KIND, row.business_id()))
            .collect();
        let values: Vec<Vec<agdb::DbKeyValue>> = rows.iter().map(Self::row_key_values).collect();
        let result = self.txn.exec_mut(
            QueryBuilder::insert()
                .nodes()
                .aliases(aliases)
                .values(values)
                .query(),
        )?;
        Ok(result
            .elements
            .iter()
            .map(|element| NodeId::from_db(element.id))
            .collect())
    }

    /// Inserts an edge between existing nodes; inserting an edge that
    /// already exists is a no-op.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] if either endpoint does not exist and
    /// [`Error::Storage`] if the insert fails.
    pub fn insert_edge(
        &mut self,
        from_kind: NodeKind,
        from: NodeId,
        edge_kind: EdgeKind,
        to_kind: NodeKind,
        to: NodeId,
    ) -> Result<(), Error> {
        self.ensure_node(from_kind, from)?;
        self.ensure_node(to_kind, to)?;
        if outgoing(self.reader(), from, edge_kind)?.contains(&to) {
            return Ok(());
        }
        self.txn.exec_mut(
            QueryBuilder::insert()
                .edges()
                .from([from.to_db()])
                .to([to.to_db()])
                .values([vec![(TYPE_KEY, edge_kind.key().to_string()).into()]])
                .query(),
        )?;
        Ok(())
    }

    fn ensure_node(&mut self, kind: NodeKind, id: NodeId) -> Result<(), Error> {
        match self
            .txn
            .exec(QueryBuilder::select().ids([id.to_db()]).query())
        {
            Ok(_) => Ok(()),
            Err(error) if is_not_found(&error) => Err(Error::not_found(kind, id.to_string())),
            Err(error) => Err(error.into()),
        }
    }

    /// Removes the edge between two nodes if present; absent edges are a
    /// no-op.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the underlying queries fail.
    pub fn remove_edge(
        &mut self,
        from: NodeId,
        edge_kind: EdgeKind,
        to: NodeId,
    ) -> Result<(), Error> {
        let targets = outgoing(self.reader(), from, edge_kind)?;
        if !targets.contains(&to) {
            return Ok(());
        }
        let edge_id = self.find_edge_id(from, edge_kind, to)?;
        if let Some(edge_id) = edge_id {
            self.txn
                .exec_mut(QueryBuilder::remove().ids([edge_id]).query())?;
        }
        Ok(())
    }

    fn find_edge_id(
        &mut self,
        from: NodeId,
        edge_kind: EdgeKind,
        to: NodeId,
    ) -> Result<Option<DbId>, Error> {
        let result = self.txn.exec(
            QueryBuilder::search()
                .from(from.to_db())
                .where_()
                .distance(agdb::CountComparison::Equal(1))
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
            .find(|element| element.to == to.to_db())
            .map(|element| element.id))
    }

    /// Removes nodes together with their attached edges. Absent ids are a
    /// no-op.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the removal fails.
    pub fn remove(&mut self, ids: &[NodeId]) -> Result<(), Error> {
        if ids.is_empty() {
            return Ok(());
        }
        let raw_ids: Vec<DbId> = ids.iter().map(|id| id.to_db()).collect();
        self.txn
            .exec_mut(QueryBuilder::remove().ids(raw_ids).query())?;
        Ok(())
    }

    /// Sets one key on a node (upsert).
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the write fails.
    pub fn set_key(&mut self, id: NodeId, key: &str, value: Value) -> Result<(), Error> {
        self.txn.exec_mut(
            QueryBuilder::insert()
                .values([[(key, agdb::DbValue::from(value)).into()]])
                .ids([id.to_db()])
                .query(),
        )?;
        Ok(())
    }

    /// Removes one key from a node; absent keys are a no-op.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the removal fails.
    pub fn clear_key(&mut self, id: NodeId, key: &str) -> Result<(), Error> {
        self.txn.exec_mut(
            QueryBuilder::remove()
                .values([key])
                .ids([id.to_db()])
                .query(),
        )?;
        Ok(())
    }

    /// Resolves a business id to its node handle via the kind's alias.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the alias lookup fails for a reason
    /// other than absence.
    pub fn resolve(&self, kind: NodeKind, business_id: &str) -> Result<Option<NodeId>, Error> {
        resolve(self.reader(), kind, business_id)
    }

    /// Reads one node as a typed row.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] if a required row key is missing or has
    /// the wrong value kind, and [`Error::Storage`] on lookup failure.
    pub fn read_node<T: Row>(&self, id: NodeId) -> Result<Option<T>, Error> {
        Ok(read_nodes::<T>(self.reader(), &[id])?.into_iter().next())
    }

    /// Reads many nodes as typed rows in the given order.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] if any id does not exist,
    /// [`Error::Invalid`] for malformed rows, [`Error::Storage`] otherwise.
    pub fn read_nodes<T: Row>(&self, ids: &[NodeId]) -> Result<Vec<T>, Error> {
        read_nodes::<T>(self.reader(), ids)
    }

    /// Lists every node of a kind.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the scan fails.
    pub fn all_nodes(&self, kind: NodeKind) -> Result<Vec<NodeId>, Error> {
        all_nodes(self.reader(), kind)
    }

    /// Scans nodes of a kind with an optional filter, deterministic order,
    /// and offset/limit pagination applied to matching elements.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the scan fails.
    pub fn scan_nodes(
        &self,
        kind: NodeKind,
        condition: Option<&Condition>,
        order: &Order,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<NodeId>, Error> {
        scan_nodes(self.reader(), kind, condition, order, offset, limit)
    }

    /// Counts nodes of a kind matching an optional filter.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the scan fails.
    pub fn count_nodes(&self, kind: NodeKind, condition: Option<&Condition>) -> Result<u64, Error> {
        count_nodes(self.reader(), kind, condition)
    }

    /// Lists far-endpoint nodes of outgoing edges of an edge kind.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the traversal fails.
    pub fn outgoing(&self, from: NodeId, edge_kind: EdgeKind) -> Result<Vec<NodeId>, Error> {
        outgoing(self.reader(), from, edge_kind)
    }

    /// Lists start-node handles of incoming edges of an edge kind.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the traversal fails.
    pub fn incoming(&self, to: NodeId, edge_kind: EdgeKind) -> Result<Vec<NodeId>, Error> {
        incoming(self.reader(), to, edge_kind)
    }

    /// Counts outgoing edges of an edge kind.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the traversal fails.
    pub fn count_outgoing(&self, from: NodeId, edge_kind: EdgeKind) -> Result<u64, Error> {
        count_outgoing(self.reader(), from, edge_kind)
    }

    /// Counts incoming edges of an edge kind.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the traversal fails.
    pub fn count_incoming(&self, to: NodeId, edge_kind: EdgeKind) -> Result<u64, Error> {
        count_incoming(self.reader(), to, edge_kind)
    }

    /// Reads one non-row metadata key from a node.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] if the node does not exist and
    /// [`Error::Invalid`] if the stored value does not convert to `T`.
    pub fn read_value<T>(&self, kind: NodeKind, id: NodeId, key: &str) -> Result<Option<T>, Error>
    where
        T: std::convert::TryFrom<Value, Error = Value>,
    {
        read_value::<T>(self.reader(), kind, id, key)
    }
}
