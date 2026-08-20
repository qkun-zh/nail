use super::context::{build_state, test_config};

use crate::repository::article::{ArticleDraft, create_article, owner_of};
use crate::repository::role::{ROLE_RECYCLER, hold_role, unhold_role};
use crate::repository::transfer::{TransferTargetError, transfer_account_assets, transfer_article};
use crate::repository::version::VersionDraft;

fn pdf_hash(seed: u8) -> String {
    format!("{seed:x}").repeat(32)
}

async fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::create_user(&state.database, &nail_common::hash::email(email))
        .await
        .expect("user")
}

#[tokio::test]
async fn transfer_account_assets_removes_the_user_node() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = create_user(&state, "alice@example.com").await;

    let outcome = transfer_account_assets(&state.database, &user_id)
        .await
        .expect("transfer");
    assert!(outcome.transferred_article_ids.is_empty());

    let entry = crate::repository::user::read_user(&state.database, &user_id)
        .await
        .expect("read");
    assert_eq!(entry, None);
}

#[tokio::test]
async fn transfer_account_assets_is_idempotent_for_a_missing_user() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let outcome = transfer_account_assets(&state.database, "missing")
        .await
        .expect("transfer");
    assert!(outcome.transferred_article_ids.is_empty());
}

#[tokio::test]
async fn transfer_article_repoints_the_owner_edge_to_the_recycler() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let recycler_id = create_user(&state, "user-zero@example.com").await;
    let article_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.clone(),
            title: "Article".to_string(),
            summary: "summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: uuid::Uuid::now_v7().to_string(),
                version_number: "1.0.0".to_string(),
                content_hash: pdf_hash(1),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create");

    assert_eq!(
        owner_of(&state.database, &article_id).await.expect("owner"),
        Some(author_id.clone())
    );

    transfer_article(&state.database, &article_id)
        .await
        .expect("transfer");

    assert_eq!(
        owner_of(&state.database, &article_id).await.expect("owner"),
        Some(recycler_id)
    );
}

#[tokio::test]
async fn transfer_article_reports_a_missing_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let error = transfer_article(&state.database, "missing")
        .await
        .expect_err("missing");
    assert!(matches!(error, TransferTargetError::TargetMissing));
}

async fn create_article_for(
    state: &crate::infrastructure::state::AppState,
    author_id: &str,
    title: &str,
    hash: &str,
) -> (String, String) {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.to_string(),
            summary: "summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: hash.to_string(),
                note: "note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    (article_id, version_id)
}

async fn user_zero_id(state: &crate::infrastructure::state::AppState) -> String {
    crate::repository::user::read_user_by_email_address_hash(
        &state.database,
        &nail_common::hash::email("user-zero@example.com"),
    )
    .await
    .expect("lookup user zero")
    .expect("seeded user zero")
}

#[tokio::test]
async fn recycler_selection_chooses_the_least_loaded_holder() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    unhold_role(&state.database, &user_zero_id(&state).await, ROLE_RECYCLER)
        .await
        .expect("unhold user zero");

    let busy = create_user(&state, "busy@example.com").await;
    let free = create_user(&state, "free@example.com").await;
    hold_role(&state.database, &busy, ROLE_RECYCLER)
        .await
        .expect("hold busy");
    hold_role(&state.database, &free, ROLE_RECYCLER)
        .await
        .expect("hold free");

    let (busy_article, busy_version) =
        create_article_for(&state, &busy, "Busy One", &pdf_hash(21)).await;
    create_article_for(&state, &busy, "Busy Two", &pdf_hash(22)).await;
    let comment_id = uuid::Uuid::now_v7().to_string();
    crate::repository::comment::create_top_level_comment(
        &state.database,
        &comment_id,
        &free,
        &busy_version,
        "hello",
    )
    .await
    .expect("comment");

    let author = create_user(&state, "carol@example.com").await;
    let (transferred, _) =
        create_article_for(&state, &author, "Carol Article", &pdf_hash(23)).await;
    transfer_article(&state.database, &transferred)
        .await
        .expect("transfer");

    assert_eq!(
        owner_of(&state.database, &transferred)
            .await
            .expect("owner"),
        Some(free.clone())
    );
    assert_eq!(
        owner_of(&state.database, &busy_article)
            .await
            .expect("owner"),
        Some(busy)
    );
}

#[tokio::test]
async fn recycler_selection_breaks_ties_by_larger_user_id() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    unhold_role(&state.database, &user_zero_id(&state).await, ROLE_RECYCLER)
        .await
        .expect("unhold user zero");

    let first = create_user(&state, "first@example.com").await;
    let second = create_user(&state, "second@example.com").await;
    hold_role(&state.database, &first, ROLE_RECYCLER)
        .await
        .expect("hold first");
    hold_role(&state.database, &second, ROLE_RECYCLER)
        .await
        .expect("hold second");
    create_article_for(&state, &first, "First Article", &pdf_hash(31)).await;
    create_article_for(&state, &second, "Second Article", &pdf_hash(32)).await;

    let author = create_user(&state, "carol@example.com").await;
    let (transferred, _) =
        create_article_for(&state, &author, "Carol Article", &pdf_hash(33)).await;
    transfer_article(&state.database, &transferred)
        .await
        .expect("transfer");

    let expected = if first > second { first } else { second };
    assert_eq!(
        owner_of(&state.database, &transferred)
            .await
            .expect("owner"),
        Some(expected)
    );
}

#[tokio::test]
async fn account_transfer_excludes_the_transferring_author() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_zero = user_zero_id(&state).await;
    create_article_for(&state, &user_zero, "Zero One", &pdf_hash(41)).await;
    create_article_for(&state, &user_zero, "Zero Two", &pdf_hash(42)).await;

    let author = create_user(&state, "alice@example.com").await;
    hold_role(&state.database, &author, ROLE_RECYCLER)
        .await
        .expect("hold recycler");
    let (article_id, version_id) = create_article_for(&state, &author, "Mine", &pdf_hash(43)).await;
    let comment_id = uuid::Uuid::now_v7().to_string();
    crate::repository::comment::create_top_level_comment(
        &state.database,
        &comment_id,
        &author,
        &version_id,
        "mine",
    )
    .await
    .expect("comment");

    let outcome = transfer_account_assets(&state.database, &author)
        .await
        .expect("transfer account");
    assert_eq!(outcome.transferred_article_ids, vec![article_id.clone()]);
    assert_eq!(
        owner_of(&state.database, &article_id).await.expect("owner"),
        Some(user_zero.clone())
    );
    assert_eq!(
        crate::repository::comment::owner_of_comment(&state.database, &comment_id)
            .await
            .expect("comment owner"),
        Some(user_zero)
    );
    let entry = crate::repository::user::read_user(&state.database, &author)
        .await
        .expect("read user");
    assert_eq!(entry, None);
}

#[tokio::test]
async fn transfer_article_reports_no_recycler() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    unhold_role(&state.database, &user_zero_id(&state).await, ROLE_RECYCLER)
        .await
        .expect("unhold user zero");

    let author = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_for(&state, &author, "Ownerless", &pdf_hash(51)).await;

    let error = transfer_article(&state.database, &article_id)
        .await
        .expect_err("no recycler");
    assert!(matches!(error, TransferTargetError::NoRecycler));
}
