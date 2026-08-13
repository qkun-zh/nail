
use std::time::Duration;

use common::hash;
use uuid::Uuid;

use crate::repo::token::TokenCaches;
use crate::repo::token::{authenticate, challenge, deregister, download, email_update, session};
use crate::unit_tests::context::TestCtx;

fn caches(ttl: Duration) -> TokenCaches {
    TokenCaches::new(ttl, ttl, ttl, ttl, 100_000)
}

fn v7() -> String {
    Uuid::now_v7().to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authenticate_token_roundtrip_keeps_reverse_index_consistent() {
    let c = caches(Duration::from_secs(60));
    let token = v7();
    let email_hash = hash::email("alice@qq.com");
    authenticate::create_authenticate_token(&c, &token, &email_hash, "subject");

    let key = hash::token(&token);
    assert_eq!(c.authenticate.get(&key).unwrap().email_subject, "subject");
    assert!(c.authenticate.get(&token).is_none(), "缓存不得存明文 token");
    let candidates: Vec<String> = c
        .authenticate_by_email_hash
        .get(&email_hash)
        .map(|members| members.into_iter().map(|m| m.token).collect())
        .unwrap();
    assert!(candidates.contains(&key), "反向索引必须含 token 哈希");

    let entry = authenticate::consume_authenticate_token(&c, &token).expect("可消费");
    assert_eq!(entry.email_address_hash, email_hash);
    assert!(c.authenticate.get(&key).is_none());
    assert!(
        c.authenticate_by_email_hash.get(&email_hash).is_none(),
        "空集合不得残留"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authenticate_consume_refuses_expired_but_not_evicted_token() {
    let c = caches(Duration::from_millis(80));
    let token = v7();
    let email_hash = hash::email("alice@qq.com");
    authenticate::create_authenticate_token(&c, &token, &email_hash, "s");
    std::thread::sleep(Duration::from_millis(200));
    assert!(authenticate::consume_authenticate_token(&c, &token).is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authenticate_consume_is_atomic_under_concurrency() {
    let c = caches(Duration::from_secs(60));
    let token = v7();
    authenticate::create_authenticate_token(&c, &token, "h", "s");

    let wins = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let c = c.clone();
        let token = token.clone();
        let wins = wins.clone();
        tasks.push(tokio::spawn(async move {
            if authenticate::consume_authenticate_token(&c, &token).is_some() {
                wins.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }
    assert_eq!(
        wins.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "并发消费恰好一个赢家"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_token_find_delete_and_reverse_index() {
    let c = caches(Duration::from_secs(60));
    let user_id = v7();
    let token = v7();
    session::create_session_token(&c, &token, &user_id);
    assert_eq!(
        session::find_user_id_by_session_token(&c, &token).as_deref(),
        Some(user_id.as_str())
    );

    session::delete_session_token(&c, &token);
    assert!(session::find_user_id_by_session_token(&c, &token).is_none());
    assert!(
        c.session_by_user.get(&user_id).is_none(),
        "反向索引必须同步清空"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_session_token_is_idempotent() {
    let c = caches(Duration::from_secs(60));
    let token = v7();
    session::create_session_token(&c, &token, &v7());
    session::delete_session_token(&c, &token);
    session::delete_session_token(&c, &token);
    session::delete_session_token(&c, &token);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_session_tokens_by_user_id_clears_all_sessions() {
    let c = caches(Duration::from_secs(60));
    let user_id = v7();
    let t1 = v7();
    let t2 = v7();
    let t3 = v7();
    for t in [&t1, &t2, &t3] {
        session::create_session_token(&c, t, &user_id);
    }
    assert_eq!(session::delete_session_tokens_by_user_id(&c, &user_id), 3);
    for t in [&t1, &t2, &t3] {
        assert!(session::find_user_id_by_session_token(&c, t).is_none());
    }
    assert!(c.session_by_user.get(&user_id).is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_token_consume_checks_minted_at_time_to_live() {
    let c = caches(Duration::from_secs(60));
    let user_id = v7();
    let token = v7();
    deregister::create_deregister_token(&c, &token, &user_id, "h");
    let entry = deregister::consume_deregister_token(&c, &token).expect("可消费");
    assert_eq!(entry.user_id, user_id);
    assert!(c.deregister.get(&token).is_none());
    assert!(c.deregister_by_user.get(&user_id).is_none());

    let c2 = caches(Duration::from_millis(80));
    let token2 = v7();
    deregister::create_deregister_token(&c2, &token2, &user_id, "h");
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        deregister::consume_deregister_token(&c2, &token2).is_none(),
        "minted_at 超过 TTL 必须拒绝"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_consume_is_atomic_under_concurrency() {
    let c = caches(Duration::from_secs(60));
    let token = v7();
    deregister::create_deregister_token(&c, &token, &v7(), "h");

    let wins = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut tasks = Vec::new();
    for _ in 0..12 {
        let c = c.clone();
        let token = token.clone();
        let wins = wins.clone();
        tasks.push(tokio::spawn(async move {
            if deregister::consume_deregister_token(&c, &token).is_some() {
                wins.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }
    assert_eq!(wins.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn download_token_consume_refuses_expired_by_created_at_age() {
    let c = caches(Duration::from_millis(80));
    let token = v7();
    download::create_download_token(&c, &token, "version-1", "user-1");
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        download::consume_download_token(&c, &token).is_none(),
        "过期（TTL 感知）必须拒绝"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn download_token_find_and_consume_are_single_use() {
    let c = caches(Duration::from_secs(60));
    let token = v7();
    download::create_download_token(&c, &token, "v", "u");
    let found = download::find_download_token(&c, &token).expect("find 必须可见");
    assert_eq!(found.version_id, "v");
    assert_eq!(found.user_id, "u");
    assert!(c.download.get(&hash::token(&token)).is_some());
    let entry = download::consume_download_token(&c, &token).expect("可消费");
    assert_eq!(entry.version_id, "v");
    assert!(
        c.download.get(&hash::token(&token)).is_none(),
        "消费后主缓存必须清空"
    );
    assert!(download::consume_download_token(&c, &token).is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn download_consume_is_atomic_under_concurrency() {
    let c = caches(Duration::from_secs(60));
    let token = v7();
    download::create_download_token(&c, &token, "v", "u");

    let wins = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut tasks = Vec::new();
    for _ in 0..12 {
        let c = c.clone();
        let token = token.clone();
        let wins = wins.clone();
        tasks.push(tokio::spawn(async move {
            if download::consume_download_token(&c, &token).is_some() {
                wins.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }
    assert_eq!(wins.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn email_update_new_send_overwrites_old_row() {
    let c = caches(Duration::from_secs(60));
    let user_id = v7();
    let (old_plain, new_plain) = (v7(), v7());
    let (old_hash, new_hash) = (hash::token(&old_plain), hash::token(&new_plain));
    email_update::create_email_update_token(
        &c, &user_id, "old-hash", "new-hash", &old_hash, &new_hash,
    );
    let (old2, new2) = (v7(), v7());
    let (old2_hash, new2_hash) = (hash::token(&old2), hash::token(&new2));
    email_update::create_email_update_token(
        &c, &user_id, "old-hash", "new-hash", &old2_hash, &new2_hash,
    );
    assert!(
        email_update::consume_email_update_token_if_matches(&c, &user_id, &old_hash, &new_hash,)
            .is_none(),
        "被覆盖的旧 token 对必须失效"
    );
    let entry =
        email_update::consume_email_update_token_if_matches(&c, &user_id, &old2_hash, &new2_hash)
            .expect("最新行可消费");
    assert_eq!(entry.old_email_address_hash, "old-hash");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn email_update_consume_restores_row_on_pair_mismatch() {
    let c = caches(Duration::from_secs(60));
    let user_id = v7();
    let (a, b) = (v7(), v7());
    let (a_hash, b_hash) = (hash::token(&a), hash::token(&b));
    email_update::create_email_update_token(&c, &user_id, "old-hash", "new-hash", &a_hash, &b_hash);
    let (x, y) = (v7(), v7());
    let (x_hash, y_hash) = (hash::token(&x), hash::token(&y));
    assert!(
        email_update::consume_email_update_token_if_matches(&c, &user_id, &x_hash, &y_hash,)
            .is_none()
    );
    assert!(
        email_update::consume_email_update_token_if_matches(&c, &user_id, &a_hash, &b_hash,)
            .is_some()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn email_update_consume_refuses_expired_row() {
    let c = caches(Duration::from_millis(80));
    let user_id = v7();
    let (a, b) = (v7(), v7());
    let (a_hash, b_hash) = (hash::token(&a), hash::token(&b));
    email_update::create_email_update_token(&c, &user_id, "old-hash", "new-hash", &a_hash, &b_hash);
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        email_update::consume_email_update_token_if_matches(&c, &user_id, &a_hash, &b_hash,)
            .is_none(),
        "created_at 过期必须拒绝"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn challenge_consume_is_atomic_and_checks_age() {
    let c = caches(Duration::from_secs(60));
    let id = v7();
    challenge::create_challenge(&c, &id);
    let wins = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut tasks = Vec::new();
    for _ in 0..12 {
        let c = c.clone();
        let id = id.clone();
        let wins = wins.clone();
        tasks.push(tokio::spawn(async move {
            if challenge::consume_challenge(&c, &id) {
                wins.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }
    assert_eq!(wins.load(std::sync::atomic::Ordering::SeqCst), 1);

    let c2 = caches(Duration::from_millis(80));
    let id2 = v7();
    challenge::create_challenge(&c2, &id2);
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        !challenge::consume_challenge(&c2, &id2),
        "过期 challenge 必须拒绝"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn eviction_listener_keeps_reverse_index_a_projection() {
    let c = caches(Duration::from_secs(60));
    let email_hash = hash::email("bob@qq.com");
    authenticate::create_authenticate_token(&c, &v7(), &email_hash, "s");
    let token2 = v7();
    authenticate::create_authenticate_token(&c, &token2, &email_hash, "s");
    c.authenticate.invalidate(&hash::token(&token2));
    let members = c
        .authenticate_by_email_hash
        .get(&email_hash)
        .expect("另一个成员仍在");
    assert_eq!(members.len(), 1, "listener 必须摘除被 invalidate 的成员");
    assert_ne!(members[0].token, hash::token(&token2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cache_fields_are_public_and_inspectable() {
    let ctx = TestCtx::new().await;
    let c = &ctx.state.cache;
    let user_id = v7();
    let session = ctx.session_for(&user_id);
    assert!(c.session.get(&hash::token(&session)).is_some());
    assert!(c.session_by_user.get(&user_id).is_some());
}
