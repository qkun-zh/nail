use std::time::Duration;

use uuid::Uuid;

use crate::{
    Cache, CacheConfig, CacheError, CacheValue, Challenge, ChallengeId, Hash,
    OldAndNewEmailAddressAndTokenHashes, Table, UserId, UserIdAndEmailAddressHash, VersionId,
    VersionIdAndUserId,
};

fn uuid_v7() -> String {
    Uuid::now_v7().to_string()
}

fn hash_32() -> String {
    "a".repeat(32)
}

fn session_cache() -> Table<UserId> {
    Table::new(Duration::from_mins(1), 100)
}

fn email_update_entry() -> OldAndNewEmailAddressAndTokenHashes {
    OldAndNewEmailAddressAndTokenHashes {
        old_email_address_hash: Hash::new(hash_32()).expect("hash"),
        new_email_address_hash: Hash::new("b".repeat(32)).expect("hash"),
        old_email_token_hash: Hash::new("c".repeat(32)).expect("hash"),
        new_email_token_hash: Hash::new("d".repeat(32)).expect("hash"),
    }
}

#[test]
fn hash_new_accepts_32_hex_chars() {
    let hash = Hash::new(hash_32()).expect("hash");
    assert_eq!(hash.as_str(), hash_32());
    let upper = Hash::new("A".repeat(32)).expect("upper hex");
    assert_eq!(upper.as_str(), "A".repeat(32));
}

#[test]
fn hash_new_rejects_wrong_shapes() {
    assert_eq!(Hash::new("a".repeat(31)), Err(CacheError::InvalidHash));
    assert_eq!(Hash::new("a".repeat(33)), Err(CacheError::InvalidHash));
    assert_eq!(Hash::new("z".repeat(32)), Err(CacheError::InvalidHash));
    assert_eq!(Hash::new(String::new()), Err(CacheError::InvalidHash));
}

#[test]
fn uuid_newtypes_accept_a_uuid_v7() {
    let id = uuid_v7();
    assert_eq!(UserId::new(id.clone()).expect("user id").as_str(), id);
    assert_eq!(VersionId::new(id.clone()).expect("version id").as_str(), id);
    assert_eq!(
        ChallengeId::new(id.clone()).expect("challenge id").as_str(),
        id
    );
}

#[test]
fn uuid_newtypes_reject_other_versions_and_garbage() {
    assert_eq!(
        UserId::new(Uuid::new_v4().to_string()),
        Err(CacheError::InvalidId)
    );
    assert_eq!(
        VersionId::new("not-a-uuid".to_string()),
        Err(CacheError::InvalidId)
    );
    assert_eq!(ChallengeId::new(String::new()), Err(CacheError::InvalidId));
}

#[test]
fn insert_read_and_delete_round_trip() {
    let cache: Table<UserId> = session_cache();
    let key = uuid_v7();
    let user_id = UserId::new(uuid_v7()).expect("user id");
    cache.insert(&key, user_id.clone());
    assert_eq!(cache.read(&key), Some(user_id.clone()));
    assert_eq!(cache.delete(&key), Some(user_id));
    assert_eq!(cache.read(&key), None);
}

#[test]
fn delete_removes_the_reverse_member() {
    let cache: Table<UserId> = session_cache();
    let key = uuid_v7();
    let user_id = UserId::new(uuid_v7()).expect("user id");
    let reverse_key = user_id.as_str().to_string();
    cache.insert(&key, user_id);
    assert!(cache.delete(&key).is_some());
    assert_eq!(cache.delete_by_reverse_key(&reverse_key), 0);
}

#[test]
fn delete_by_reverse_key_removes_every_entry_of_an_entity() {
    let cache: Table<UserId> = session_cache();
    let first = uuid_v7();
    let second = uuid_v7();
    let user_id = UserId::new(uuid_v7()).expect("user id");
    let reverse_key = user_id.as_str().to_string();
    cache.insert(&first, user_id.clone());
    cache.insert(&second, user_id);
    assert_eq!(cache.delete_by_reverse_key(&reverse_key), 2);
    assert_eq!(cache.read(&first), None);
    assert_eq!(cache.read(&second), None);
}

#[test]
fn delete_if_removes_the_entry_only_when_the_predicate_matches() {
    let cache: Table<OldAndNewEmailAddressAndTokenHashes> = Table::new(Duration::from_mins(1), 100);
    let entry = email_update_entry();
    cache.insert("user-1", entry.clone());

    let mismatched = cache.delete_if("user-1", |current| {
        current.old_email_token_hash.as_str() == "wrong"
    });
    assert!(mismatched.is_none());
    assert_eq!(cache.read("user-1"), Some(entry.clone()));

    let matched = cache.delete_if("user-1", |current| {
        current.old_email_token_hash.as_str() == "c".repeat(32)
    });
    assert_eq!(matched, Some(entry));
    assert_eq!(cache.read("user-1"), None);
}

#[test]
fn delete_if_removes_the_reverse_member() {
    let cache: Table<UserIdAndEmailAddressHash> = Table::new(Duration::from_mins(1), 100);
    let key = uuid_v7();
    let user_id = UserId::new(uuid_v7()).expect("user id");
    let reverse_key = user_id.as_str().to_string();
    cache.insert(
        &key,
        UserIdAndEmailAddressHash {
            user_id,
            email_address_hash: Hash::new(hash_32()).expect("hash"),
        },
    );
    let removed = cache
        .delete_if(&key, |current| current.user_id.as_str() == reverse_key)
        .expect("removed");
    assert_eq!(removed.email_address_hash.as_str(), hash_32());
    assert_eq!(cache.delete_by_reverse_key(&reverse_key), 0);
}

#[test]
fn reverse_keys_are_wired_to_the_right_fields() {
    let hash = Hash::new(hash_32()).expect("hash");
    assert_eq!(hash.reverse_key(), Some(hash.as_str()));
    let user_id = UserId::new(uuid_v7()).expect("user id");
    assert_eq!(user_id.reverse_key(), Some(user_id.as_str()));
    let deletion = UserIdAndEmailAddressHash {
        user_id: user_id.clone(),
        email_address_hash: hash,
    };
    assert_eq!(deletion.reverse_key(), Some(user_id.as_str()));
    let email_update = email_update_entry();
    assert_eq!(email_update.reverse_key(), None);
    let download = VersionIdAndUserId {
        version_id: VersionId::new(uuid_v7()).expect("version id"),
        user_id: UserId::new(uuid_v7()).expect("user id"),
    };
    assert_eq!(download.reverse_key(), None);
    assert_eq!(Challenge.reverse_key(), None);
}

#[test]
fn caches_hold_the_six_tables() {
    let config = CacheConfig {
        user_creation_ttl_seconds: 8000,
        session_ttl_seconds: 8000,
        email_update_ttl_seconds: 8000,
        user_deletion_ttl_seconds: 8000,
        challenge_ttl_seconds: 300,
        download_ttl_seconds: 60,
        cache_capacity: 100,
    };
    let caches = Cache::new(&config);

    let creation_key = uuid_v7();
    let creation_hash = Hash::new(hash_32()).expect("hash");
    caches
        .user_creation
        .insert(&creation_key, creation_hash.clone());
    assert_eq!(
        caches.user_creation.read(&creation_key),
        Some(creation_hash)
    );

    let session_key = uuid_v7();
    let user_id = UserId::new(uuid_v7()).expect("user id");
    caches.session.insert(&session_key, user_id.clone());
    assert_eq!(caches.session.read(&session_key), Some(user_id));

    let email_update = email_update_entry();
    caches.email_update.insert("user-1", email_update.clone());
    assert_eq!(caches.email_update.read("user-1"), Some(email_update));

    let deletion_key = uuid_v7();
    let deletion = UserIdAndEmailAddressHash {
        user_id: UserId::new(uuid_v7()).expect("user id"),
        email_address_hash: Hash::new(hash_32()).expect("hash"),
    };
    caches.user_deletion.insert(&deletion_key, deletion.clone());
    assert_eq!(caches.user_deletion.read(&deletion_key), Some(deletion));

    let challenge_id = uuid_v7();
    caches.challenge.insert(&challenge_id, Challenge);
    assert!(caches.challenge.delete(&challenge_id).is_some());

    let download_key = uuid_v7();
    let download = VersionIdAndUserId {
        version_id: VersionId::new(uuid_v7()).expect("version id"),
        user_id: UserId::new(uuid_v7()).expect("user id"),
    };
    caches.download.insert(&download_key, download.clone());
    assert_eq!(caches.download.read(&download_key), Some(download));
}

#[test]
fn config_defaults_apply_to_missing_fields() {
    let config: CacheConfig = toml::from_str("").expect("empty toml");
    assert_eq!(config.user_creation_ttl_seconds, 8000);
    assert_eq!(config.session_ttl_seconds, 8000);
    assert_eq!(config.email_update_ttl_seconds, 8000);
    assert_eq!(config.user_deletion_ttl_seconds, 8000);
    assert_eq!(config.challenge_ttl_seconds, 300);
    assert_eq!(config.download_ttl_seconds, 60);
    assert_eq!(config.cache_capacity, 100_000);
}

#[test]
fn config_validate_rejects_zero_fields() {
    for field in [
        "user_creation_ttl_seconds",
        "session_ttl_seconds",
        "email_update_ttl_seconds",
        "user_deletion_ttl_seconds",
        "challenge_ttl_seconds",
        "download_ttl_seconds",
        "cache_capacity",
    ] {
        let config: CacheConfig = toml::from_str(&format!("{field} = 0")).expect("toml");
        assert!(
            config.validate().is_err(),
            "{field} = 0 must fail validation"
        );
    }
}

#[test]
fn config_load_reads_a_toml_file() {
    let path = std::env::temp_dir().join(format!("nail_cache_config_{}", uuid_v7()));
    std::fs::write(&path, "session_ttl_seconds = 42\ncache_capacity = 7\n").expect("write config");
    let config = CacheConfig::load(&path).expect("load config");
    assert_eq!(config.session_ttl_seconds, 42);
    assert_eq!(config.cache_capacity, 7);
    assert_eq!(config.user_creation_ttl_seconds, 8000);
    std::fs::remove_file(path).expect("remove config");
}

#[test]
fn config_load_rejects_a_missing_file() {
    assert!(CacheConfig::load("/nonexistent/nail-cache.toml").is_err());
}
