
use agdb::QueryBuilder;
use uuid::Uuid;

use crate::repo::db::DbHandle;
use crate::repo::types::{
    ENTITY_TYPE_PERMISSION, KEY_CONTENT_HASH, KEY_EMAIL_ADDRESS_HASH, KEY_PERMISSION_NAME,
    KEY_ROLE_NAME, KEY_TAG_NAME, KEY_TITLE, KEY_TYPE, KEY_USER_NAME, alias_of,
};

const TEST_USER_ZERO_EMAIL: &str = "zero@test.local";

async fn initialized_database() -> DbHandle {
    let db = crate::repo::new("memory").await.expect("连接");
    crate::repo::schema::init_graph(&db, TEST_USER_ZERO_EMAIL)
        .await
        .expect("init");
    db
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn init_graph_creates_all_seven_indexes() {
    let db = initialized_database().await;
    let db_guard = db.read().await;
    let result = db_guard
        .exec(QueryBuilder::select().indexes().query())
        .expect("索引列表");
    let names: Vec<String> = result
        .elements
        .first()
        .map(|el| {
            el.values
                .iter()
                .filter_map(|kv| match &kv.key {
                    agdb::DbValue::String(name) => Some(name.clone()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    for key in [
        KEY_EMAIL_ADDRESS_HASH,
        KEY_USER_NAME,
        KEY_TITLE,
        KEY_CONTENT_HASH,
        KEY_TAG_NAME,
        KEY_ROLE_NAME,
        KEY_PERMISSION_NAME,
    ] {
        assert!(
            names.contains(&key.to_string()),
            "缺少索引 {key}: {names:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn init_graph_is_idempotent() {
    let db = initialized_database().await;
    crate::repo::schema::init_graph(&db, TEST_USER_ZERO_EMAIL)
        .await
        .expect("第二次 init 必须成功");
    crate::repo::schema::init_graph(&db, TEST_USER_ZERO_EMAIL)
        .await
        .expect("第三次 init 必须成功");
    let db_guard = db.read().await;
    let hits = crate::repo::db::find_by_index_sync(
        &db_guard,
        KEY_EMAIL_ADDRESS_HASH,
        &common::hash::email(TEST_USER_ZERO_EMAIL),
    )
    .expect("查重");
    assert_eq!(hits.len(), 1, "重复 init 不得重复建 user zero");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn init_graph_seeds_user_zero_findable_by_email_hash() {
    let db = initialized_database().await;
    let found = crate::repo::user::find_user_by_email_address_hash(
        &db,
        &common::hash::email(TEST_USER_ZERO_EMAIL),
    )
    .await
    .expect("查询");
    let user_zero_id = found.expect("user zero 必须可按 email hash 命中");
    let entry = crate::repo::user::read_user(&db, &user_zero_id)
        .await
        .expect("查询")
        .expect("user zero 必须存在");
    assert_eq!(
        entry.email_address_hash,
        common::hash::email(TEST_USER_ZERO_EMAIL)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn init_graph_seeds_required_roles_to_user_zero() {
    let db = initialized_database().await;
    let user_zero_id = crate::repo::user::find_user_by_email_address_hash(
        &db,
        &common::hash::email(TEST_USER_ZERO_EMAIL),
    )
    .await
    .expect("查询")
    .expect("user zero 必须存在");
    let auth = crate::repo::authorization::read_user_authorization(&db, &user_zero_id)
        .await
        .expect("读授权");
    let held: Vec<&str> = auth.roles.iter().map(|r| r.role_name.as_str()).collect();
    for required in crate::repo::authorization::REQUIRED_ROLES {
        assert!(
            held.contains(required),
            "user zero 必须持有必需角色 {required}，实际: {held:?}"
        );
    }
    let admin = auth
        .roles
        .iter()
        .find(|r| r.role_name == crate::repo::authorization::ROLE_ADMIN)
        .expect("admin 角色");
    for permission in crate::repo::authorization::ALL_PERMISSIONS {
        assert!(
            admin.permissions.iter().any(|p| p == permission),
            "admin 必须持有权限点 {permission}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn init_graph_seeds_all_permission_points() {
    let db = initialized_database().await;
    let db_guard = db.read().await;
    for name in crate::repo::authorization::ALL_PERMISSIONS {
        let hits = crate::repo::db::find_by_index_sync(&db_guard, KEY_PERMISSION_NAME, name)
            .expect("按名查权限点");
        assert_eq!(hits.len(), 1, "权限点 {name} 必须存在且唯一");
        let ty = db_guard
            .exec(agdb::QueryBuilder::select().ids([hits[0]]).query())
            .expect("读 type")
            .elements
            .first()
            .and_then(|el| {
                el.values
                    .iter()
                    .find(|kv| kv.key == agdb::DbValue::String(KEY_TYPE.to_string()))
            })
            .and_then(|kv| match &kv.value {
                agdb::DbValue::String(v) => Some(v.clone()),
                _ => None,
            })
            .expect("权限点必须有 type");
        assert_eq!(ty, ENTITY_TYPE_PERMISSION, "权限点 {name} 的 type 必须正确");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn init_graph_indexes_serve_deduplication_lookups() {
    let db = initialized_database().await;
    let probe_id = Uuid::now_v7().to_string();
    {
        let mut db_guard = db.write().await;
        db_guard
            .exec_mut(
                QueryBuilder::insert()
                    .nodes()
                    .aliases([alias_of("probe", &probe_id)])
                    .values([[
                        (KEY_EMAIL_ADDRESS_HASH, "probe-email-hash").into(),
                        (KEY_USER_NAME, "probe-user-name").into(),
                        (KEY_TITLE, "probe-title").into(),
                        (KEY_CONTENT_HASH, "probe-content-hash").into(),
                        (KEY_TAG_NAME, "probe-tag-name").into(),
                        (KEY_ROLE_NAME, "probe-role-name").into(),
                        (KEY_PERMISSION_NAME, "probe-permission-name").into(),
                    ]])
                    .query(),
            )
            .expect("插探针节点");
    }
    let db_guard = db.read().await;
    for (key, value) in [
        (KEY_EMAIL_ADDRESS_HASH, "probe-email-hash"),
        (KEY_USER_NAME, "probe-user-name"),
        (KEY_TITLE, "probe-title"),
        (KEY_CONTENT_HASH, "probe-content-hash"),
        (KEY_TAG_NAME, "probe-tag-name"),
        (KEY_ROLE_NAME, "probe-role-name"),
        (KEY_PERMISSION_NAME, "probe-permission-name"),
    ] {
        let hits = crate::repo::db::find_by_index_sync(&db_guard, key, value).expect("索引查询");
        assert_eq!(hits.len(), 1, "索引 {key} 必须命中探针值 {value}");
    }
}
