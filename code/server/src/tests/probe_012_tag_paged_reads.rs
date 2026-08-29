use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use cache::UserId;

use super::context::TestCtx;

async fn session_for(context: &TestCtx, email: &str) -> (String, String) {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .expect("user");
    let token = Uuid::now_v7().to_string();
    let key = crate::logic::session::cache_key(&token).expect("cache key");
    context
        .state
        .cache
        .session
        .insert(&key, UserId::new(user_id.clone()).expect("user id"));
    (user_id, token)
}

async fn seed_tags(context: &TestCtx, token: &str, count: usize) {
    for index in 0..count {
        let (status, body) = context
            .post(
                "/tags",
                json!({ "name": format!("probe-tag-{index:02}") }),
                Some(token),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "body: {body}");
    }
}

#[tokio::test]
async fn tag_reads_page_by_explicit_page_and_limit() {
    let ctx = TestCtx::new().await.expect("ctx");
    let (_, token) = session_for(&ctx, "user-zero@example.com").await;
    seed_tags(&ctx, &token, 25).await;

    let (status, body) = ctx.get("/tags?page=1&limit=8", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"].as_u64(), Some(25));
    assert_eq!(body["data"]["has_next"].as_bool(), Some(true));
    assert_eq!(body["data"]["items"].as_array().expect("items").len(), 8);

    let (_, body) = ctx.get("/tags?page=2&limit=8", Some(&token)).await;
    assert_eq!(body["data"]["has_next"].as_bool(), Some(true));
    assert_eq!(body["data"]["items"].as_array().expect("items").len(), 8);

    let (_, body) = ctx.get("/tags?page=4&limit=8", Some(&token)).await;
    assert_eq!(body["data"]["total"].as_u64(), Some(25));
    assert_eq!(body["data"]["has_next"].as_bool(), Some(false));
    assert_eq!(body["data"]["items"].as_array().expect("items").len(), 1);

    let (status, body) = ctx.get("/tags?page=3&limit=10", Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "slice re-slice honors limit");
    assert_eq!(body["data"]["items"].as_array().expect("items").len(), 5);
}
