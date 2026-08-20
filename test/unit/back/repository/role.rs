use super::context::{build_state, test_config};

use crate::repository::role::{
    PERMISSION_ARTICLE_CREATE, PERMISSION_ARTICLE_READ, PERMISSION_COMMENT_CREATE,
    PERMISSION_COMMENT_DELETE_TRANSFER, PERMISSION_COMMENT_READ, PERMISSION_ROLE_CREATE,
    PERMISSION_ROLE_DELETE, PERMISSION_ROLE_GRANT, PERMISSION_ROLE_READ, PERMISSION_ROLE_REVOKE,
    PERMISSION_ROLE_UPDATE, PERMISSION_USER_DELETE_TRANSFER, PERMISSION_USER_READ,
    PERMISSION_VERSION_DELETE_HARD, PERMISSION_VERSION_READ, ROLE_MEMBER, ROLE_RECYCLER,
    create_permission, create_role, grant_permission_to_role, hold_role, read_role,
    user_holds_permission, user_holds_role, users_holding_role,
};

#[tokio::test]
async fn create_role_is_idempotent() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let first = create_role(&state.database, "editor").await.expect("create");
    let second = create_role(&state.database, "editor").await.expect("create");
    assert_eq!(first, second);
}

#[tokio::test]
async fn create_permission_and_grant_are_idempotent() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    create_permission(&state.database, "Article::Create")
        .await
        .expect("permission");
    create_permission(&state.database, "Article::Create")
        .await
        .expect("permission");
    create_role(&state.database, "editor").await.expect("role");
    grant_permission_to_role(&state.database, "editor", "Article::Create")
        .await
        .expect("grant");
    grant_permission_to_role(&state.database, "editor", "Article::Create")
        .await
        .expect("grant");
}

#[tokio::test]
async fn hold_role_and_holds_check_agree() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("alice@example.com");
    let user_id = crate::repository::user::create_user(&state.database, &hash)
        .await
        .expect("user");
    create_role(&state.database, "editor").await.expect("role");
    assert!(
        !user_holds_role(&state.database, &user_id, "editor")
            .await
            .expect("check")
    );
    hold_role(&state.database, &user_id, "editor")
        .await
        .expect("hold");
    assert!(
        user_holds_role(&state.database, &user_id, "editor")
            .await
            .expect("check")
    );
}

#[tokio::test]
async fn user_zero_holds_all_required_roles_after_seeding() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("user-zero@example.com");
    let user_id = crate::repository::user::read_user_by_email_address_hash(&state.database, &hash)
        .await
        .expect("lookup")
        .expect("user zero");
    for role_name in crate::repository::role::REQUIRED_ROLES {
        assert!(
            user_holds_role(&state.database, &user_id, role_name)
                .await
                .expect("check")
        );
    }
}

#[tokio::test]
async fn user_holds_permission_is_true_for_a_role_that_grants_it() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("user-zero@example.com");
    let user_id = crate::repository::user::read_user_by_email_address_hash(&state.database, &hash)
        .await
        .expect("lookup")
        .expect("user zero");
    assert!(
        user_holds_permission(&state.database, &user_id, PERMISSION_USER_READ)
            .await
            .expect("check")
    );
}

#[tokio::test]
async fn user_holds_permission_is_false_for_a_plain_member() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = crate::repository::user::create_user(
        &state.database,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    hold_role(&state.database, &user_id, ROLE_MEMBER)
        .await
        .expect("hold");
    assert!(
        !user_holds_permission(&state.database, &user_id, PERMISSION_USER_READ)
            .await
            .expect("check")
    );
}

#[tokio::test]
async fn user_holds_permission_is_false_for_unknown_user_or_permission() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    assert!(
        !user_holds_permission(&state.database, "missing", PERMISSION_USER_READ)
            .await
            .expect("check")
    );
    let hash = nail_common::hash::email("user-zero@example.com");
    let user_id = crate::repository::user::read_user_by_email_address_hash(&state.database, &hash)
        .await
        .expect("lookup")
        .expect("user zero");
    assert!(
        !user_holds_permission(&state.database, &user_id, "No::SuchPermission")
            .await
            .expect("check")
    );
}

#[tokio::test]
async fn users_holding_role_lists_recycler_holders() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("user-zero@example.com");
    let user_zero = crate::repository::user::read_user_by_email_address_hash(&state.database, &hash)
        .await
        .expect("lookup")
        .expect("user zero");
    let recyclers = users_holding_role(&state.database, ROLE_RECYCLER)
        .await
        .expect("list");
    assert_eq!(recyclers, vec![user_zero]);
}

#[tokio::test]
async fn member_role_holds_exactly_the_seeded_baseline_permissions() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let role = read_role(&state.database, ROLE_MEMBER)
        .await
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
    ];
    expected.sort_unstable();

    assert_eq!(actual, expected);
}

#[tokio::test]
async fn every_schema_action_is_seeded_as_a_permission_and_granted_to_admin() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("user-zero@example.com");
    let user_zero = crate::repository::user::read_user_by_email_address_hash(&state.database, &hash)
        .await
        .expect("lookup")
        .expect("user zero");
    let schema: cedar_policy::Schema = crate::infrastructure::cedar::SCHEMA
        .parse()
        .expect("schema");
    for action in schema.actions() {
        let name = action.id().unescaped().to_string();
        assert!(
            user_holds_permission(&state.database, &user_zero, &name)
                .await
                .expect("check"),
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
    assert!(
        !user_holds_role(&state.database, "nonexistent", ROLE_MEMBER)
            .await
            .expect("check")
    );
}

#[tokio::test]
async fn user_holds_role_returns_false_for_unknown_role() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("alice@example.com");
    let user_id = crate::repository::user::create_user(&state.database, &hash)
        .await
        .expect("user");
    assert!(
        !user_holds_role(&state.database, &user_id, "NoSuchRole")
            .await
            .expect("check")
    );
}

#[tokio::test]
async fn users_holding_role_returns_empty_for_unknown_role() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let users = users_holding_role(&state.database, "NoSuchRole")
        .await
        .expect("list");
    assert!(users.is_empty());
}
