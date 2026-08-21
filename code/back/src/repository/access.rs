use database::{EdgeKind, Error, NodeId, NodeKind, ReadScope, Row, WriteScope};

/// Uniform read access over both scope types so internal helpers work in
/// read and write contexts alike.
pub(crate) trait GraphRead {
    fn scope_resolve(&self, kind: NodeKind, business_id: &str) -> Result<Option<NodeId>, Error>;

    fn scope_find_by_key(&self, key: &str, value: &str) -> Result<Option<NodeId>, Error>;

    fn scope_read_node<T: Row>(&self, id: NodeId) -> Result<Option<T>, Error>;

    fn scope_read_nodes<T: Row>(&self, ids: &[NodeId]) -> Result<Vec<T>, Error>;

    fn scope_outgoing(&self, from: NodeId, edge_kind: EdgeKind) -> Result<Vec<NodeId>, Error>;

    fn scope_incoming(&self, to: NodeId, edge_kind: EdgeKind) -> Result<Vec<NodeId>, Error>;

    fn scope_count_incoming(&self, to: NodeId, edge_kind: EdgeKind) -> Result<u64, Error>;
}

impl GraphRead for ReadScope<'_, '_> {
    fn scope_resolve(&self, kind: NodeKind, business_id: &str) -> Result<Option<NodeId>, Error> {
        self.resolve(kind, business_id)
    }

    fn scope_find_by_key(&self, key: &str, value: &str) -> Result<Option<NodeId>, Error> {
        self.find_by_key(key, value)
    }

    fn scope_read_node<T: Row>(&self, id: NodeId) -> Result<Option<T>, Error> {
        self.read_node::<T>(id)
    }

    fn scope_read_nodes<T: Row>(&self, ids: &[NodeId]) -> Result<Vec<T>, Error> {
        self.read_nodes::<T>(ids)
    }

    fn scope_outgoing(&self, from: NodeId, edge_kind: EdgeKind) -> Result<Vec<NodeId>, Error> {
        self.outgoing(from, edge_kind)
    }

    fn scope_incoming(&self, to: NodeId, edge_kind: EdgeKind) -> Result<Vec<NodeId>, Error> {
        self.incoming(to, edge_kind)
    }

    fn scope_count_incoming(&self, to: NodeId, edge_kind: EdgeKind) -> Result<u64, Error> {
        self.count_incoming(to, edge_kind)
    }
}

impl GraphRead for WriteScope<'_, '_> {
    fn scope_resolve(&self, kind: NodeKind, business_id: &str) -> Result<Option<NodeId>, Error> {
        self.resolve(kind, business_id)
    }

    fn scope_find_by_key(&self, key: &str, value: &str) -> Result<Option<NodeId>, Error> {
        self.find_by_key(key, value)
    }

    fn scope_read_node<T: Row>(&self, id: NodeId) -> Result<Option<T>, Error> {
        self.read_node::<T>(id)
    }

    fn scope_read_nodes<T: Row>(&self, ids: &[NodeId]) -> Result<Vec<T>, Error> {
        self.read_nodes::<T>(ids)
    }

    fn scope_outgoing(&self, from: NodeId, edge_kind: EdgeKind) -> Result<Vec<NodeId>, Error> {
        self.outgoing(from, edge_kind)
    }

    fn scope_incoming(&self, to: NodeId, edge_kind: EdgeKind) -> Result<Vec<NodeId>, Error> {
        self.incoming(to, edge_kind)
    }

    fn scope_count_incoming(&self, to: NodeId, edge_kind: EdgeKind) -> Result<u64, Error> {
        self.count_incoming(to, edge_kind)
    }
}
