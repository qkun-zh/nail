use super::context::TestCtx;

#[tokio::test]
async fn create_challenge_returns_the_configured_difficulty_and_stores_the_challenge() {
    let context = TestCtx::new().await.expect("test context");
    let challenge = crate::logic::challenge::create_challenge(
        &context.state.configurator,
        &context.state.cache,
    );
    assert_eq!(challenge.difficulty, context.difficulty());
    assert!(
        context
            .state
            .cache
            .challenge
            .consume(&challenge.id.to_string())
            .is_some()
    );
}

#[tokio::test]
async fn create_challenge_is_single_use() {
    let context = TestCtx::new().await.expect("test context");
    let challenge = crate::logic::challenge::create_challenge(
        &context.state.configurator,
        &context.state.cache,
    );
    assert!(
        context
            .state
            .cache
            .challenge
            .consume(&challenge.id.to_string())
            .is_some()
    );
    assert!(
        context
            .state
            .cache
            .challenge
            .consume(&challenge.id.to_string())
            .is_none()
    );
}
