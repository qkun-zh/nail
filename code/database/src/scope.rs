use agdb::DbAnyTransaction;

use crate::error::Error;
use crate::kinds::{EdgeKind, NodeKind};
use crate::node_id::NodeId;
use crate::read::{
    all_nodes, count_incoming, count_outgoing, find_by_key, incoming, outgoing, read_nodes, resolve,
};
use crate::row::Row;

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

    /// Finds a node by an indexed key-value pair. The key must have an
    /// index ensured at open time. Indexes are global across kinds; callers
    /// verify the returned node by reading its row.
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the key has no index or the lookup
    /// fails.
    pub fn find_by_key(&self, key: &str, value: &str) -> Result<Option<NodeId>, Error> {
        find_by_key(self.reader(), key, value)
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
}
