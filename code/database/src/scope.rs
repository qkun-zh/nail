use agdb::DbAnyTransaction;

use crate::condition::{Condition, Order};
use crate::error::Error;
use crate::kinds::{EdgeKind, NodeKind};
use crate::node_id::NodeId;
use crate::read::{
    all_nodes, count_incoming, count_nodes, count_outgoing, incoming, outgoing, read_nodes,
    read_value, resolve, scan_nodes,
};
use crate::row::Row;
use crate::value::Value;

/// Read-only view of the database inside a `Database::read` closure.
pub struct ReadScope<'db, 'txn> {
    txn: &'txn DbAnyTransaction<'db>,
}

impl<'db, 'txn> ReadScope<'db, 'txn> {
    pub(crate) fn new(txn: &'txn DbAnyTransaction<'db>) -> Self {
        Self { txn }
    }

    fn reader(&self) -> &DbAnyTransaction<'db> {
        self.txn
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
