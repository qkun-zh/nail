
pub mod authenticate;
pub mod challenge;
pub mod deregister;
pub mod download;
pub mod email_update;
pub mod session;

use std::sync::Arc;
use std::time::{Duration, Instant};

use moka::notification::RemovalCause;
use moka::policy::EvictionPolicy;
use moka::sync::Cache;

use crate::repo::types::{
    AuthenticateTokenEntry, DeregisterTokenEntry, DownloadTokenEntry, EmailUpdateTokenEntry,
    SessionTokenEntry,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReverseMember {
    pub(crate) token: String,
    pub(crate) expires_at: Instant,
}

#[derive(Clone)]
pub struct TokenCaches {
    pub authenticate: Cache<String, AuthenticateTokenEntry>,
    pub session: Cache<String, SessionTokenEntry>,
    pub email_update: Cache<String, EmailUpdateTokenEntry>,
    pub deregister: Cache<String, DeregisterTokenEntry>,
    pub download: Cache<String, DownloadTokenEntry>,
    pub challenge: Cache<String, ()>,
    pub authenticate_by_email_hash: Cache<String, Vec<ReverseMember>>,
    pub session_by_user: Cache<String, Vec<ReverseMember>>,
    pub deregister_by_user: Cache<String, Vec<ReverseMember>>,
}

impl TokenCaches {
    pub fn new(
        token_ttl: Duration,
        session_ttl: Duration,
        download_token_ttl: Duration,
        challenge_ttl: Duration,
        capacity: u64,
    ) -> Self {
        let authenticate_by_email_hash = build_reverse_cache(capacity);
        let session_by_user = build_reverse_cache(capacity);
        let deregister_by_user = build_reverse_cache(capacity);

        Self {
            authenticate: build_main_cache(token_ttl, capacity, {
                let reverse = authenticate_by_email_hash.clone();
                move |key: Arc<String>, entry: AuthenticateTokenEntry, _cause: RemovalCause| {
                    reverse_remove(&reverse, &entry.email_address_hash, &key);
                }
            }),
            session: build_main_cache(session_ttl, capacity, {
                let reverse = session_by_user.clone();
                move |key: Arc<String>, entry: SessionTokenEntry, _cause: RemovalCause| {
                    reverse_remove(&reverse, &entry.user_id, &key);
                }
            }),
            email_update: build_main_cache(token_ttl, capacity, |_, _, _| {}),
            deregister: build_main_cache(token_ttl, capacity, {
                let reverse = deregister_by_user.clone();
                move |key: Arc<String>, entry: DeregisterTokenEntry, _cause: RemovalCause| {
                    reverse_remove(&reverse, &entry.user_id, &key);
                }
            }),
            download: build_main_cache(download_token_ttl, capacity, |_, _, _| {}),
            challenge: build_main_cache(challenge_ttl, capacity, |_, _, _| {}),
            authenticate_by_email_hash,
            session_by_user,
            deregister_by_user,
        }
    }
}

fn build_main_cache<V: Clone + Send + Sync + 'static>(
    ttl: Duration,
    capacity: u64,
    listener: impl Fn(Arc<String>, V, RemovalCause) + Send + Sync + 'static,
) -> Cache<String, V> {
    Cache::builder()
        .max_capacity(capacity)
        .time_to_live(ttl)
        .eviction_policy(EvictionPolicy::lru())
        .eviction_listener(listener)
        .build()
}

fn build_reverse_cache(capacity: u64) -> Cache<String, Vec<ReverseMember>> {
    Cache::builder()
        .max_capacity(capacity)
        .eviction_policy(EvictionPolicy::lru())
        .build()
}

pub(crate) fn reverse_add(
    cache: &Cache<String, Vec<ReverseMember>>,
    key: &str,
    token: &str,
    expires_at: Instant,
) {
    cache
        .entry(key.to_string())
        .and_compute_with(|maybe_entry| {
            let mut members = maybe_entry.map(|e| e.into_value()).unwrap_or_default();
            let member = ReverseMember {
                token: token.to_string(),
                expires_at,
            };
            let idx = members.partition_point(|m| m.expires_at <= expires_at);
            members.insert(idx, member);
            moka::ops::compute::Op::Put(members)
        });
}

pub(crate) fn reverse_remove(cache: &Cache<String, Vec<ReverseMember>>, key: &str, token: &str) {
    cache
        .entry(key.to_string())
        .and_compute_with(|maybe_entry| match maybe_entry {
            Some(e) => {
                let mut members = e.into_value();
                members.retain(|m| m.token != token);
                if members.is_empty() {
                    moka::ops::compute::Op::Remove
                } else {
                    moka::ops::compute::Op::Put(members)
                }
            }
            None => moka::ops::compute::Op::Nop,
        });
}
