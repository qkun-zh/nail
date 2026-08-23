use agdb::DbAnyTransaction;

use crate::error::Error;
use crate::kinds::{EdgeKind, NodeKind};
use crate::node_id::NodeId;
use crate::read::{
    all_nodes, count_incoming, count_outgoing, find_by_key, incoming, outgoing, read_nodes, resolve,
};
use crate::row::Row;

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

    pub fn resolve(&self, kind: NodeKind, business_id: &str) -> Result<Option<NodeId>, Error> {
        resolve(self.reader(), kind, business_id)
    }

    pub fn find_by_key(&self, key: &str, value: &str) -> Result<Option<NodeId>, Error> {
        find_by_key(self.reader(), key, value)
    }

    pub fn read_node<T: Row>(&self, id: NodeId) -> Result<Option<T>, Error> {
        Ok(read_nodes::<T>(self.reader(), &[id])?.into_iter().next())
    }

    pub fn read_nodes<T: Row>(&self, ids: &[NodeId]) -> Result<Vec<T>, Error> {
        read_nodes::<T>(self.reader(), ids)
    }

    pub fn all_nodes(&self, kind: NodeKind) -> Result<Vec<NodeId>, Error> {
        all_nodes(self.reader(), kind)
    }

    pub fn outgoing(&self, from: NodeId, edge_kind: EdgeKind) -> Result<Vec<NodeId>, Error> {
        outgoing(self.reader(), from, edge_kind)
    }

    pub fn incoming(&self, to: NodeId, edge_kind: EdgeKind) -> Result<Vec<NodeId>, Error> {
        incoming(self.reader(), to, edge_kind)
    }

    pub fn count_outgoing(&self, from: NodeId, edge_kind: EdgeKind) -> Result<u64, Error> {
        count_outgoing(self.reader(), from, edge_kind)
    }

    pub fn count_incoming(&self, to: NodeId, edge_kind: EdgeKind) -> Result<u64, Error> {
        count_incoming(self.reader(), to, edge_kind)
    }
}
