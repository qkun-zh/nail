use std::sync::Arc;
use std::time::{Duration, Instant};

use moka::notification::RemovalCause;
use moka::policy::EvictionPolicy;
use moka::sync::Cache;

pub fn token_key(token: &str) -> anyhow::Result<String> {
    nail_common::hash::token(token)
}

pub trait CacheEntry: Clone + Send + Sync + 'static {
    fn reverse_key(&self) -> Option<&str> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReverseMember {
    key: String,
    expires_at: Instant,
}

#[derive(Clone)]
pub struct TokenCache<E: CacheEntry> {
    main: Cache<String, E>,
    reverse: Cache<String, Vec<ReverseMember>>,
    ttl: Duration,
}

impl<E: CacheEntry> TokenCache<E> {
    pub fn new(ttl: Duration, capacity: u64) -> Self {
        let reverse = build_reverse_cache(capacity);
        let main = Cache::builder()
            .max_capacity(capacity)
            .time_to_live(ttl)
            .eviction_policy(EvictionPolicy::lru())
            .eviction_listener({
                let reverse = reverse.clone();
                move |key: Arc<String>, entry: E, _cause: RemovalCause| {
                    if let Some(reverse_key) = entry.reverse_key() {
                        reverse_remove(&reverse, reverse_key, &key);
                    }
                }
            })
            .build();
        Self {
            main,
            reverse,
            ttl,
        }
    }

    pub fn insert(&self, key: &str, entry: E) {
        let key = key.to_string();
        if let Some(reverse_key) = entry.reverse_key() {
            reverse_add(&self.reverse, reverse_key, &key, Instant::now() + self.ttl);
        }
        self.main.insert(key, entry);
    }

    pub fn consume(&self, key: &str) -> Option<E> {
        let key = key.to_string();
        let result = self
            .main
            .entry(key.clone())
            .and_compute_with(|maybe_entry| match maybe_entry {
                Some(_) => moka::ops::compute::Op::Remove,
                None => moka::ops::compute::Op::Nop,
            });
        let moka::ops::compute::CompResult::Removed(entry) = result else {
            return None;
        };
        let entry = entry.into_value();
        if let Some(reverse_key) = entry.reverse_key() {
            reverse_remove(&self.reverse, reverse_key, &key);
        }
        Some(entry)
    }

    pub fn consume_if(&self, key: &str, matches: impl FnOnce(&E) -> bool) -> Option<E> {
        let key = key.to_string();
        let result = self
            .main
            .entry(key.clone())
            .and_compute_with(|maybe_entry| match maybe_entry {
                Some(entry) if matches(entry.value()) => moka::ops::compute::Op::Remove,
                Some(_) => moka::ops::compute::Op::Nop,
                None => moka::ops::compute::Op::Nop,
            });
        let moka::ops::compute::CompResult::Removed(entry) = result else {
            return None;
        };
        let entry = entry.into_value();
        if let Some(reverse_key) = entry.reverse_key() {
            reverse_remove(&self.reverse, reverse_key, &key);
        }
        Some(entry)
    }

    pub fn read(&self, key: &str) -> Option<E> {
        self.main.get(key)
    }

    pub fn delete(&self, key: &str) -> Option<E> {
        let removed = self.main.remove(key);
        if let Some(entry) = &removed
            && let Some(reverse_key) = entry.reverse_key()
        {
            reverse_remove(&self.reverse, reverse_key, key);
        }
        removed
    }

    pub fn delete_by_reverse_key(&self, reverse_key: &str) -> u64 {
        let Some(members) = self.reverse.get(reverse_key) else {
            return 0;
        };
        let count = members.len() as u64;
        self.reverse.invalidate(reverse_key);
        for member in &members {
            self.main.invalidate(&member.key);
        }
        count
    }

    pub fn run_pending_tasks(&self) {
        self.main.run_pending_tasks();
    }
}

#[derive(Debug, Clone)]
pub struct CreateUserTokenEntry {
    pub email_address_hash: String,
    pub email_subject: String,
}

impl CacheEntry for CreateUserTokenEntry {
    fn reverse_key(&self) -> Option<&str> {
        Some(&self.email_address_hash)
    }
}

#[derive(Debug, Clone)]
pub struct SessionTokenEntry {
    pub user_id: String,
}

impl CacheEntry for SessionTokenEntry {
    fn reverse_key(&self) -> Option<&str> {
        Some(&self.user_id)
    }
}

#[derive(Debug, Clone)]
pub struct ChallengeEntry;

impl CacheEntry for ChallengeEntry {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailUpdateTokenEntry {
    pub old_email_address_hash: String,
    pub new_email_address_hash: String,
    pub token_from_old_email_hash: String,
    pub token_from_new_email_hash: String,
}

impl CacheEntry for EmailUpdateTokenEntry {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteUserTokenEntry {
    pub user_id: String,
    pub email_address_hash: String,
}

impl CacheEntry for DeleteUserTokenEntry {
    fn reverse_key(&self) -> Option<&str> {
        Some(&self.user_id)
    }
}

#[derive(Clone)]
pub struct TokenCaches {
    pub create_user: TokenCache<CreateUserTokenEntry>,
    pub session: TokenCache<SessionTokenEntry>,
    pub email_update: TokenCache<EmailUpdateTokenEntry>,
    pub delete_user: TokenCache<DeleteUserTokenEntry>,
    pub challenge: TokenCache<ChallengeEntry>,
}

impl TokenCaches {
    pub fn new(
        token_ttl: Duration,
        session_ttl: Duration,
        challenge_ttl: Duration,
        capacity: u64,
    ) -> Self {
        Self {
            create_user: TokenCache::new(token_ttl, capacity),
            session: TokenCache::new(session_ttl, capacity),
            email_update: TokenCache::new(token_ttl, capacity),
            delete_user: TokenCache::new(token_ttl, capacity),
            challenge: TokenCache::new(challenge_ttl, capacity),
        }
    }
}

fn build_reverse_cache(capacity: u64) -> Cache<String, Vec<ReverseMember>> {
    Cache::builder()
        .max_capacity(capacity)
        .eviction_policy(EvictionPolicy::lru())
        .build()
}

fn reverse_add(
    cache: &Cache<String, Vec<ReverseMember>>,
    key: &str,
    member_key: &str,
    expires_at: Instant,
) {
    cache
        .entry(key.to_string())
        .and_compute_with(|maybe_entry| {
            let mut members = maybe_entry.map(|entry| entry.into_value()).unwrap_or_default();
            let member = ReverseMember {
                key: member_key.to_string(),
                expires_at,
            };
            let index = members.partition_point(|member| member.expires_at <= expires_at);
            members.insert(index, member);
            moka::ops::compute::Op::Put(members)
        });
}

fn reverse_remove(cache: &Cache<String, Vec<ReverseMember>>, key: &str, member_key: &str) {
    cache
        .entry(key.to_string())
        .and_compute_with(|maybe_entry| match maybe_entry {
            Some(entry) => {
                let mut members = entry.into_value();
                members.retain(|member| member.key != member_key);
                if members.is_empty() {
                    moka::ops::compute::Op::Remove
                } else {
                    moka::ops::compute::Op::Put(members)
                }
            }
            None => moka::ops::compute::Op::Nop,
        });
}
