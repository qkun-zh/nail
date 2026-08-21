use super::context::{build_state, test_config};

use crate::repository::tag::create_tag_in_scope;

#[tokio::test]
async fn create_tag_returns_new_tag() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let tag = state
        .database
        .write(|scope| create_tag_in_scope(scope, "rust"))
        .expect("tag creation");
    assert_eq!(tag.name, "rust");
    assert!(!tag.id.is_empty());
}

#[tokio::test]
async fn create_tag_is_idempotent() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let (first, second) = state
        .database
        .write(|scope| {
            let first = create_tag_in_scope(scope, "rust")?;
            let second = create_tag_in_scope(scope, "rust")?;
            Ok((first, second))
        })
        .expect("transaction");
    assert_eq!(first.id, second.id);
    assert_eq!(first.name, second.name);
}

#[tokio::test]
async fn different_tag_names_get_different_ids() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let (rust, axum) = state
        .database
        .write(|scope| {
            let rust = create_tag_in_scope(scope, "rust")?;
            let axum = create_tag_in_scope(scope, "axum")?;
            Ok((rust, axum))
        })
        .expect("transaction");
    assert_ne!(rust.id, axum.id);
}
