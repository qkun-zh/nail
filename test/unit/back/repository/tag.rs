use super::context::{build_state, test_config};

use crate::repository::tag::create_tag_in_txn;

#[tokio::test]
async fn create_tag_returns_new_tag() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let mut guard = state.graph.write().await;
    let result: Result<_, agdb::DbError> = guard.transaction_mut(|txn| {
        let tag = create_tag_in_txn(txn, "rust")?;
        Ok(tag)
    });
    let tag = result.expect("transaction");
    assert_eq!(tag.name, "rust");
    assert!(!tag.id.is_empty());
}

#[tokio::test]
async fn create_tag_is_idempotent() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let mut guard = state.graph.write().await;
    let result: Result<_, agdb::DbError> = guard.transaction_mut(|txn| {
        let first = create_tag_in_txn(txn, "rust")?;
        let second = create_tag_in_txn(txn, "rust")?;
        Ok((first, second))
    });
    let (first, second) = result.expect("transaction");
    assert_eq!(first.id, second.id);
    assert_eq!(first.name, second.name);
}

#[tokio::test]
async fn different_tag_names_get_different_ids() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let mut guard = state.graph.write().await;
    let result: Result<_, agdb::DbError> = guard.transaction_mut(|txn| {
        let rust = create_tag_in_txn(txn, "rust")?;
        let axum = create_tag_in_txn(txn, "axum")?;
        Ok((rust, axum))
    });
    let (rust, axum) = result.expect("transaction");
    assert_ne!(rust.id, axum.id);
}
