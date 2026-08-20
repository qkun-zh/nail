mod cache;
mod config;
mod value;

pub use cache::{Cache, Caches};
pub use config::CacheConfig;
pub use value::{
    CacheError, CacheValue, Challenge, ChallengeId, Hash, OldAndNewEmailAddressAndTokenHashes,
    UserId, UserIdAndEmailAddressHash, VersionId, VersionIdAndUserId,
};

#[cfg(test)]
#[path = "../../../test/unit/cache/tests.rs"]
mod tests;
