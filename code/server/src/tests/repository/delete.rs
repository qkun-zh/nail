use super::context::{build_state, test_config};

use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::delete::{
    delete_article, delete_user, delete_version, soft_delete_article, soft_delete_comment,
    soft_delete_user, soft_delete_version, undelete_soft_user,
};
use crate::repository::schema::CommentRow;
use crate::repository::version::{VersionDraft, versions_of};
use database::{EdgeKind, NodeKind};

fn pdf_hash(seed: u8) -> String {
    format!("{seed:x}").repeat(32)
}

fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::create_user(
        &state.database,
        &common::hash::hash(email.as_bytes()).expect("hash must succeed"),
    )
    .expect("user")
}

fn create_article_fixture(
    state: &crate::infrastructure::state::AppState,
    author_id: &str,
    hash: &str,
) -> (String, String) {
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: "Article".to_string(),
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
    .expect("create article");
    (article_id, version_id)
}

fn insert_comment_node_in_scope(
    scope: &mut database::WriteScope<'_, '_>,
    author_node: database::NodeId,
    comment_id: &str,
) -> database::NodeId {
    scope
        .insert_node(&CommentRow {
            id: comment_id.to_string(),
            content: "a comment".to_string(),
        })
        .expect("comment node");
    let comment_node = scope
        .resolve(NodeKind::Comment, comment_id)
        .expect("resolve comment")
        .expect("comment present");
    scope
        .insert_edge(
            NodeKind::User,
            author_node,
            EdgeKind::UserAuthorComment,
            NodeKind::Comment,
            comment_node,
        )
        .expect("user comment edge");
    comment_node
}

fn insert_comment(
    state: &crate::infrastructure::state::AppState,
    version_id: &str,
    author_id: &str,
) -> String {
    let comment_id = uuid::Uuid::now_v7().to_string();
    state
        .database
        .write(|scope| {
            let author_node = scope
                .resolve(NodeKind::User, author_id)?
                .expect("author present");
            let comment_node = insert_comment_node_in_scope(scope, author_node, &comment_id);
            let version_node = scope
                .resolve(NodeKind::Version, version_id)?
                .expect("version present");
            scope.insert_edge(
                NodeKind::Comment,
                comment_node,
                EdgeKind::CommentAttachVersion,
                NodeKind::Version,
                version_node,
            )
        })
        .expect("comment insert");
    comment_id
}

fn insert_reply(
    state: &crate::infrastructure::state::AppState,
    parent_comment_id: &str,
    author_id: &str,
) -> String {
    let comment_id = uuid::Uuid::now_v7().to_string();
    state
        .database
        .write(|scope| {
            let author_node = scope
                .resolve(NodeKind::User, author_id)?
                .expect("author present");
            let comment_node = insert_comment_node_in_scope(scope, author_node, &comment_id);
            let parent_node = scope
                .resolve(NodeKind::Comment, parent_comment_id)?
                .expect("parent present");
            scope.insert_edge(
                NodeKind::Comment,
                comment_node,
                EdgeKind::CommentReplyComment,
                NodeKind::Comment,
                parent_node,
            )
        })
        .expect("reply insert");
    comment_id
}

fn insert_comment_tree(
    state: &crate::infrastructure::state::AppState,
    version_id: &str,
    author_id: &str,
) {
    let top = insert_comment(state, version_id, author_id);
    let reply = insert_reply(state, &top, author_id);
    insert_reply(state, &reply, author_id);
}

fn assert_no_comment_subtree_remains(state: &crate::infrastructure::state::AppState) {
    let comments = state
        .database
        .read(|scope| scope.count_nodes(NodeKind::Comment, None))
        .expect("count comments");
    assert_eq!(comments, 0, "no comment nodes remain");
}

#[tokio::test]
async fn delete_user_removes_the_user_node() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = create_user(&state, "alice@example.com");

    delete_user(&state.database, &user_id).expect("delete");

    let entry = crate::repository::user::read_user(&state.database, &user_id).expect("read");
    assert_eq!(entry, None);
}

#[tokio::test]
async fn delete_user_is_idempotent_for_a_missing_user() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    delete_user(&state.database, "missing").expect("delete");
}

#[tokio::test]
async fn delete_article_cascades_versions_and_comments_and_collects_hashes() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    insert_comment(&state, &version_id, &author_id);

    let outcome = delete_article(&state.database, &article_id).expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(1)]);

    assert!(
        crate::repository::article::read_article(&state.database, &article_id)
            .expect("read")
            .is_none()
    );
    assert!(
        crate::repository::version::read_version(&state.database, &version_id)
            .expect("read")
            .is_none()
    );
}

#[tokio::test]
async fn delete_article_is_idempotent_for_a_missing_article() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let outcome = delete_article(&state.database, "missing").expect("delete");
    assert!(outcome.removed_pdf_hashes.is_empty());
}

#[tokio::test]
async fn delete_version_removes_only_the_version_and_refreshes_latest() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, first_version) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    let second_version = uuid::Uuid::now_v7().to_string();
    crate::repository::version::create_version(
        &state.database,
        &article_id,
        &VersionDraft {
            version_id: second_version.clone(),
            version_number: "2.0.0".to_string(),
            content_hash: pdf_hash(2),
            note: "note".to_string(),
        },
    )
    .expect("v2");

    let outcome = delete_version(&state.database, &second_version).expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(2)]);

    let (remaining, _) = versions_of(&state.database, &article_id, 10, 0).expect("versions");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, first_version);
}

#[tokio::test]
async fn delete_version_is_idempotent_for_a_missing_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let outcome = delete_version(&state.database, "missing").expect("delete");
    assert!(outcome.removed_pdf_hashes.is_empty());
}

#[tokio::test]
async fn delete_article_removes_a_nested_comment_subtree_and_collects_the_pdf_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    insert_comment_tree(&state, &version_id, &author_id);

    let outcome = delete_article(&state.database, &article_id).expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(1)]);

    assert_no_comment_subtree_remains(&state);
}

#[tokio::test]
async fn delete_version_removes_a_nested_comment_subtree_and_collects_the_pdf_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    insert_comment_tree(&state, &version_id, &author_id);

    let outcome = delete_version(&state.database, &version_id).expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(1)]);

    assert_no_comment_subtree_remains(&state);
}

#[tokio::test]
async fn delete_user_removes_a_nested_comment_subtree_and_collects_the_pdf_hash() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    insert_comment_tree(&state, &version_id, &author_id);

    let outcome = delete_user(&state.database, &author_id).expect("delete");
    assert_eq!(outcome.removed_pdf_hashes, vec![pdf_hash(1)]);

    assert_no_comment_subtree_remains(&state);
    assert_eq!(
        crate::repository::user::read_user(&state.database, &author_id).expect("read user"),
        None
    );
}

fn has_soft_deleted_flag(
    state: &crate::infrastructure::state::AppState,
    kind: NodeKind,
    id: &str,
) -> bool {
    crate::repository::delete::is_soft_deleted(&state.database, kind, id).expect("flag read")
}

#[tokio::test]
async fn soft_delete_user_cascades_the_flag_over_articles_comments_and_the_user() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    insert_comment_tree(&state, &version_id, &author_id);

    soft_delete_user(&state.database, &author_id).expect("soft delete");

    assert!(has_soft_deleted_flag(&state, NodeKind::User, &author_id));
    assert!(has_soft_deleted_flag(
        &state,
        NodeKind::Article,
        &article_id
    ));
    assert!(has_soft_deleted_flag(
        &state,
        NodeKind::Version,
        &version_id
    ));
    let comments = state
        .database
        .read(|scope| scope.count_nodes(NodeKind::Comment, None))
        .expect("count comments");
    assert_eq!(comments, 3);
}

#[tokio::test]
async fn undelete_soft_user_clears_the_flags_over_the_whole_subtree() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    insert_comment_tree(&state, &version_id, &author_id);

    soft_delete_user(&state.database, &author_id).expect("soft delete");
    undelete_soft_user(&state.database, &author_id).expect("undelete");

    assert!(!has_soft_deleted_flag(&state, NodeKind::User, &author_id));
    assert!(!has_soft_deleted_flag(
        &state,
        NodeKind::Article,
        &article_id
    ));
    assert!(!has_soft_deleted_flag(
        &state,
        NodeKind::Version,
        &version_id
    ));
}

#[tokio::test]
async fn soft_delete_user_is_idempotent_for_a_missing_user() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    soft_delete_user(&state.database, "missing").expect("soft delete");
    undelete_soft_user(&state.database, "missing").expect("undelete");
}

#[tokio::test]
async fn soft_delete_article_cascades_the_flag_over_the_subtree() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    insert_comment(&state, &version_id, &author_id);

    soft_delete_article(&state.database, &article_id).expect("soft delete");

    assert!(has_soft_deleted_flag(
        &state,
        NodeKind::Article,
        &article_id
    ));
    assert!(has_soft_deleted_flag(
        &state,
        NodeKind::Version,
        &version_id
    ));
    let (remaining, _) = versions_of(&state.database, &article_id, 10, 0).expect("versions");
    assert_eq!(remaining.len(), 0, "versions hidden by article flag");
    assert!(
        crate::repository::version::read_version(&state.database, &version_id)
            .expect("read version")
            .is_some(),
        "row stays available at the repository layer; visibility gating lives in logic"
    );
}

#[tokio::test]
async fn soft_delete_version_cascades_the_flag_over_versions_and_comments() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, first_version) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    let second_version = uuid::Uuid::now_v7().to_string();
    crate::repository::version::create_version(
        &state.database,
        &article_id,
        &VersionDraft {
            version_id: second_version.clone(),
            version_number: "2.0.0".to_string(),
            content_hash: pdf_hash(2),
            note: "note".to_string(),
        },
    )
    .expect("v2");
    insert_comment(&state, &second_version, &author_id);

    soft_delete_version(&state.database, &second_version).expect("soft delete");

    assert!(has_soft_deleted_flag(
        &state,
        NodeKind::Version,
        &second_version
    ));
    assert!(!has_soft_deleted_flag(
        &state,
        NodeKind::Version,
        &first_version
    ));
    assert!(!has_soft_deleted_flag(
        &state,
        NodeKind::Article,
        &article_id
    ));
    let (page, _) = crate::repository::comment::read_comments_page_by_version(
        &state.database,
        &second_version,
        10,
        0,
    )
    .expect("comment page");
    assert!(
        page.is_empty(),
        "comments hidden under a soft-deleted version"
    );
}

#[tokio::test]
async fn soft_delete_comment_cascades_the_flag_over_the_reply_subtree() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    let top = insert_comment(&state, &version_id, &author_id);
    let reply = insert_reply(&state, &top, &author_id);

    soft_delete_comment(&state.database, &top).expect("soft delete");

    assert!(has_soft_deleted_flag(&state, NodeKind::Comment, &top));
    assert!(has_soft_deleted_flag(&state, NodeKind::Comment, &reply));
    assert!(
        crate::repository::comment::read_comment_item(&state.database, &reply)
            .expect("read reply")
            .is_some(),
        "reply row stays available at the repository layer"
    );
}

#[tokio::test]
async fn soft_delete_is_idempotent_for_a_missing_node() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    soft_delete_article(&state.database, "missing").expect("soft delete");
    soft_delete_version(&state.database, "missing").expect("soft delete");
    soft_delete_comment(&state.database, "missing").expect("soft delete");
}

#[tokio::test]
async fn clearing_the_soft_deleted_flag_revives_the_node() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    let top = insert_comment(&state, &version_id, &author_id);

    soft_delete_article(&state.database, &article_id).expect("soft delete article");
    soft_delete_version(&state.database, &version_id).expect("soft delete version");
    soft_delete_comment(&state.database, &top).expect("soft delete comment");

    crate::repository::delete::clear_soft_deleted_flag(&state.database, &article_id)
        .expect("clear article");
    crate::repository::delete::clear_soft_deleted_flag(&state.database, &version_id)
        .expect("clear version");
    crate::repository::delete::clear_soft_deleted_flag(&state.database, &top)
        .expect("clear comment");

    assert!(
        crate::repository::article::read_article(&state.database, &article_id)
            .expect("read article")
            .is_some(),
        "article revived"
    );
    assert!(
        crate::repository::version::read_version(&state.database, &version_id)
            .expect("read version")
            .is_some(),
        "version revived"
    );
    assert!(
        crate::repository::comment::read_comment_item(&state.database, &top)
            .expect("read comment")
            .is_some(),
        "comment revived"
    );
    assert!(!has_soft_deleted_flag(
        &state,
        NodeKind::Article,
        &article_id
    ));
    assert!(!has_soft_deleted_flag(
        &state,
        NodeKind::Version,
        &version_id
    ));
    assert!(!has_soft_deleted_flag(&state, NodeKind::Comment, &top));
}

#[tokio::test]
async fn soft_deleted_article_stays_available_at_repository_layer_while_versions_hide() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, _version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));

    soft_delete_article(&state.database, &article_id).expect("soft delete");

    assert!(
        crate::repository::article::read_article(&state.database, &article_id)
            .expect("read article")
            .is_some(),
        "row stays available at the repository layer; visibility gating lives in logic"
    );
    let (remaining, _) = versions_of(&state.database, &article_id, 10, 0).expect("versions");
    assert_eq!(
        remaining.len(),
        0,
        "versions hidden after article soft delete"
    );
}

#[tokio::test]
async fn soft_deleted_latest_version_falls_back_to_the_live_latest() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, first_version) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    let second_version = uuid::Uuid::now_v7().to_string();
    crate::repository::version::create_version(
        &state.database,
        &article_id,
        &VersionDraft {
            version_id: second_version.clone(),
            version_number: "2.0.0".to_string(),
            content_hash: pdf_hash(2),
            note: "note".to_string(),
        },
    )
    .expect("v2");

    soft_delete_version(&state.database, &second_version).expect("soft delete v2");

    let view = crate::repository::article::read_article(&state.database, &article_id)
        .expect("read article")
        .expect("article");
    assert_eq!(
        view.latest_version_id, first_version,
        "falls back to live latest"
    );
    assert_eq!(view.latest_version, "1.0.0");
}

#[tokio::test]
async fn soft_deleted_only_version_leaves_no_dangling_latest() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));

    soft_delete_version(&state.database, &version_id).expect("soft delete");

    let view = crate::repository::article::read_article(&state.database, &article_id)
        .expect("read article")
        .expect("article");
    assert!(view.latest_version_id.is_empty(), "no dangling latest id");
    assert!(view.latest_version.is_empty(), "no dangling latest number");
}

#[tokio::test]
async fn versions_of_excludes_soft_deleted_versions_and_reports_has_next() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (article_id, first_version) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    let second_version = uuid::Uuid::now_v7().to_string();
    crate::repository::version::create_version(
        &state.database,
        &article_id,
        &VersionDraft {
            version_id: second_version.clone(),
            version_number: "2.0.0".to_string(),
            content_hash: pdf_hash(2),
            note: "note".to_string(),
        },
    )
    .expect("v2");
    let third_version = uuid::Uuid::now_v7().to_string();
    crate::repository::version::create_version(
        &state.database,
        &article_id,
        &VersionDraft {
            version_id: third_version.clone(),
            version_number: "3.0.0".to_string(),
            content_hash: pdf_hash(3),
            note: "note".to_string(),
        },
    )
    .expect("v3");

    soft_delete_version(&state.database, &second_version).expect("soft delete v2");

    let (page, has_next) = versions_of(&state.database, &article_id, 1, 0).expect("page one");
    assert_eq!(page.len(), 1, "page one has one live version");
    assert!(has_next, "second live version exists");
    assert!(
        page.iter().all(|item| item.id != second_version),
        "deleted version excluded"
    );

    let (page_two, has_next) = versions_of(&state.database, &article_id, 1, 1).expect("page two");
    assert_eq!(page_two.len(), 1);
    assert!(!has_next, "no further live versions");
    let mut seen: Vec<&str> = page
        .iter()
        .chain(&page_two)
        .map(|item| item.id.as_str())
        .collect();
    seen.sort_unstable();
    let mut live: Vec<&str> = vec![first_version.as_str(), third_version.as_str()];
    live.sort_unstable();
    assert_eq!(seen, live, "pages tile exactly the live versions");
}

#[tokio::test]
async fn read_version_stays_available_at_repository_layer_for_a_soft_deleted_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));

    soft_delete_version(&state.database, &version_id).expect("soft delete");

    assert!(
        crate::repository::version::read_version(&state.database, &version_id)
            .expect("read version")
            .is_some(),
        "row stays available at the repository layer; visibility gating lives in logic"
    );
}

#[tokio::test]
async fn comment_page_hides_soft_deleted_comments_and_their_replies() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    let top = insert_comment(&state, &version_id, &author_id);
    let _reply = insert_reply(&state, &top, &author_id);

    soft_delete_comment(&state.database, &top).expect("soft delete top");

    let (page, has_next) = crate::repository::comment::read_comments_page_by_version(
        &state.database,
        &version_id,
        10,
        0,
    )
    .expect("comment page");
    assert!(page.is_empty(), "deleted top-level comment hidden");
    assert!(!has_next);
    let children_error =
        crate::repository::comment::read_comment_children_page(&state.database, &top, 10, 0)
            .expect_err("children page hidden with deleted parent");
    assert!(
        matches!(children_error, database::Error::NotFound { .. }),
        "deleted parent surfaces as not found, got {children_error:?}"
    );
    assert!(
        crate::repository::comment::read_comment_item(&state.database, &top)
            .expect("read top")
            .is_some(),
        "deleted comment row stays available at the repository layer; lists still hide it"
    );
}

#[tokio::test]
async fn comment_page_tiles_around_soft_deleted_comments() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let (_article_id, version_id) = create_article_fixture(&state, &author_id, &pdf_hash(1));
    let first = insert_comment(&state, &version_id, &author_id);
    let second = insert_comment(&state, &version_id, &author_id);
    let third = insert_comment(&state, &version_id, &author_id);

    soft_delete_comment(&state.database, &second).expect("soft delete second");

    let (page_one, has_next) = crate::repository::comment::read_comments_page_by_version(
        &state.database,
        &version_id,
        1,
        0,
    )
    .expect("page one");
    assert_eq!(page_one.len(), 1);
    assert!(has_next, "one of two live comments remains");
    let (page_two, has_next) = crate::repository::comment::read_comments_page_by_version(
        &state.database,
        &version_id,
        1,
        1,
    )
    .expect("page two");
    assert_eq!(page_two.len(), 1);
    assert!(!has_next);
    let mut seen: Vec<String> = page_one
        .into_iter()
        .chain(page_two)
        .map(|item| item.id)
        .collect();
    seen.sort_unstable();
    let mut live: Vec<String> = vec![first.clone(), third.clone()];
    live.sort_unstable();
    assert_eq!(seen, live, "pages tile exactly the live comments");
}

#[tokio::test]
async fn delete_refresh_keeps_the_semver_latest_version() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author_id = create_user(&state, "alice@example.com");
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_1_0_0 = "ffffffff-ffff-4fff-8fff-ffffffffffff".to_string();
    let version_9_9_9 = "11111111-1111-4111-8111-111111111111".to_string();
    let version_10 = "22222222-2222-4222-8222-222222222222".to_string();
    create_article(
        &state.database,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id,
            title: "Article".to_string(),
            summary: "summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: version_1_0_0.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: pdf_hash(1),
                note: "note".to_string(),
            },
        },
    )
    .expect("create article");
    for (version_id, number, hash_seed) in
        [(&version_9_9_9, "9.9.9", 2), (&version_10, "10.0.0", 3)]
    {
        crate::repository::version::create_version(
            &state.database,
            &article_id,
            &VersionDraft {
                version_id: version_id.clone(),
                version_number: number.to_string(),
                content_hash: pdf_hash(hash_seed),
                note: "note".to_string(),
            },
        )
        .expect("create version");
    }

    delete_version(&state.database, &version_10).expect("delete 10.0.0");

    let view = crate::repository::article::read_article(&state.database, &article_id)
        .expect("read article")
        .expect("article");
    assert_eq!(
        view.latest_version, "9.9.9",
        "semver max survives the delete"
    );
    assert_eq!(
        view.latest_version_id, version_9_9_9,
        "latest id follows the semver max"
    );
}
