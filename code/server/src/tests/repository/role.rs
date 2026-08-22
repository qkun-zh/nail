use super::context::{build_state, test_config};

use crate::repository::role::{
    PERMISSION_ARTICLE_CREATE, PERMISSION_ARTICLE_READ, PERMISSION_COMMENT_CREATE,
    PERMISSION_COMMENT_DELETE_TRANSFER, PERMISSION_COMMENT_READ, PERMISSION_ROLE_CREATE,
    PERMISSION_ROLE_DELETE, PERMISSION_ROLE_GRANT, PERMISSION_ROLE_READ, PERMISSION_ROLE_REVOKE,
    PERMISSION_ROLE_UPDATE, PERMISSION_TAG_APPLY, PERMISSION_TAG_CREATE, PERMISSION_TAG_READ,
    PERMISSION_TAG_UNAPPLY, PERMISSION_USER_DELETE_TRANSFER, PERMISSION_USER_READ,
    PERMISSION_VERSION_DELETE_HARD, PERMISSION_VERSION_READ, ROLE_MEMBER, ROLE_RECYCLER,
    create_permission, create_role, grant_permission_to_role, hold_role, read_role,
    user_holds_permission, user_holds_role, users_holding_role,
};

#[tokio::test]
async fn create_role_is_idempotent() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let first = create_role(&state.database, "editor").expect("create");
    let second = create_role(&state.database, "editor").expect("create");
    assert_eq!(first, second);
}

#[tokio::test]
async fn create_permission_and_grant_are_idempotent() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    create_permission(&state.database, "Article::Create").expect("permission");
    create_permission(&state.database, "Article::Create").expect("permission");
    create_role(&state.database, "editor").expect("role");
    grant_permission_to_role(&state.database, "editor", "Article::Create").expect("grant");
    grant_permission_to_role(&state.database, "editor", "Article::Create").expect("grant");
}

#[tokio::test]
async fn hold_role_and_holds_check_agree() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed");
    let user_id = crate::repository::user::create_user(&state.database, &hash).expect("user");
    create_role(&state.database, "editor").expect("role");
    assert!(!user_holds_role(&state.database, &user_id, "editor").expect("check"));
    hold_role(&state.database, &user_id, "editor").expect("hold");
    assert!(user_holds_role(&state.database, &user_id, "editor").expect("check"));
}

#[tokio::test]
async fn user_zero_holds_all_required_roles_after_seeding() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = common::hash::hash("user-zero@example.com".as_bytes()).expect("hash must succeed");
    let user_id = crate::repository::user::read_user_by_email_address_hash(&state.database, &hash)
        .expect("lookup")
        .expect("user zero");
    for role_name in crate::repository::role::REQUIRED_ROLES {
        assert!(user_holds_role(&state.database, &user_id, role_name).expect("check"));
    }
}

#[tokio::test]
async fn user_holds_permission_is_true_for_a_role_that_grants_it() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = common::hash::hash("user-zero@example.com".as_bytes()).expect("hash must succeed");
    let user_id = crate::repository::user::read_user_by_email_address_hash(&state.database, &hash)
        .expect("lookup")
        .expect("user zero");
    assert!(user_holds_permission(&state.database, &user_id, PERMISSION_USER_READ).expect("check"));
}

#[tokio::test]
async fn user_holds_permission_is_false_for_a_plain_member() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = crate::repository::user::create_user(
        &state.database,
        &common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed"),
    )
    .expect("user");
    hold_role(&state.database, &user_id, ROLE_MEMBER).expect("hold");
    assert!(
        !user_holds_permission(&state.database, &user_id, PERMISSION_USER_READ).expect("check")
    );
}

#[tokio::test]
async fn user_holds_permission_is_false_for_unknown_user_or_permission() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    assert!(
        !user_holds_permission(&state.database, "missing", PERMISSION_USER_READ).expect("check")
    );
    let hash = common::hash::hash("user-zero@example.com".as_bytes()).expect("hash must succeed");
    let user_id = crate::repository::user::read_user_by_email_address_hash(&state.database, &hash)
        .expect("lookup")
        .expect("user zero");
    assert!(
        !user_holds_permission(&state.database, &user_id, "No::SuchPermission").expect("check")
    );
}

#[tokio::test]
async fn users_holding_role_lists_recycler_holders() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = common::hash::hash("user-zero@example.com".as_bytes()).expect("hash must succeed");
    let user_zero =
        crate::repository::user::read_user_by_email_address_hash(&state.database, &hash)
            .expect("lookup")
            .expect("user zero");
    let recyclers = users_holding_role(&state.database, ROLE_RECYCLER).expect("list");
    assert_eq!(recyclers, vec![user_zero]);
}

#[tokio::test]
async fn member_role_holds_exactly_the_seeded_baseline_permissions() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let role = read_role(&state.database, ROLE_MEMBER)
        .expect("read")
        .expect("member role");

    let mut actual: Vec<&str> = role.permissions.iter().map(String::as_str).collect();
    actual.sort_unstable();
    let mut expected: Vec<&str> = vec![
        PERMISSION_ARTICLE_CREATE,
        PERMISSION_COMMENT_CREATE,
        PERMISSION_ARTICLE_READ,
        PERMISSION_VERSION_READ,
        PERMISSION_COMMENT_READ,
        PERMISSION_TAG_READ,
        PERMISSION_TAG_CREATE,
        PERMISSION_TAG_APPLY,
        PERMISSION_TAG_UNAPPLY,
    ];
    expected.sort_unstable();

    assert_eq!(actual, expected);
}

#[tokio::test]
async fn every_schema_action_is_seeded_as_a_permission_and_granted_to_admin() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = common::hash::hash("user-zero@example.com".as_bytes()).expect("hash must succeed");
    let user_zero =
        crate::repository::user::read_user_by_email_address_hash(&state.database, &hash)
            .expect("lookup")
            .expect("user zero");
    for name in authorizer::ALL_PERMISSIONS {
        assert!(
            user_holds_permission(&state.database, &user_zero, name).expect("check"),
            "admin must hold every schema action: {name}"
        );
    }
}

#[test]
fn generated_permission_constants_have_expected_names() {
    assert_eq!(PERMISSION_ARTICLE_CREATE, "Article::Create");
    assert_eq!(PERMISSION_VERSION_DELETE_HARD, "Version::Delete::Hard");
    assert_eq!(
        PERMISSION_COMMENT_DELETE_TRANSFER,
        "Comment::Delete::Transfer"
    );
    assert_eq!(PERMISSION_USER_READ, "User::Read");
    assert_eq!(PERMISSION_ROLE_CREATE, "Role::Create");
    assert_eq!(PERMISSION_ROLE_READ, "Role::Read");
    assert_eq!(PERMISSION_ROLE_UPDATE, "Role::Update");
    assert_eq!(PERMISSION_ROLE_DELETE, "Role::Delete");
    assert_eq!(PERMISSION_ROLE_GRANT, "Role::Grant");
    assert_eq!(PERMISSION_ROLE_REVOKE, "Role::Revoke");
    assert_eq!(PERMISSION_USER_DELETE_TRANSFER, "User::Delete::Transfer");
}

#[tokio::test]
async fn user_holds_role_returns_false_for_unknown_user() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    assert!(!user_holds_role(&state.database, "nonexistent", ROLE_MEMBER).expect("check"));
}

#[tokio::test]
async fn user_holds_role_returns_false_for_unknown_role() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = common::hash::hash("alice@example.com".as_bytes()).expect("hash must succeed");
    let user_id = crate::repository::user::create_user(&state.database, &hash).expect("user");
    assert!(!user_holds_role(&state.database, &user_id, "NoSuchRole").expect("check"));
}

#[tokio::test]
async fn users_holding_role_returns_empty_for_unknown_role() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let users = users_holding_role(&state.database, "NoSuchRole").expect("list");
    assert!(users.is_empty());
}

#[test]
fn article_restore_is_renamed_to_undelete_soft() {
    let vocabulary = crate::repository::role::permission_vocabulary();
    assert!(vocabulary.contains(&"Article::Undelete::Soft"));
    assert!(!vocabulary.contains(&"Article::Restore"));
}

#[test]
fn version_restore_is_renamed_to_undelete_soft() {
    let vocabulary = crate::repository::role::permission_vocabulary();
    assert!(vocabulary.contains(&"Version::Undelete::Soft"));
    assert!(!vocabulary.contains(&"Version::Restore"));
}

#[test]
fn comment_restore_is_renamed_to_undelete_soft() {
    let vocabulary = crate::repository::role::permission_vocabulary();
    assert!(vocabulary.contains(&"Comment::Undelete::Soft"));
    assert!(!vocabulary.contains(&"Comment::Restore"));
}
