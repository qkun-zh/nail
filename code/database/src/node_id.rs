use std::fmt;

use agdb::DbId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(i64);

impl NodeId {
    pub(crate) fn from_db(id: DbId) -> Self {
        Self(id.0)
    }

    pub(crate) fn to_db(self) -> DbId {
        DbId(self.0)
    }

    #[cfg(test)]
    pub(crate) fn from_raw(raw: u64) -> Self {
        Self(raw.cast_signed())
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "node({})", self.0)
    }
}
