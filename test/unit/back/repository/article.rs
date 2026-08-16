use super::context::{build_state, test_config};

use crate::repository::article::{
    ArticleDraft, ArticleUpdate, CreateArticleError, UpdateArticleError, create_article, owner_of,
    read_article, update_article,
};
use crate::repository::version::VersionDraft;

fn pdf_hash(seed: u8) -> String {
    format!("{seed:x}").repeat(32)
}

fn version_draft(number: &str, hash: &str) -> VersionDraft {
    VersionDraft {
        version_id: uuid::Uuid::now_v7().to_string(),
        version_number: number.to_string(),
        content_hash: hash.to_string(),
        note: "initial note".to_string(),
    }
}

fn article_draft(author_id: &str, title: &str, hash: &str, tags: Vec<String>) -> ArticleDraft {
    ArticleDraft {
        article_id: uuid::Uuid::now_v7().to_string(),
        author_id: author_id.to_string(),
        title: title.to_string(),
        summary: "a summary".to_string(),
        tags,
        first_version: version_draft("1.0.0", hash),
    }
}

async fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::create_user(&state.graph, &nail_common::hash::email(email))
        .await
        .expect("user")
}

async fn create_article_fixture(
    state: &crate::infrastructure::state::AppState,
    author_id: &str,
    title: &str,
    hash: &str,
) -> (String, String) {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.to_string(),
            summary: "a summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: hash.to_string(),
                note: "initial note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    (article_id, version_id)
}

#[tokio::test]
async fn create_article_writes_nodes_and_edges_and_reads_back() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();

    create_article(
        &state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.clone(),
            title: "My Article".to_string(),
            summary: "A longer summary.".to_string(),
            tags: vec!["rust".to_string(), "db".to_string()],
            first_version: VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: pdf_hash(1),
                note: "first".to_string(),
            },
        },
    )
    .await
    .expect("create");

    let detail = read_article(&state.graph, &article_id)
        .await
        .expect("read")
        .expect("article");
    assert_eq!(detail.title, "My Article");
    assert_eq!(detail.author_id, author_id);
    assert_eq!(detail.tags.len(), 2);
    assert!(detail.tags.iter().any(|tag| tag.name == "rust"));

    let version = crate::repository::version::read_version(&state.graph, &version_id)
        .await
        .expect("version")
        .expect("version");
    assert_eq!(version.version_number, "1.0.0");
    assert_eq!(version.content_hash, pdf_hash(1));
}

#[tokio::test]
async fn create_article_rejects_a_missing_author() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let error = create_article(
        &state.graph,
        &article_draft("missing", "Title", &pdf_hash(1), vec!["go".to_string()]),
    )
    .await
    .expect_err("missing author");
    assert!(matches!(error, CreateArticleError::AuthorMissing));
}

#[tokio::test]
async fn create_article_rejects_a_duplicate_title() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    create_article_fixture(&state, &author_id, "Duplicated", &pdf_hash(1)).await;

    let error = create_article(
        &state.graph,
        &article_draft(
            &author_id,
            "Duplicated",
            &pdf_hash(2),
            vec!["go".to_string()],
        ),
    )
    .await
    .expect_err("duplicate title");
    assert!(matches!(error, CreateArticleError::TitleTaken));
}

#[tokio::test]
async fn create_article_rejects_a_duplicate_content_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    create_article_fixture(&state, &author_id, "First", &pdf_hash(3)).await;

    let error = create_article(
        &state.graph,
        &article_draft(&author_id, "Second", &pdf_hash(3), vec!["go".to_string()]),
    )
    .await
    .expect_err("duplicate content hash");
    assert!(matches!(error, CreateArticleError::ContentHashTaken));
}

#[tokio::test]
async fn update_article_changes_fields_and_reconciles_tags() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Article", &pdf_hash(1)).await;

    update_article(
        &state.graph,
        &article_id,
        &ArticleUpdate {
            title: "Renamed".to_string(),
            summary: "new summary".to_string(),
            tags: vec!["go".to_string()],
        },
    )
    .await
    .expect("update");

    let detail = read_article(&state.graph, &article_id)
        .await
        .expect("read")
        .expect("article");
    assert_eq!(detail.title, "Renamed");
    assert_eq!(detail.summary, "new summary");
    assert_eq!(detail.tags.len(), 1);
    assert_eq!(detail.tags[0].name, "go");
}

#[tokio::test]
async fn update_article_rejects_a_duplicate_title() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "First", &pdf_hash(1)).await;
    create_article_fixture(&state, &author_id, "Second", &pdf_hash(2)).await;

    let error = update_article(
        &state.graph,
        &article_id,
        &ArticleUpdate {
            title: "Second".to_string(),
            summary: "summary".to_string(),
            tags: vec!["go".to_string()],
        },
    )
    .await
    .expect_err("duplicate title");
    assert!(matches!(error, UpdateArticleError::TitleTaken));
}

#[tokio::test]
async fn update_article_returns_missing_for_an_unknown_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let error = update_article(
        &state.graph,
        "missing",
        &ArticleUpdate {
            title: "Title".to_string(),
            summary: "summary".to_string(),
            tags: vec!["go".to_string()],
        },
    )
    .await
    .expect_err("missing");
    assert!(matches!(error, UpdateArticleError::Missing));
}

#[tokio::test]
async fn owner_of_returns_the_author() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let (article_id, _) = create_article_fixture(&state, &author_id, "Titled", &pdf_hash(9)).await;
    assert_eq!(
        owner_of(&state.graph, &article_id).await.expect("owner"),
        Some(author_id)
    );
    assert_eq!(
        owner_of(&state.graph, "missing").await.expect("owner"),
        None
    );
}

#[tokio::test]
async fn concurrent_identical_content_hashes_are_serialized_by_the_write_lock() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let shared_hash = pdf_hash(7);

    let first_draft = article_draft(
        &author_id,
        "Concurrent A",
        &shared_hash,
        vec!["a".to_string()],
    );
    let second_draft = article_draft(
        &author_id,
        "Concurrent B",
        &shared_hash,
        vec!["b".to_string()],
    );
    let first = create_article(&state.graph, &first_draft);
    let second = create_article(&state.graph, &second_draft);

    let (left, right) = tokio::join!(first, second);
    let mut accepted = 0;
    let mut deduplicated = 0;
    for result in [left, right] {
        match result {
            Ok(()) => accepted += 1,
            Err(CreateArticleError::ContentHashTaken) => deduplicated += 1,
            Err(other) => panic!("unexpected create result: {other}"),
        }
    }
    assert_eq!(
        accepted, 1,
        "exactly one identical content hash must be accepted"
    );
    assert_eq!(
        deduplicated, 1,
        "the racing duplicate must be rejected as ContentHashTaken"
    );
}

async fn tag_node_ids_by_name(
    state: &crate::infrastructure::state::AppState,
    name: &str,
) -> Vec<agdb::DbId> {
    let guard = state.graph.read().await;
    crate::repository::graph::find_by_index_sync(
        &guard,
        crate::repository::schema::KEY_TAG_NAME,
        name,
    )
    .expect("tag name index lookup")
}

#[tokio::test]
async fn update_article_removes_orphan_tags_and_keeps_shared_tags() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com").await;
    let first_draft = article_draft(
        &author_id,
        "Shared First",
        &pdf_hash(11),
        vec!["shared".to_string(), "one".to_string()],
    );
    let first_id = first_draft.article_id.clone();
    create_article(&state.graph, &first_draft)
        .await
        .expect("create first");
    let second_draft = article_draft(
        &author_id,
        "Shared Second",
        &pdf_hash(12),
        vec!["shared".to_string(), "two".to_string()],
    );
    let second_id = second_draft.article_id.clone();
    create_article(&state.graph, &second_draft)
        .await
        .expect("create second");
    assert_eq!(tag_node_ids_by_name(&state, "shared").await.len(), 1);

    update_article(
        &state.graph,
        &first_id,
        &ArticleUpdate {
            title: "Shared First".to_string(),
            summary: "a summary".to_string(),
            tags: vec!["one".to_string()],
        },
    )
    .await
    .expect("update first");

    let second_view = read_article(&state.graph, &second_id)
        .await
        .expect("read second")
        .expect("second article");
    assert!(second_view.tags.iter().any(|tag| tag.name == "shared"));
    assert_eq!(tag_node_ids_by_name(&state, "shared").await.len(), 1);

    update_article(
        &state.graph,
        &second_id,
        &ArticleUpdate {
            title: "Shared Second".to_string(),
            summary: "a summary".to_string(),
            tags: vec!["two".to_string()],
        },
    )
    .await
    .expect("update second");

    assert!(tag_node_ids_by_name(&state, "shared").await.is_empty());
    assert_eq!(tag_node_ids_by_name(&state, "one").await.len(), 1);
    assert_eq!(tag_node_ids_by_name(&state, "two").await.len(), 1);
}
