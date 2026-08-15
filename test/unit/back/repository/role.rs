use super::context::{build_state, test_config};

use crate::repository::role::{
    PERMISSION_USER_READ, ROLE_MEMBER, ROLE_RECYCLER, apply_tag_to_role, create_permission,
    create_role, grant_permission_to_role, hold_role, read_role, remove_tag_from_role,
    user_holds_permission, user_holds_role, users_holding_role,
};

#[tokio::test]
async fn create_role_is_idempotent() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let first = create_role(&state.graph, "editor").await.expect("create");
    let second = create_role(&state.graph, "editor").await.expect("create");
    assert_eq!(first, second);
}

#[tokio::test]
async fn create_permission_and_grant_are_idempotent() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    create_permission(&state.graph, "Article::Create")
        .await
        .expect("permission");
    create_permission(&state.graph, "Article::Create")
        .await
        .expect("permission");
    create_role(&state.graph, "editor").await.expect("role");
    grant_permission_to_role(&state.graph, "editor", "Article::Create")
        .await
        .expect("grant");
    grant_permission_to_role(&state.graph, "editor", "Article::Create")
        .await
        .expect("grant");
}

#[tokio::test]
async fn hold_role_and_holds_check_agree() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("alice@example.com");
    let user_id = crate::repository::user::create_user(&state.graph, &hash)
        .await
        .expect("user");
    create_role(&state.graph, "editor").await.expect("role");
    assert!(
        !user_holds_role(&state.graph, &user_id, "editor")
            .await
            .expect("check")
    );
    hold_role(&state.graph, &user_id, "editor")
        .await
        .expect("hold");
    assert!(
        user_holds_role(&state.graph, &user_id, "editor")
            .await
            .expect("check")
    );
}

#[tokio::test]
async fn user_zero_holds_all_required_roles_after_seeding() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("user-zero@example.com");
    let user_id = crate::repository::user::read_user_by_email_address_hash(&state.graph, &hash)
        .await
        .expect("lookup")
        .expect("user zero");
    for role_name in crate::repository::role::REQUIRED_ROLES {
        assert!(
            user_holds_role(&state.graph, &user_id, role_name)
                .await
                .expect("check")
        );
    }
}

#[tokio::test]
async fn user_holds_permission_is_true_for_a_role_that_grants_it() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("user-zero@example.com");
    let user_id = crate::repository::user::read_user_by_email_address_hash(&state.graph, &hash)
        .await
        .expect("lookup")
        .expect("user zero");
    assert!(
        user_holds_permission(&state.graph, &user_id, PERMISSION_USER_READ)
            .await
            .expect("check")
    );
}

#[tokio::test]
async fn user_holds_permission_is_false_for_a_plain_member() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let user_id = crate::repository::user::create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    hold_role(&state.graph, &user_id, ROLE_MEMBER)
        .await
        .expect("hold");
    assert!(
        !user_holds_permission(&state.graph, &user_id, PERMISSION_USER_READ)
            .await
            .expect("check")
    );
}

#[tokio::test]
async fn user_holds_permission_is_false_for_unknown_user_or_permission() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    assert!(
        !user_holds_permission(&state.graph, "missing", PERMISSION_USER_READ)
            .await
            .expect("check")
    );
    let hash = nail_common::hash::email("user-zero@example.com");
    let user_id = crate::repository::user::read_user_by_email_address_hash(&state.graph, &hash)
        .await
        .expect("lookup")
        .expect("user zero");
    assert!(
        !user_holds_permission(&state.graph, &user_id, "No::SuchPermission")
            .await
            .expect("check")
    );
}

#[tokio::test]
async fn users_holding_role_lists_recycler_holders() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let hash = nail_common::hash::email("user-zero@example.com");
    let user_zero = crate::repository::user::read_user_by_email_address_hash(&state.graph, &hash)
        .await
        .expect("lookup")
        .expect("user zero");
    let recyclers = users_holding_role(&state.graph, ROLE_RECYCLER)
        .await
        .expect("list");
    assert_eq!(recyclers, vec![user_zero]);
}

async fn role_apply_tag_edge_count(
    state: &crate::infrastructure::state::AppState,
    role_name: &str,
) -> usize {
    let guard = state.graph.read().await;
    let role_node = crate::repository::graph::resolve_node_id_sync(
        &guard,
        crate::repository::schema::ENTITY_TYPE_ROLE,
        role_name,
    )
    .expect("role lookup")
    .expect("role exists");
    let edges = guard
        .exec(
            agdb::QueryBuilder::search()
                .from(role_node)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(crate::repository::schema::KEY_TYPE)
                .value(crate::repository::schema::EDGE_ROLE_APPLY_TAG)
                .query(),
        )
        .expect("role_apply_tag edge query");
    edges.elements.len()
}

#[tokio::test]
async fn role_tag_scopes_apply_read_and_remove() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    create_role(&state.graph, "editor").await.expect("role");
    apply_tag_to_role(&state.graph, "editor", "rust")
        .await
        .expect("apply rust");
    apply_tag_to_role(&state.graph, "editor", "db")
        .await
        .expect("apply db");

    let role = read_role(&state.graph, "editor")
        .await
        .expect("read")
        .expect("role");
    let mut scopes = role.scopes.clone();
    scopes.sort();
    assert_eq!(scopes, vec!["db", "rust"]);
    assert!(role.permissions.is_empty());

    apply_tag_to_role(&state.graph, "editor", "rust")
        .await
        .expect("apply rust again");
    assert_eq!(role_apply_tag_edge_count(&state, "editor").await, 2);

    remove_tag_from_role(&state.graph, "editor", "rust")
        .await
        .expect("remove rust");
    let role = read_role(&state.graph, "editor")
        .await
        .expect("read")
        .expect("role");
    assert_eq!(role.scopes, vec!["db"]);
    assert_eq!(role_apply_tag_edge_count(&state, "editor").await, 1);
}
