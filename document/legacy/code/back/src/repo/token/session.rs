use std::time::Instant;

use crate::repo::token::{TokenCaches, reverse_add, reverse_remove};
use crate::repo::types::SessionTokenEntry;

fn key(token: &str) -> String {
    common::hash::token(token)
}

pub fn create_session_token(caches: &TokenCaches, token: &str, user_id: &str) {
    let key = key(token);
    let now = Instant::now();
    let ttl = caches
        .session
        .policy()
        .time_to_live()
        .expect("session cache must be TTL-bound");
    reverse_add(&caches.session_by_user, user_id, &key, now + ttl);
    caches.session.insert(
        key,
        SessionTokenEntry {
            user_id: user_id.to_string(),
        },
    );
}

pub fn find_user_id_by_session_token(caches: &TokenCaches, token: &str) -> Option<String> {
    caches.session.get(&key(token)).map(|entry| entry.user_id)
}

pub fn delete_session_token(caches: &TokenCaches, token: &str) {
    let key = key(token);
    let Some(entry) = caches.session.remove(&key) else {
        return;
    };
    reverse_remove(&caches.session_by_user, &entry.user_id, &key);
}

pub fn delete_session_tokens_by_user_id(caches: &TokenCaches, user_id: &str) -> u64 {
    let Some(members) = caches.session_by_user.get(user_id) else {
        return 0;
    };
    let count = members.len() as u64;
    caches.session_by_user.invalidate(user_id);
    for member in &members {
        caches.session.invalidate(&member.token);
    }
    count
}
