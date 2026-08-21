//! Dedicated full-text search crate for nail, wrapping the seekstorm engine.
//!
//! Invariants:
//! - The engine is never exposed; all reads and writes go through this API.
//! - One article is the unit of replacement: `replace_article` swaps the
//!   complete document set of one article atomically (delete + index + commit).
//! - Every write batch ends in exactly one commit; a commit is the only
//!   happens-before edge that makes documents visible to readers.
//! - Schema is versioned on disk (`nail_schema_version`); a mismatch or a
//!   corrupt index directory is healed by a transparent rebuild, never by a
//!   panic.
//! - Errors are typed ([`Error`]); no `unwrap`, no `expect`, no new panics.

pub mod doc;
pub mod error;
pub(crate) mod field;
pub mod index;
pub mod outcome;
pub mod read;
pub(crate) mod schema;

#[cfg(test)]
#[path = "../../../test/unit/searcher/harness.rs"]
mod searcher_tests;

pub use doc::{CommentDoc, IndexDoc, VersionDoc};
pub use error::Error;
pub use field::SearchField;
pub use index::{SearchIndex, Stats};
pub use outcome::{CommentHit, DocHit, FieldHit, SearchOutcome, VersionHit};
pub use read::SearchRequest;
