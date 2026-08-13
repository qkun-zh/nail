
use crate::repo::tag::{
    find_tag_ids_by_names_contains, get_or_create_tag_in_txn, read_article_tags,
    read_tags_by_articles,
};
use crate::unit_tests::context::TestCtx;
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_or_create_tag_in_txn_is_idempotent() {
    let ctx = TestCtx::new().await;
    let first = crate::unit_tests::context::get_or_create_tag(&ctx.state.db, "#rust")
        .await
        .expect("第一次");
    let second = crate::unit_tests::context::get_or_create_tag(&ctx.state.db, "#rust")
        .await
        .expect("第二次");
    assert_eq!(first.id, second.id, "同名 tag 必须复用同一行");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_or_create_tag_in_txn_rolls_back_with_transaction() {
    let ctx = TestCtx::new().await;
    let mut db = ctx.state.db.write().await;
    let err = db
        .transaction_mut(|txn| -> Result<(), agdb::DbError> {
            get_or_create_tag_in_txn(txn, "#temp")?;
            Err(agdb::DbError::query(
                agdb::DbErrorType::TypeError,
                "force rollback",
            ))
        })
        .expect_err("事务必须回滚");
    drop(err);
    drop(db);
    let found = find_tag_ids_by_names_contains(&ctx.state.db, &["#temp".to_string()])
        .await
        .expect("查询");
    assert!(found.is_empty(), "回滚的 tag 不得残留");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_get_or_create_converges_on_single_row() {
    let ctx = TestCtx::new().await;
    let db = ctx.state.db.clone();
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let db = db.clone();
        tasks.push(tokio::spawn(async move {
            crate::unit_tests::context::get_or_create_tag(&db, "#race")
                .await
                .expect("并发创建")
                .id
        }));
    }
    let mut ids: Vec<String> = Vec::new();
    for t in tasks {
        ids.push(t.await.unwrap());
    }
    ids.dedup();
    assert_eq!(ids.len(), 1, "并发创建同名 tag 必须收敛到同一行");
    let found = find_tag_ids_by_names_contains(&ctx.state.db, &["#race".to_string()])
        .await
        .expect("查询");
    assert_eq!(found.len(), 1, "库内只能有一行 #race");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn find_tag_ids_by_names_contains_returns_hits_only() {
    let ctx = TestCtx::new().await;
    let (_user_id, _session) = ctx.register("alice@qq.com").await;
    crate::unit_tests::context::get_or_create_tag(&ctx.state.db, "#alpha")
        .await
        .expect("alpha");
    crate::unit_tests::context::get_or_create_tag(&ctx.state.db, "#beta")
        .await
        .expect("beta");

    let hits = find_tag_ids_by_names_contains(
        &ctx.state.db,
        &["#alpha".to_string(), "#gamma".to_string()],
    )
    .await
    .expect("查询");
    assert_eq!(hits.len(), 1, "只返回命中的 tag");
    assert_eq!(hits[0].0, "#alpha");
    let empty = find_tag_ids_by_names_contains(&ctx.state.db, &[])
        .await
        .expect("查询");
    assert!(empty.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_article_tags_follows_create_order() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let article_id = ctx
        .create_article(&session, "t", "s", "#c#b#a", "1.0.0", "n")
        .await
        .0;
    let rows = read_article_tags(&ctx.state.db, &article_id)
        .await
        .expect("查询");
    let names: Vec<String> = rows
        .iter()
        .filter_map(|v| {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    assert_eq!(names, vec!["#c", "#b", "#a"], "按创建顺序返回");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_tags_by_articles_batches_across_articles() {
    let ctx = TestCtx::new().await;
    let (_user_id, session) = ctx.register("alice@qq.com").await;
    let a1 = ctx
        .create_article(&session, "t1", "s", "#x", "1.0.0", "n")
        .await
        .0;
    let a2 = ctx
        .create_article(&session, "t2", "s", "#x#y", "1.0.0", "n")
        .await
        .0;

    let rows = read_tags_by_articles(&ctx.state.db, &[a1.clone(), a2.clone()])
        .await
        .expect("查询");
    assert_eq!(rows.len(), 2);
    let tags_of = |aid: &str| -> Vec<String> {
        rows.iter()
            .find(|(id, _)| id == aid)
            .map(|(_, tags)| {
                tags.iter()
                    .filter_map(|v| {
                        v.get("name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    assert_eq!(tags_of(&a1), vec!["#x"]);
    assert_eq!(tags_of(&a2), vec!["#x", "#y"]);
    let empty = read_tags_by_articles(&ctx.state.db, &[])
        .await
        .expect("查询");
    assert!(empty.is_empty());
}
