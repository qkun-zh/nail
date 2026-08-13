use std::time::Duration;

use crate::repository::cache::{
    AuthenticateTokenEntry, CacheEntry, ChallengeEntry, SessionTokenEntry, TokenCache, token_key,
};

#[derive(Debug, Clone)]
struct ReverseEntry(String);

impl CacheEntry for ReverseEntry {
    fn reverse_key(&self) -> Option<&str> {
        Some(&self.0)
    }
}

fn cache<E: CacheEntry>(with_reverse: bool) -> TokenCache<E> {
    TokenCache::new(Duration::from_secs(60), 100, with_reverse)
}

#[test]
fn challenge_cache_consumes_a_key_once() {
    let cache: TokenCache<ChallengeEntry> = cache(false);
    cache.insert("challenge-id", ChallengeEntry);
    assert!(cache.consume("challenge-id").is_some());
    assert!(cache.consume("challenge-id").is_none());
}

#[test]
fn insert_read_and_delete_round_trip() {
    let cache: TokenCache<SessionTokenEntry> = cache(true);
    let key = token_key("token").expect("token key");
    cache.insert(
        &key,
        SessionTokenEntry {
            user_id: "user-1".to_string(),
        },
    );
    assert_eq!(cache.read(&key).expect("entry").user_id, "user-1");
    let removed = cache.delete(&key).expect("removed");
    assert_eq!(removed.user_id, "user-1");
    assert!(cache.read(&key).is_none());
}

#[test]
fn reverse_index_removes_all_tokens_for_a_key() {
    let cache: TokenCache<SessionTokenEntry> = cache(true);
    let first = token_key("token-1").expect("token key");
    let second = token_key("token-2").expect("token key");
    cache.insert(
        &first,
        SessionTokenEntry {
            user_id: "user-1".to_string(),
        },
    );
    cache.insert(
        &second,
        SessionTokenEntry {
            user_id: "user-1".to_string(),
        },
    );
    assert_eq!(cache.delete_by_reverse_key("user-1"), 2);
    assert!(cache.read(&first).is_none());
    assert!(cache.read(&second).is_none());
}

#[test]
fn consume_removes_the_reverse_member() {
    let cache: TokenCache<ReverseEntry> = cache(true);
    let key = token_key("token").expect("token key");
    cache.insert(&key, ReverseEntry("group-a".to_string()));
    assert!(cache.consume(&key).is_some());
    assert_eq!(cache.delete_by_reverse_key("group-a"), 0);
}

#[test]
fn capacity_eviction_removes_the_reverse_member() {
    let cache: TokenCache<ReverseEntry> = TokenCache::new(Duration::from_secs(60), 1, true);
    let first = token_key("token-1").expect("token key");
    let second = token_key("token-2").expect("token key");
    cache.insert(&first, ReverseEntry("group-a".to_string()));
    cache.insert(&second, ReverseEntry("group-b".to_string()));
    cache.run_pending_tasks();
    assert_eq!(cache.delete_by_reverse_key("group-a"), 0);
    assert_eq!(cache.delete_by_reverse_key("group-b"), 1);
}

#[test]
fn authenticate_entry_indexes_by_email_hash() {
    let entry = AuthenticateTokenEntry {
        email_address_hash: "hash-a".to_string(),
        email_subject: "subject".to_string(),
    };
    assert_eq!(entry.reverse_key(), Some("hash-a"));
}

#[test]
fn token_key_is_the_ascon_hash_of_the_token() {
    let key = token_key("token").expect("token key");
    assert_eq!(key.len(), 64);
    assert_ne!(key, "token");
    assert_eq!(key, token_key("token").expect("token key"));
}
