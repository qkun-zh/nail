use super::context::{build_state, test_config};

use crate::logic::download::{
    consume_download_token, mint_download_token, resolve_version_pdf_path,
};
use crate::logic::error::LogicError;
use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::version::VersionDraft;

fn pdf_hash(seed: u8) -> String {
    format!("{seed:x}").repeat(32)
}

async fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::create_user(&state.graph, &nail_common::hash::email(email))
        .await
        .expect("user")
}

async fn create_article_fixture(
    state: &crate::infrastructure::state::AppState,
    author_id: &str,
    hash: &str,
) -> (String, String) {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: format!("Article {article_id}"),
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

fn token_from_url(url: &str) -> &str {
    url.split("?token=").nth(1).expect("token in minted url")
}

#[tokio::test]
async fn mint_then_consume_round_trips_and_the_token_is_single_use() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;

    let url = mint_download_token(&state, &author_id, &article_id, &version_id)
        .await
        .expect("mint");
    assert_eq!(
        url,
        format!(
            "/api/article/{article_id}/version/{version_id}/content/read?token={}",
            token_from_url(&url)
        )
    );

    let expected_path = std::path::Path::new(&state.config.server.pdf_storage_path)
        .join("11/11/11111111111111111111111111111111.pdf");
    let token = token_from_url(&url);
    let path = consume_download_token(&state, &author_id, &article_id, &version_id, token)
        .await
        .expect("consume");
    assert_eq!(path, expected_path);

    let error = consume_download_token(&state, &author_id, &article_id, &version_id, token)
        .await
        .expect_err("second consume");
    assert!(matches!(
        error,
        LogicError::BadRequest(message) if message == "invalid or expired download token"
    ));
}

#[tokio::test]
async fn consume_download_token_rejects_another_account() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let other_id = create_user(&state, "bob@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;

    let url = mint_download_token(&state, &author_id, &article_id, &version_id)
        .await
        .expect("mint");
    let token = token_from_url(&url);

    let error = consume_download_token(&state, &other_id, &article_id, &version_id, token)
        .await
        .expect_err("other user");
    assert!(matches!(
        error,
        LogicError::BadRequest(message) if message == "download token is bound to another account"
    ));
}

#[tokio::test]
async fn consume_download_token_rejects_an_unknown_token() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;

    let error = consume_download_token(
        &state,
        &author_id,
        &article_id,
        &version_id,
        &uuid::Uuid::now_v7().to_string(),
    )
    .await
    .expect_err("unknown token");
    assert!(matches!(
        error,
        LogicError::BadRequest(message) if message == "invalid or expired download token"
    ));
}

#[tokio::test]
async fn resolve_version_pdf_path_rejects_a_version_of_another_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    let (other_article, _) = create_article_fixture(&state, &author_id, &pdf_hash(2)).await;

    let error = resolve_version_pdf_path(&state, &author_id, &other_article, &version_id)
        .await
        .expect_err("wrong article");
    assert!(matches!(error, LogicError::NotFound(_)));

    let path = resolve_version_pdf_path(&state, &author_id, &article_id, &version_id)
        .await
        .expect("right article");
    assert!(path.ends_with("11/11/11111111111111111111111111111111.pdf"));
}

#[tokio::test]
async fn consume_download_token_rejects_a_version_mismatch() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1)).await;
    let (_, other_version) = create_article_fixture(&state, &author_id, &pdf_hash(2)).await;

    let url = mint_download_token(&state, &author_id, &article_id, &version_id)
        .await
        .expect("mint");
    let token = token_from_url(&url);

    let error = consume_download_token(&state, &author_id, &article_id, &other_version, token)
        .await
        .expect_err("version mismatch");
    assert!(matches!(
        error,
        LogicError::NotFound(message) if message == "version not found"
    ));
}
