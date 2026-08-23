//! Standalone authorization crate wrapping Cedar, mirroring `searcher` design.
//! - Cedar engine is never exposed; all authorization goes through this API.
//!   [`Authorizer::new`] with strict validation; failure is a typed error, never a panic.
//! - Every request is authorized against an explicit `Principal` snapshot and a
//! - `Authorizer` is cheaply cloneable (`Arc` inside) and `Send`/`Sync`.

pub mod authorizer;
pub mod error;
pub mod principal;
pub mod resource;

include!(concat!(env!("OUT_DIR"), "/permissions.rs"));
include!(concat!(env!("OUT_DIR"), "/all_permissions.rs"));
include!(concat!(env!("OUT_DIR"), "/cedar_entities.rs"));

#[cfg(test)]
#[path = "tests/harness.rs"]
mod authorizer_tests;

pub use authorizer::Authorizer;
pub use error::Error;
pub use principal::{Principal, Role};
pub use resource::Resource;
