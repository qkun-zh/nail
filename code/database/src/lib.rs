//! Graph persistence substrate for nail. Owns all agdb access behind a
//! closure-scoped, agdb-free API: `Database::read`/`Database::write` scopes,
//! opaque [`NodeId`] handles, typed [`Row`]s, and explicit write primitives.

mod condition;
mod database;
mod error;
mod kinds;
mod node_id;
mod read;
mod row;
mod scope;
mod value;
mod write;

pub use condition::{Condition, Order};
pub use database::Database;
pub use error::Error;
pub use kinds::{EdgeKind, ID_KEY, NodeKind};
pub use node_id::NodeId;
pub use row::{Row, ValueLookup};
pub use scope::ReadScope;
pub use value::Value;
pub use write::WriteScope;

#[cfg(test)]
#[path = "../../../test/unit/database/tests.rs"]
mod tests;
