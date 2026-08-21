use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use moka::notification::RemovalCause;
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
    reverse_index: MokaCache<String, Vec<String>>,
}

impl<E: CacheValue> Table<E> {
    /// Constructs a cache that expires entries after `ttl` and holds at most
    /// `capacity` entries.
    #[must_use]
    pub fn new(ttl: Duration, capacity: u64) -> Self {
        let reverse_index = build_reverse_cache(capacity);
        let entries = MokaCache::builder()
            .max_capacity(capacity)
            .time_to_live(ttl)
            .eviction_policy(EvictionPolicy::lru())
            .eviction_listener({
                let reverse_index = reverse_index.clone();
                move |key: Arc<String>, entry: E, _cause: RemovalCause| {
                    if let Some(reverse_key) = entry.reverse_key() {
                        reverse_remove(&reverse_index, reverse_key, &key);
                    }
                }
            })
            .build();
        Self {
            entries,
            reverse_index,
        }
    }

    /// Stores `value` under `key`, registering it under its reverse key.
    pub fn insert(&self, key: &str, value: E) {
        let key = key.to_string();
        if let Some(reverse_key) = value.reverse_key() {
            reverse_add(&self.reverse_index, reverse_key, &key);
        }
        self.entries.insert(key, value);
    }

    /// Returns the value stored under `key`, if any.
    #[must_use]
    pub fn read(&self, key: &str) -> Option<E> {
        self.entries.get(key)
    }

    /// Removes and returns the value stored under `key`, if any.
    #[must_use]
    pub fn delete(&self, key: &str) -> Option<E> {
        let removed = self.entries.remove(key);
        if let Some(entry) = &removed
            && let Some(reverse_key) = entry.reverse_key()
        {
            reverse_remove(&self.reverse_index, reverse_key, key);
        }
        removed
    }

    /// Removes and returns the value stored under `key` when `matches` accepts it.
    #[must_use]
    pub fn delete_if(&self, key: &str, matches: impl FnOnce(&E) -> bool) -> Option<E> {
        let key = key.to_string();
        let result = self
            .entries
            .entry(key.clone())
            .and_compute_with(|maybe_entry| match maybe_entry {
                Some(entry) if matches(entry.value()) => moka::ops::compute::Op::Remove,
                Some(_) | None => moka::ops::compute::Op::Nop,
            });
        let moka::ops::compute::CompResult::Removed(entry) = result else {
            return None;
        };
        let entry = entry.into_value();
        if let Some(reverse_key) = entry.reverse_key() {
            reverse_remove(&self.reverse_index, reverse_key, &key);
        }
        Some(entry)
    }

    /// Removes every entry registered under `reverse_key` and returns how many
    /// were removed.
    #[must_use]
    pub fn delete_by_reverse_key(&self, reverse_key: &str) -> u64 {
        let Some(members) = self.reverse_index.get(reverse_key) else {
            return 0;
        };
        let count = members.len() as u64;
        self.reverse_index.invalidate(reverse_key);
        for member in &members {
            self.entries.invalidate(member);
        }
        count
    }
}

fn build_reverse_cache(capacity: u64) -> MokaCache<String, Vec<String>> {
    MokaCache::builder()
        .max_capacity(capacity)
        .eviction_policy(EvictionPolicy::lru())
        .build()
}

fn reverse_add(cache: &MokaCache<String, Vec<String>>, key: &str, member_key: &str) {
    cache
        .entry(key.to_string())
        .and_compute_with(|maybe_entry| {
            let mut members = maybe_entry.map(moka::Entry::into_value).unwrap_or_default();
            members.push(member_key.to_string());
            moka::ops::compute::Op::Put(members)
        });
}

fn reverse_remove(cache: &MokaCache<String, Vec<String>>, key: &str, member_key: &str) {
    cache
        .entry(key.to_string())
        .and_compute_with(|maybe_entry| match maybe_entry {
            Some(entry) => {
                let mut members = entry.into_value();
                members.retain(|member| member != member_key);
                if members.is_empty() {
                    moka::ops::compute::Op::Remove
                } else {
                    moka::ops::compute::Op::Put(members)
                }
            }
            None => moka::ops::compute::Op::Nop,
        });
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
    /// Constructs the six tables from a loaded configuration.
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

    /// Loads the cache configuration file and builds the six tables.
    ///
    /// # Errors
    ///
    /// Returns an error if the config file cannot be read, parsed, or validated.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Ok(Self::new(&CacheConfig::load(path)?))
    }
}
