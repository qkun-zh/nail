use std::time::Duration;

use anyhow::Result;
use moka::policy::EvictionPolicy;
use moka::sync::Cache as MokaCache;

use crate::config::CacheConfig;
use crate::value::{
    CacheValue, Challenge, Hash, OldAndNewEmailAddressAndTokenHashes, UserId,
    UserIdAndEmailAddressHash, VersionIdAndUserId,
};

#[derive(Clone)]
pub struct Table<E: CacheValue> {
    entries: MokaCache<String, E>,
}

impl<E: CacheValue> Table<E> {
    #[must_use]
    pub fn new(ttl: Duration, capacity: u64) -> Self {
        Self {
            entries: MokaCache::builder()
                .max_capacity(capacity)
                .time_to_live(ttl)
                .eviction_policy(EvictionPolicy::lru())
                .build(),
        }
    }

    pub fn insert(&self, key: &str, value: E) {
        self.entries.insert(key.to_string(), value);
    }

    #[must_use]
    pub fn read(&self, key: &str) -> Option<E> {
        self.entries.get(key)
    }

    #[must_use]
    pub fn delete(&self, key: &str) -> Option<E> {
        self.entries.remove(key)
    }

    #[must_use]
    pub fn delete_if(&self, key: &str, matches: impl FnOnce(&E) -> bool) -> Option<E> {
        let result = self
            .entries
            .entry(key.to_string())
            .and_compute_with(|maybe_entry| match maybe_entry {
                Some(entry) if matches(entry.value()) => moka::ops::compute::Op::Remove,
                Some(_) | None => moka::ops::compute::Op::Nop,
            });
        let moka::ops::compute::CompResult::Removed(entry) = result else {
            return None;
        };
        Some(entry.into_value())
    }

    #[must_use]
    pub fn delete_by_reverse_key(&self, reverse_key: &str) -> u64 {
        let stale: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.reverse_key() == Some(reverse_key))
            .map(|(key, _)| key.as_ref().clone())
            .collect();
        let count = stale.len() as u64;
        for key in &stale {
            self.entries.invalidate(key);
        }
        count
    }

    /// TEMPORARY instrumentation: print every live entry as key -> value.
    /// Removed after observation is complete.
    pub fn dump(&self, name: &str)
    where
        E: std::fmt::Debug,
    {
        let mut keys: Vec<String> = self
            .entries
            .iter()
            .map(|(key, _)| key.as_ref().clone())
            .collect();
        keys.sort();
        eprintln!("[CACHE:{name}] {} entry(ies)", keys.len());
        for key in keys {
            let value = self.entries.get(&key);
            eprintln!("[CACHE:{name}]   key={key} value={value:?}");
        }
    }
}

#[derive(Clone)]
pub struct Cache {
    pub user_creation: Table<Hash>,
    pub session: Table<UserId>,
    pub email_update: Table<OldAndNewEmailAddressAndTokenHashes>,
    pub user_deletion: Table<UserIdAndEmailAddressHash>,
    pub challenge: Table<Challenge>,
    pub download: Table<VersionIdAndUserId>,
}

impl Cache {
    #[must_use]
    pub fn new(config: &CacheConfig) -> Self {
        let capacity = config.cache_capacity;
        Self {
            user_creation: Table::new(
                Duration::from_secs(config.user_creation_ttl_seconds),
                capacity,
            ),
            session: Table::new(Duration::from_secs(config.session_ttl_seconds), capacity),
            email_update: Table::new(
                Duration::from_secs(config.email_update_ttl_seconds),
                capacity,
            ),
            user_deletion: Table::new(
                Duration::from_secs(config.user_deletion_ttl_seconds),
                capacity,
            ),
            challenge: Table::new(Duration::from_secs(config.challenge_ttl_seconds), capacity),
            download: Table::new(Duration::from_secs(config.download_ttl_seconds), capacity),
        }
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Ok(Self::new(&CacheConfig::load(path)?))
    }
}
