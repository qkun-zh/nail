//! - Schema is versioned on disk (`nail_schema_version`); a mismatch or a
pub mod doc;
pub mod error;
pub(crate) mod field;
pub mod outcome;
pub mod read;
pub(crate) mod schema;
pub mod searcher;

#[cfg(test)]
#[path = "tests/harness.rs"]
mod searcher_tests;

pub use doc::{CommentDoc, SearchDoc, VersionDoc};
pub use error::Error;
pub use field::SearchField;
pub use outcome::{CommentHit, DocHit, FieldHit, SearchOutcome, VersionHit};
pub use read::SearchRequest;
pub use searcher::{DEFAULT_SEGMENT_NUMBER_BITS, Searcher};
