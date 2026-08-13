
use agdb::QueryBuilder;

use crate::repo::db::DbHandle;
use crate::repo::types::{ENTITY_TYPE_USER, KEY_ID, KEY_TYPE, alias_of};

async fn write_read_probe(db: &DbHandle) {
    let mut db = db.write().await;
    db.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .aliases([alias_of(ENTITY_TYPE_USER, "probe-user")])
            .values([[
                (KEY_TYPE, ENTITY_TYPE_USER).into(),
                (KEY_ID, "probe-user").into(),
            ]])
            .query(),
    )
    .expect("插入探针节点");
    let id = crate::repo::db::resolve_node_id_sync(&db, ENTITY_TYPE_USER, "probe-user")
        .expect("按别名解析")
        .expect("探针节点必须可解析");
    let business_id = crate::repo::db::read_node_sync::<crate::repo::types::IdRow>(&db, id)
        .expect("读属性")
        .expect("id 属性必须存在")
        .id;
    assert_eq!(business_id, "probe-user", "写读闭环必须一致");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn memory_indicators_all_connect() {
    for indicator in ["memory", "mem", ":memory:", "in-memory", "MEMORY", "Memory"] {
        let db = crate::repo::new(indicator).await.unwrap_or_else(|e| {
            panic!("内存指示词 {indicator:?} 必须可用: {e}");
        });
        write_read_probe(&db).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_or_root_path_rejected() {
    for path in ["", "/"] {
        let err = crate::repo::new(path).await.expect_err("必须报错");
        assert!(
            err.to_string().contains("invalid db_path"),
            "路径校验失败提示不符: {err}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_path_creates_parent_dirs_and_works() {
    let dir = std::env::temp_dir().join(format!("nail_db_test_{}", std::process::id()));
    let path = dir.join("nested").join("db.agdb");
    let db = crate::repo::new(path.to_str().unwrap())
        .await
        .expect("连接");
    assert!(dir.join("nested").is_dir(), "父目录必须自动创建");
    write_read_probe(&db).await;
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn file_database_persists_data_across_reopen() {
    let dir = std::env::temp_dir().join(format!("nail_db_reopen_{}", std::process::id()));
    let path = dir.join("persist").join("db.agdb");
    let path_str = path.to_str().unwrap().to_string();
    let email_hash = "persist-probe-hash";
    let user_id;
    {
        let db = crate::repo::new(&path_str).await.expect("首次连接");
        crate::repo::schema::init_graph(&db, "zero@persist.local")
            .await
            .expect("init");
        user_id = crate::repo::user::find_or_create_user(&db, email_hash)
            .await
            .expect("写入用户");
    }
    {
        let reopened = crate::repo::new(&path_str).await.expect("重开");
        let found = crate::repo::user::find_user_by_email_address_hash(&reopened, email_hash)
            .await
            .expect("查询");
        assert_eq!(
            found.as_deref(),
            Some(user_id.as_str()),
            "重开后数据必须仍在（memory-mapped 持久态）"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
