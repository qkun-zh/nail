use super::context::{TestCtx, build_state, test_config};
use crate::logic::error::LogicError;
use crate::logic::session::{create_session, read_session};
use crate::repository::role::ROLE_MEMBER;

fn pdf_hash(seed: u8) -> String {
    format!("{seed:x}").repeat(32)
}

fn admin(context: &TestCtx) -> String {
    crate::repository::user::read_user_by_email_address_hash(
        &context.state.database,
        &common::hash::hash("user-zero@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("lookup user zero")
    .expect("seeded user zero")
}

fn member(context: &TestCtx, email: &str) -> String {
    let user_id = crate::repository::user::create_user(
        &context.state.database,
        &common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .expect("user");
    crate::repository::role::hold_role(&context.state.database, &user_id, ROLE_MEMBER)
        .expect("member role");
    user_id
}

#[test]
fn probe_1_read_tags_must_not_panic_on_a_far_page() {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            let context = TestCtx::new().await.expect("ctx");
            let actor = admin(&context);
            context.seed_tags(&["alpha"]);
            let _ = crate::logic::tag::read_tags(&context.state, &actor, 2, 200);
        });
    }));
    assert!(
        outcome.is_ok(),
        "read_tags(page=2).await must return an empty page, not panic (out-of-range slice)"
    );
}

#[tokio::test]
async fn probe_2_delete_session_with_noncanonical_token_must_remove_the_session() {
    let context = TestCtx::new().await.expect("ctx");
    let session_token =
        create_session(&context.state, &uuid::Uuid::now_v7().to_string()).expect("create");
    let uppercased = session_token.to_uppercase();
    assert_ne!(session_token, uppercased, "case must differ");

    crate::logic::session::delete_session(&context.state, &uppercased).expect("delete");

    assert!(
        read_session(&context.state, &session_token).is_err(),
        "session must be deleted even when the client echoes the token in a different case"
    );
}

#[tokio::test]
async fn probe_3_a_comment_heavy_article_exceeds_the_32_doc_per_article_assumption() {
    let context = TestCtx::new().await.expect("ctx");
    let actor = member(&context, "alice@example.com");
    context.seed_tags(&["rust"]);

    let (_article_id, version_id) = crate::logic::article::create_article(
        &context.state,
        &actor,
        crate::logic::article::ArticleCreateInput {
            title: "Probe Three",
            summary: "summary",
            tags: "rust",
            version: "1.0.0",
            note: "note",
            upload: context.upload(b"probe3-unique-pdf"),
        },
    )
    .await
    .expect("create article");
    for c in 0..40 {
        crate::logic::comment::create_comment(
            &context.state,
            &actor,
            &version_id,
            &format!("comment {c}"),
        )
        .await
        .expect("comment");
    }

    let doc_count = context
        .state
        .searcher
        .sync_all(&context.state.database)
        .await
        .expect("rebuild search index");
    assert!(
        doc_count > 32,
        "one article with 40 comments indexes {doc_count} docs; the search page \
         window assumes at most 32 docs per article and is therefore underestimated"
    );
}

#[tokio::test]
async fn probe_4_token_must_survive_a_version_mismatch_attempt() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = crate::repository::user::create_user(
        &state.database,
        &common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("user");
    let make_article = |seed: u8| {
        let article_id = uuid::Uuid::now_v7().to_string();
        let version_id = uuid::Uuid::now_v7().to_string();
        let draft = crate::repository::article::ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.clone(),
            title: format!("Article {seed}"),
            summary: "summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: crate::repository::version::VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: pdf_hash(seed),
                note: "note".to_string(),
            },
        };
        (article_id, version_id, draft)
    };
    let (article_id, version_id, draft) = make_article(1);
    let (other_article, other_version, other_draft) = make_article(2);
    crate::repository::article::create_article(&state.database, &draft).expect("article a");
    crate::repository::article::create_article(&state.database, &other_draft).expect("article b");

    let url =
        crate::logic::download::mint_download_token(&state, &author_id, &article_id, &version_id)
            .expect("mint");
    let token = url.split("?token=").nth(1).expect("token");

    let error = crate::logic::download::consume_download_token(
        &state,
        &author_id,
        &other_article,
        &other_version,
        token,
    )
    .expect_err("mis-targeted consume");
    assert!(matches!(error, LogicError::NotFound(_)));

    crate::logic::download::consume_download_token(
        &state,
        &author_id,
        &article_id,
        &version_id,
        token,
    )
    .expect("token must survive a version-mismatch attempt for its intended target");
}
