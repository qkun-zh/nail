
pub mod entity_store;
pub mod gate;

pub const POLICY: &str = include_str!("authorization/policy.cedar");

pub const SCHEMA: &str = include_str!("authorization/schema.cedar");
