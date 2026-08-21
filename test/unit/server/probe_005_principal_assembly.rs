use super::context::TestCtx;
use crate::repository::authorization::assemble_principal;
use crate::repository::role::{PERMISSION_ARTICLE_READ, PERMISSION_USER_READ};

#[tokio::test]
async fn dedups_shared_permission_across_roles() {
    let ctx = TestCtx::new().await.expect("ctx");
    let user = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"alice@ex.com").expect("hash"),
    )
    .expect("user");
    crate::repository::role::create_role(&ctx.state.database, "r1").expect("r1");
    crate::repository::role::create_role(&ctx.state.database, "r2").expect("r2");
    crate::repository::role::create_permission(&ctx.state.database, PERMISSION_ARTICLE_READ)
        .expect("perm");
    crate::repository::role::grant_permission_to_role(
        &ctx.state.database,
        "r1",
        PERMISSION_ARTICLE_READ,
    )
    .expect("grant1");
    crate::repository::role::grant_permission_to_role(
        &ctx.state.database,
        "r2",
        PERMISSION_ARTICLE_READ,
    )
    .expect("grant2");
    crate::repository::role::hold_role(&ctx.state.database, &user, "r1").expect("hold1");
    crate::repository::role::hold_role(&ctx.state.database, &user, "r2").expect("hold2");
    let (principal, entities) = assemble_principal(&ctx.state.database, &user).expect("assemble");
    let action_count = entities
        .iter()
        .filter(|e| e.uid().to_string().contains("Action"))
        .count();
    assert_eq!(action_count, 1, "shared permission deduped");
    assert!(entities.iter().any(|e| e.uid() == principal));
}

#[tokio::test]
async fn anonymous_yields_single_user_entity() {
    let ctx = TestCtx::new().await.expect("ctx");
    let (principal, entities) =
        assemble_principal(&ctx.state.database, "anonymous").expect("principal");
    assert_eq!(principal.to_string(), "User::\"anonymous\"");
    // only principal entity plus no roles
    assert_eq!(entities.len(), 1);
}

#[tokio::test]
async fn role_perms_become_parents() {
    let ctx = TestCtx::new().await.expect("ctx");
    let user = crate::repository::user::create_user(
        &ctx.state.database,
        &common::hash::hash(b"bob2@ex.com").expect("hash"),
    )
    .expect("user");
    crate::repository::role::create_role(&ctx.state.database, "ed").expect("role");
    crate::repository::role::grant_permission_to_role(
        &ctx.state.database,
        "ed",
        PERMISSION_USER_READ,
    )
    .expect("grant");
    crate::repository::role::hold_role(&ctx.state.database, &user, "ed").expect("hold");
    let (_, entities) = assemble_principal(&ctx.state.database, &user).expect("assemble");
    // Verify via Cedar decision: principal in Action should allow User::Read on self resource via role grant
    let pset: cedar_policy::PolicySet = crate::infrastructure::cedar::POLICY
        .parse()
        .expect("policy");
    let principal_uid: cedar_policy::EntityUid = format!("User::\"{user}\"").parse().expect("uid");
    let action_uid: cedar_policy::EntityUid = "Action::\"User::Read\"".parse().expect("uid");
    let resource_uid: cedar_policy::EntityUid = format!("User::\"{user}\"").parse().expect("uid");
    // build Entities with action entity injected as authorizer does
    let mut all_entities = entities.clone();
    if !all_entities.iter().any(|e| e.uid() == action_uid) {
        all_entities.push(cedar_policy::Entity::new_no_attrs(
            action_uid.clone(),
            std::collections::HashSet::new(),
        ));
    }
    // resource entity — avoid duplicate if principal == resource (self read)
    if !all_entities.iter().any(|e| e.uid() == resource_uid) {
        all_entities.push(cedar_policy::Entity::new_no_attrs(
            resource_uid.clone(),
            std::collections::HashSet::new(),
        ));
    }
    let entities_wrapped =
        cedar_policy::Entities::from_entities(all_entities, None).expect("entities");
    let request = cedar_policy::Request::new(
        principal_uid,
        action_uid,
        resource_uid,
        cedar_policy::Context::empty(),
        None,
    )
    .expect("request");
    let decision = cedar_policy::Authorizer::new()
        .is_authorized(&request, &pset, &entities_wrapped)
        .decision();
    assert_eq!(decision, cedar_policy::Decision::Allow);
}
