use std::time::Duration;

use crate::repository::cache::{
    CacheEntry, ChallengeEntry, CreateUserTokenEntry, DeleteUserTokenEntry, DownloadTokenEntry,
    EmailUpdateTokenEntry, SessionTokenEntry, TokenCache, token_key,
};

fn cache<E: crate::repository::cache::CacheEntry>() -> TokenCache<E> {
    TokenCache::new(Duration::from_mins(1), 100)
}

#[test]
fn challenge_cache_consumes_a_key_once() {
    let cache: TokenCache<ChallengeEntry> = cache();
    cache.insert("challenge-id", ChallengeEntry);
    assert!(cache.consume("challenge-id").is_some());
    assert!(cache.consume("challenge-id").is_none());
}

#[test]
fn insert_read_and_delete_round_trip() {
    let cache: TokenCache<SessionTokenEntry> = cache();
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
    let cache: TokenCache<SessionTokenEntry> = cache();
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
    let cache: TokenCache<SessionTokenEntry> = cache();
    let key = token_key("token").expect("token key");
    cache.insert(
        &key,
        SessionTokenEntry {
            user_id: "user-1".to_string(),
        },
    );
    assert!(cache.consume(&key).is_some());
    assert_eq!(cache.delete_by_reverse_key("user-1"), 0);
}

#[test]
fn consume_if_removes_the_entry_only_when_the_predicate_matches() {
    let cache: TokenCache<EmailUpdateTokenEntry> = cache();
    let entry = EmailUpdateTokenEntry {
        old_email_hash: "old-hash".to_string(),
        new_email_hash: "new-hash".to_string(),
        token_hash_from_old_email: "old-token-hash".to_string(),
        token_hash_from_new_email: "new-token-hash".to_string(),
    };
    cache.insert("user-1", entry.clone());

    let mismatched = cache.consume_if("user-1", |current| current.token_hash_from_old_email == "wrong");
    assert!(mismatched.is_none());
    assert_eq!(cache.read("user-1"), Some(entry.clone()));

    let matched = cache.consume_if("user-1", |current| {
        current.token_hash_from_old_email == "old-token-hash"
    });
    assert_eq!(matched, Some(entry));
    assert!(cache.read("user-1").is_none());
}

#[test]
fn entry_reverse_keys_are_wired_to_the_right_fields() {
    let create_user = CreateUserTokenEntry {
        email_address_hash: "hash-a".to_string(),
    };
    assert_eq!(create_user.reverse_key(), Some("hash-a"));

    let delete_user = DeleteUserTokenEntry {
        user_id: "user-1".to_string(),
        email_address_hash: "hash".to_string(),
    };
    assert_eq!(delete_user.reverse_key(), Some("user-1"));

    let email_update = EmailUpdateTokenEntry {
        old_email_hash: "old".to_string(),
        new_email_hash: "new".to_string(),
        token_hash_from_old_email: "old-token".to_string(),
        token_hash_from_new_email: "new-token".to_string(),
    };
    assert_eq!(email_update.reverse_key(), None);
}

#[test]
fn delete_user_cache_supports_reverse_deletion_by_user() {
    let cache: TokenCache<DeleteUserTokenEntry> = cache();
    let key = token_key("delete-user-token").expect("token key");
    cache.insert(
        &key,
        DeleteUserTokenEntry {
            user_id: "user-1".to_string(),
            email_address_hash: "hash".to_string(),
        },
    );
    assert_eq!(cache.read(&key).expect("entry").user_id, "user-1");
    assert_eq!(cache.delete_by_reverse_key("user-1"), 1);
    assert!(cache.read(&key).is_none());
}

#[test]
fn token_key_is_the_ascon_hash_of_the_token() {
    let key = token_key("token").expect("token key");
    assert_eq!(key.len(), 64);
    assert_ne!(key, "token");
    assert_eq!(key, token_key("token").expect("token key"));
}

#[test]
fn download_token_is_single_use() {
    let cache: TokenCache<DownloadTokenEntry> = cache();
    let key = token_key("download-token").expect("token key");
    cache.insert(
        &key,
        DownloadTokenEntry {
            version_id: "version-1".to_string(),
            user_id: "user-1".to_string(),
        },
    );

    let consumed = cache.consume(&key).expect("first consume");
    assert_eq!(consumed.version_id, "version-1");
    assert_eq!(consumed.user_id, "user-1");
    assert!(cache.consume(&key).is_none());
}

#[test]
fn download_token_consume_if_removes_only_the_matching_user() {
    let cache: TokenCache<DownloadTokenEntry> = cache();
    let key = token_key("download-token").expect("token key");
    cache.insert(
        &key,
        DownloadTokenEntry {
            version_id: "version-1".to_string(),
            user_id: "user-1".to_string(),
        },
    );

    let mismatched = cache.consume_if(&key, |entry| entry.user_id == "someone-else");
    assert!(mismatched.is_none());
    assert_eq!(cache.read(&key).expect("still present").user_id, "user-1");

    let matched = cache.consume_if(&key, |entry| entry.user_id == "user-1");
    assert_eq!(matched.expect("consumed").version_id, "version-1");
    assert!(cache.read(&key).is_none());
}

#[test]
fn download_token_has_no_reverse_key() {
    let entry = DownloadTokenEntry {
        version_id: "version-1".to_string(),
        user_id: "user-1".to_string(),
    };
    assert_eq!(entry.reverse_key(), None);
}
