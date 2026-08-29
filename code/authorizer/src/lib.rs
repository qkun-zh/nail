//! Standalone authorization crate wrapping Cedar, mirroring `searcher` design.
//! - Cedar engine is never exposed; all authorization goes through this API.
//! - The policy set has two halves: handwritten static policies plus, for every
//!   schema action, one template linked once per durable [`Grant`]. Grants are
//!   projected from the database at startup and re-projected via
//!   [`Authorizer::reload`] whenever an administrator grants or revokes.
//! - Policies, requests and entities are all strictly validated against the
//!   schema; malformed requests surface as [`Error::InvalidRequest`].
//! - `Authorizer` is cheaply cloneable (`Arc` inside) and `Send`/`Sync`.

pub mod authorizer;
pub mod error;
pub mod principal;
pub mod request_context;
pub mod resource;

include!(concat!(env!("OUT_DIR"), "/permissions.rs"));
include!(concat!(env!("OUT_DIR"), "/all_permissions.rs"));
include!(concat!(env!("OUT_DIR"), "/cedar_entities.rs"));

#[cfg(test)]
#[path = "tests/harness.rs"]
mod authorizer_tests;

pub use authorizer::{Authorizer, Grant};
pub use error::Error;
pub use principal::Principal;
pub use request_context::RequestContext;
pub use resource::Resource;
