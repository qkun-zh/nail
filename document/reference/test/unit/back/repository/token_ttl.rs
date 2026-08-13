
use std::time::{Duration, Instant};

use common::hash;
use uuid::Uuid;

use crate::repo::token::TokenCaches;
use crate::repo::token::{authenticate, session};

fn caches(ttl: Duration) -> TokenCaches {
    TokenCaches::new(ttl, ttl, ttl, ttl, 100_000)
}

fn v7() -> String {
    Uuid::now_v7().to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn main_caches_have_bare_cache_level_ttl_and_members_in_same_timeseries() {
    let ttl = Duration::from_secs(60);
    let c = caches(ttl);

    for cache in [
        c.authenticate.policy(),
        c.session.policy(),
        c.email_update.policy(),
        c.deregister.policy(),
        c.download.policy(),
    ] {
        assert_eq!(
            cache.time_to_live(),
            Some(ttl),
            "主缓存必须设裸 time_to_live（不得退回 per-entry Expiry）"
        );
    }

    let email_hash = hash::email("probe@qq.com");
    let token = v7();
    authenticate::create_authenticate_token(&c, &token, &email_hash, "s");
    let members = c.authenticate_by_email_hash.get(&email_hash).unwrap();
    assert_eq!(members.len(), 1);
    let m = &members[0];
    let now = Instant::now();
    let remain = m.expires_at.saturating_duration_since(now);
    assert!(remain <= ttl, "成员到期不得晚于 now+ttl");
    assert!(
        remain >= ttl - Duration::from_secs(1),
        "成员到期必须是 now+ttl（同一时间序），实际剩余 {remain:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bare_ttl_still_ttl_aware_rejects_expired_not_evicted() {
    let c = caches(Duration::from_millis(80));
    let token = v7();
    session::create_session_token(&c, &token, "u");
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        session::find_user_id_by_session_token(&c, &token).is_none(),
        "裸 TTL 下过期未驱逐条目必须被 get 过滤"
    );
}
