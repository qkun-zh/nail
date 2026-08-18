use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use cedar_policy::{Entity, EntityUid, RestrictedExpression};

use crate::infrastructure::cedar::{POLICY, SCHEMA, decide};

fn uid(text: &str) -> EntityUid {
    text.parse::<EntityUid>().expect("entity uid")
}

fn expression(text: &str) -> RestrictedExpression {
    RestrictedExpression::from_str(text).expect("expression")
}

fn user_entity(id: &str, parents: HashSet<EntityUid>) -> Entity {
    Entity::new_no_attrs(uid(&format!("User::\"{id}\"")), parents)
}

fn article_entity(id: &str, owner: &str) -> Entity {
    Entity::new(
        uid(&format!("Article::\"{id}\"")),
        HashMap::from([(
            "owner".to_string(),
            expression(&format!("User::\"{owner}\"")),
        )]),
        HashSet::new(),
    )
    .expect("article entity")
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

#[test]
fn schema_actions_equal_the_permission_constants() {
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema");
    let mut declared: Vec<String> = schema
        .actions()
        .map(|action| action.id().unescaped().to_string())
        .collect();
    declared.sort();

    let mut constants: Vec<String> = crate::repository::role::permission_vocabulary()
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    constants.sort();

    assert_eq!(declared, constants);
}

#[test]
fn policy_set_validates_against_the_schema() {
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema");
    let pset: cedar_policy::PolicySet = POLICY.parse().expect("policy");
    let result =
        cedar_policy::Validator::new(schema).validate(&pset, cedar_policy::ValidationMode::Strict);
    let errors: Vec<String> = result
        .validation_errors()
        .map(std::string::ToString::to_string)
        .collect();
    assert!(
        result.validation_passed(),
        "policy does not validate against schema: {errors:?}"
    );
}

fn policy_action_names() -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = POLICY;
    while let Some(start) = rest.find("Action::\"") {
        rest = &rest[start + "Action::\"".len()..];
        match rest.find('"') {
            Some(end) => {
                names.push(rest[..end].to_string());
                rest = &rest[end + 1..];
            }
            None => break,
        }
    }
    names.sort();
    names.dedup();
    names
}

#[test]
fn every_action_referenced_by_policy_exists_in_the_schema() {
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema");
    let declared: HashSet<String> = schema
        .actions()
        .map(|action| action.id().unescaped().to_string())
        .collect();

    let missing: Vec<String> = policy_action_names()
        .into_iter()
        .filter(|name| !declared.contains(name))
        .collect();

    assert!(
        missing.is_empty(),
        "policy references actions missing from schema.cedar: {missing:?}"
    );
}

#[test]
fn generated_route_constants_match_their_literal_paths() {
    use crate::interface::router::{
        ROUTE_ARTICLE_ID_VERSION_VERSION_ID_CONTENT_READ, ROUTE_CHALLENGE_CREATE,
        ROUTE_COMMENT_ID_DELETE, ROUTE_USER_ID_READ, ROUTE_VERSION_ID_COMMENTS_CREATE,
    };
    assert_eq!(ROUTE_CHALLENGE_CREATE, "/challenge/create");
    assert_eq!(ROUTE_USER_ID_READ, "/user/{id}/read");
    assert_eq!(
        ROUTE_ARTICLE_ID_VERSION_VERSION_ID_CONTENT_READ,
        "/article/{id}/version/{version_id}/content/read"
    );
    assert_eq!(
        ROUTE_VERSION_ID_COMMENTS_CREATE,
        "/version/{id}/comments/create"
    );
    assert_eq!(ROUTE_COMMENT_ID_DELETE, "/comment/{id}/delete");
}

#[test]
fn read_requires_a_role_grant() {
    let member_role = Entity::new_no_attrs(
        uid("Role::\"member\""),
        HashSet::from([uid("Action::\"Article::Read\"")]),
    );
    let action = Entity::new_no_attrs(uid("Action::\"Article::Read\""), HashSet::new());
    let member = user_entity("alice", HashSet::from([uid("Role::\"member\"")]));
    let article = article_entity("article-1", "bob");

    assert!(
        decide(
            &uid("User::\"alice\""),
            "Article::Read",
            &uid("Article::\"article-1\""),
            vec![member, member_role.clone(), action.clone(), article.clone()],
        )
        .expect("member read")
    );

    let grantless = user_entity("carol", HashSet::new());
    assert!(
        !decide(
            &uid("User::\"carol\""),
            "Article::Read",
            &uid("Article::\"article-1\""),
            vec![grantless, member_role, action, article],
        )
        .expect("grantless read")
    );
}

#[test]
fn user_self_view_allows_anyone_and_other_users_need_a_grant() {
    let alice = user_entity("alice", HashSet::new());
    assert!(
        decide(
            &uid("User::\"alice\""),
            "User::Read",
            &uid("User::\"alice\""),
            vec![alice],
        )
        .expect("self view")
    );

    let alice = user_entity("alice", HashSet::new());
    let bob = user_entity("bob", HashSet::new());
    assert!(
        !decide(
            &uid("User::\"alice\""),
            "User::Read",
            &uid("User::\"bob\""),
            vec![alice, bob.clone()],
        )
        .expect("other view denied")
    );

    let admin_role = Entity::new_no_attrs(
        uid("Role::\"admin\""),
        HashSet::from([uid("Action::\"User::Read\"")]),
    );
    let action = Entity::new_no_attrs(uid("Action::\"User::Read\""), HashSet::new());
    let admin = user_entity("admin", HashSet::from([uid("Role::\"admin\"")]));
    assert!(
        decide(
            &uid("User::\"admin\""),
            "User::Read",
            &uid("User::\"bob\""),
            vec![admin, admin_role, action, bob],
        )
        .expect("granted other view")
    );
}

#[test]
fn owner_bypass_allows_update_but_not_for_a_non_owner() {
    let principal = user_entity("alice", HashSet::new());
    let resource = article_entity("article-1", "alice");

    assert!(
        decide(
            &uid("User::\"alice\""),
            "Article::Update",
            &uid("Article::\"article-1\""),
            vec![principal.clone(), resource.clone()],
        )
        .expect("owner update")
    );

    let outsider = user_entity("bob", HashSet::new());
    assert!(
        !decide(
            &uid("User::\"bob\""),
            "Article::Update",
            &uid("Article::\"article-1\""),
            vec![outsider, resource],
        )
        .expect("outsider update")
    );
}

#[test]
fn role_permission_grants_via_principal_in_action() {
    let editor = Entity::new_no_attrs(
        uid("Role::\"editor\""),
        HashSet::from([uid("Action::\"Article::Update\"")]),
    );
    let action = Entity::new_no_attrs(uid("Action::\"Article::Update\""), HashSet::new());
    let principal = user_entity("alice", HashSet::from([uid("Role::\"editor\"")]));

    let article = article_entity("article-1", "bob");
    assert!(
        decide(
            &uid("User::\"alice\""),
            "Article::Update",
            &uid("Article::\"article-1\""),
            vec![principal.clone(), editor.clone(), action.clone(), article],
        )
        .expect("holder update")
    );

    let non_holder = user_entity("bob", HashSet::new());
    assert!(
        !decide(
            &uid("User::\"bob\""),
            "Article::Update",
            &uid("Article::\"article-1\""),
            vec![non_holder, editor, action],
        )
        .expect("non-holder update")
    );
}

#[test]
fn user_self_deregistration_soft_and_transfer() {
    let alice = user_entity("alice", HashSet::new());
    assert!(
        decide(
            &uid("User::\"alice\""),
            "User::Delete::Soft",
            &uid("User::\"alice\""),
            vec![alice.clone()],
        )
        .expect("self soft delete")
    );
    assert!(
        decide(
            &uid("User::\"alice\""),
            "User::Delete::Transfer",
            &uid("User::\"alice\""),
            vec![alice.clone()],
        )
        .expect("self transfer")
    );

    let bob = user_entity("bob", HashSet::new());
    assert!(
        !decide(
            &uid("User::\"alice\""),
            "User::Delete::Soft",
            &uid("User::\"bob\""),
            vec![alice, bob],
        )
        .expect("other user soft delete denied")
    );
}

#[test]
fn user_undelete_soft_requires_a_grant() {
    let alice = user_entity("alice", HashSet::new());
    assert!(
        !decide(
            &uid("User::\"alice\""),
            "User::Undelete::Soft",
            &uid("User::\"bob\""),
            vec![alice],
        )
        .expect("member undelete denied")
    );

    let admin_role = Entity::new_no_attrs(
        uid("Role::\"admin\""),
        HashSet::from([uid("Action::\"User::Undelete::Soft\"")]),
    );
    let action = Entity::new_no_attrs(uid("Action::\"User::Undelete::Soft\""), HashSet::new());
    let admin = user_entity("admin", HashSet::from([uid("Role::\"admin\"")]));
    let bob = user_entity("bob", HashSet::new());
    assert!(
        decide(
            &uid("User::\"admin\""),
            "User::Undelete::Soft",
            &uid("User::\"bob\""),
            vec![admin, admin_role, action, bob],
        )
        .expect("admin undelete")
    );
}

#[test]
fn user_create_is_permitted_for_the_anonymous_principal() {
    let anonymous = user_entity("anonymous", HashSet::new());
    assert!(
        decide(
            &uid("User::\"anonymous\""),
            "User::Create",
            &uid("Virtual::\"user-create\""),
            vec![anonymous],
        )
        .expect("anonymous registration")
    );
}

#[test]
fn role_crud_requires_the_admin_console_and_grant_revoke_the_role_resource() {
    let admin_role = Entity::new_no_attrs(
        uid("Role::\"admin\""),
        HashSet::from([
            uid("Action::\"Role::Create\""),
            uid("Action::\"Role::Read\""),
            uid("Action::\"Role::Update\""),
            uid("Action::\"Role::Delete\""),
        ]),
    );
    let admin = user_entity("admin", HashSet::from([uid("Role::\"admin\"")]));
    let action_entity = Entity::new_no_attrs(uid("Action::\"Role::Create\""), HashSet::new());
    assert!(
        decide(
            &uid("User::\"admin\""),
            "Role::Create",
            &uid("Virtual::\"role-console\""),
            vec![admin.clone(), admin_role.clone(), action_entity],
        )
        .expect("admin role create")
    );
    for action in ["Role::Read", "Role::Update", "Role::Delete"] {
        let action_entity =
            Entity::new_no_attrs(uid(&format!("Action::\"{action}\"")), HashSet::new());
        assert!(
            decide(
                &uid("User::\"admin\""),
                action,
                &uid("Role::\"editor\""),
                vec![admin.clone(), admin_role.clone(), action_entity],
            )
            .expect("admin role crud")
        );
    }

    let grant_role = Entity::new_no_attrs(
        uid("Role::\"admin\""),
        HashSet::from([
            uid("Action::\"Role::Grant\""),
            uid("Action::\"Role::Revoke\""),
        ]),
    );
    for action in ["Role::Grant", "Role::Revoke"] {
        let action_entity =
            Entity::new_no_attrs(uid(&format!("Action::\"{action}\"")), HashSet::new());
        assert!(
            decide(
                &uid("User::\"admin\""),
                action,
                &uid("Role::\"editor\""),
                vec![admin.clone(), grant_role.clone(), action_entity],
            )
            .expect("admin role grant/revoke")
        );
    }

    let member = user_entity("alice", HashSet::from([uid("Role::\"member\"")]));
    assert!(
        !decide(
            &uid("User::\"alice\""),
            "Role::Read",
            &uid("Virtual::\"admin-console\""),
            vec![member],
        )
        .expect("member role read denied")
    );
}

#[test]
fn admin_without_a_grant_is_denied() {
    let admin = Entity::new_no_attrs(uid("Role::\"admin\""), HashSet::new());
    let principal = user_entity("alice", HashSet::from([uid("Role::\"admin\"")]));
    let resource = user_entity("bob", HashSet::new());

    assert!(
        !decide(
            &uid("User::\"alice\""),
            "User::Delete::Hard",
            &uid("User::\"bob\""),
            vec![principal, admin, resource],
        )
        .expect("admin without grant")
    );
}

#[test]
fn admin_holding_a_grant_is_allowed() {
    let admin = Entity::new_no_attrs(
        uid("Role::\"admin\""),
        HashSet::from([uid("Action::\"User::Delete::Hard\"")]),
    );
    let principal = user_entity("alice", HashSet::from([uid("Role::\"admin\"")]));
    let resource = user_entity("bob", HashSet::new());

    assert!(
        decide(
            &uid("User::\"alice\""),
            "User::Delete::Hard",
            &uid("User::\"bob\""),
            vec![principal, admin, resource],
        )
        .expect("admin with grant")
    );
}
