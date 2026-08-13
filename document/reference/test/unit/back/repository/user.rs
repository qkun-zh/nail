
use common::hash;
use uuid::Uuid;

use crate::repo::user::{
    UserWriteError, create_user, find_or_create_user, find_user_by_email_address_hash, read_user,
    read_user_names_by_ids, update_user_email, update_user_name,
};
use crate::unit_tests::context::TestCtx;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_user_sets_default_uuid_name() {
    let ctx = TestCtx::new().await;
    let user_id = Uuid::now_v7().to_string();
    let email_hash = hash::email("alice@qq.com");
    create_user(&ctx.state.db, &user_id, &email_hash)
        .await
        .expect("create");
    let entry = read_user(&ctx.state.db, &user_id)
        .await
        .expect("查询")
        .expect("用户必须存在");
    assert_eq!(entry.email_address_hash, email_hash);
    assert_eq!(
        entry.name,
        user_id.replace('-', ""),
        "默认名 = uuidv7 去连字符"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn find_or_create_user_is_idempotent() {
    let ctx = TestCtx::new().await;
    let email_hash = hash::email("alice@qq.com");
    let id1 = find_or_create_user(&ctx.state.db, &email_hash)
        .await
        .expect("第一次创建");
    let id2 = find_or_create_user(&ctx.state.db, &email_hash)
        .await
        .expect("第二次查找");
    assert_eq!(id1, id2, "同 email hash 必须返回同一 user");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn find_or_create_user_under_concurrency_has_single_winner() {
    let ctx = TestCtx::new().await;
    let email_hash = hash::email("alice@qq.com");
    let db = ctx.state.db.clone();
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let db = db.clone();
        let email_hash = email_hash.clone();
        tasks.push(tokio::spawn(async move {
            find_or_create_user(&db, &email_hash)
                .await
                .expect("并发创建")
        }));
    }
    let mut ids: Vec<String> = Vec::new();
    for t in tasks {
        ids.push(t.await.unwrap());
    }
    ids.dedup();
    assert_eq!(ids.len(), 1, "并发 find_or_create 必须收敛到同一账号");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn find_user_by_email_address_hash_is_exact_match() {
    let ctx = TestCtx::new().await;
    let email_hash = hash::email("alice@qq.com");
    let user_id = find_or_create_user(&ctx.state.db, &email_hash)
        .await
        .expect("创建");
    let found = find_user_by_email_address_hash(&ctx.state.db, &hash::email("alice@qq.com"))
        .await
        .expect("查询");
    assert_eq!(found.as_deref(), Some(user_id.as_str()));
    let variant = find_user_by_email_address_hash(&ctx.state.db, &hash::email("Alice@QQ.com"))
        .await
        .expect("查询");
    assert!(variant.is_none(), "未归一化的变体是不同 hash → 不命中");
    let missing = find_user_by_email_address_hash(&ctx.state.db, &hash::email("nobody@qq.com"))
        .await
        .expect("查询");
    assert!(missing.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_user_email_compare_and_swap_semantics() {
    let ctx = TestCtx::new().await;
    let user_id = Uuid::now_v7().to_string();
    let old_hash = hash::email("alice@qq.com");
    let new_hash = hash::email("bob@qq.com");
    create_user(&ctx.state.db, &user_id, &old_hash)
        .await
        .expect("create");

    assert!(
        update_user_email(&ctx.state.db, &user_id, &old_hash, &new_hash)
            .await
            .expect("CAS 成功")
    );
    let entry = read_user(&ctx.state.db, &user_id)
        .await
        .expect("查询")
        .expect("存在");
    assert_eq!(entry.email_address_hash, new_hash);

    assert!(
        !update_user_email(&ctx.state.db, &user_id, &old_hash, &new_hash)
            .await
            .expect("CAS 失败"),
        "old hash 不匹配必须返回 false"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_user_email_unique_conflict_is_detectable() {
    let ctx = TestCtx::new().await;
    let user_a = Uuid::now_v7().to_string();
    let user_b = Uuid::now_v7().to_string();
    create_user(&ctx.state.db, &user_a, &hash::email("a@qq.com"))
        .await
        .expect("a");
    create_user(&ctx.state.db, &user_b, &hash::email("b@qq.com"))
        .await
        .expect("b");
    let res = update_user_email(
        &ctx.state.db,
        &user_a,
        &hash::email("a@qq.com"),
        &hash::email("b@qq.com"),
    )
    .await
    .expect_err("新值被占必须报错");
    assert!(
        matches!(res, UserWriteError::AlreadyTaken),
        "必须是 AlreadyTaken（新 hash 被占语义），实际: {res:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_user_name_missing_user_is_detectable() {
    let ctx = TestCtx::new().await;
    let err = update_user_name(&ctx.state.db, &Uuid::now_v7().to_string(), "name")
        .await
        .expect_err("缺失用户必须报错");
    assert!(
        matches!(err, UserWriteError::UserMissing),
        "缺失用户 → UserMissing，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_user_name_persists() {
    let ctx = TestCtx::new().await;
    let user_id = Uuid::now_v7().to_string();
    create_user(&ctx.state.db, &user_id, &hash::email("a@qq.com"))
        .await
        .expect("create");
    assert!(
        update_user_name(&ctx.state.db, &user_id, "Alice-01")
            .await
            .expect("改名")
    );
    let entry = read_user(&ctx.state.db, &user_id)
        .await
        .expect("查询")
        .expect("存在");
    assert_eq!(entry.name, "Alice-01");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_user_names_by_ids_batch_with_missing_fallback() {
    let ctx = TestCtx::new().await;
    let id_a = Uuid::now_v7().to_string();
    let id_b = Uuid::now_v7().to_string();
    create_user(&ctx.state.db, &id_a, &hash::email("a@qq.com"))
        .await
        .expect("a");
    create_user(&ctx.state.db, &id_b, &hash::email("b@qq.com"))
        .await
        .expect("b");
    update_user_name(&ctx.state.db, &id_a, "Alice")
        .await
        .expect("改名");

    let empty = read_user_names_by_ids(&ctx.state.db, &[])
        .await
        .expect("空列表");
    assert!(empty.is_empty());

    let rows = read_user_names_by_ids(
        &ctx.state.db,
        &[id_a.clone(), id_b.clone(), Uuid::now_v7().to_string()],
    )
    .await
    .expect("批量");
    let map: std::collections::HashMap<String, String> = rows.into_iter().collect();
    assert_eq!(map.get(&id_a).map(|s| s.as_str()), Some("Alice"));
    assert_eq!(
        map.get(&id_b).map(|s| s.as_str()),
        Some(id_b.replace('-', "")).as_deref(),
        "b 未改名 → 默认名"
    );
}
