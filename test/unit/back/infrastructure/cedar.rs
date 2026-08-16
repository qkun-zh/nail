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

fn user_entity_owned(id: &str) -> Entity {
    Entity::new(
        uid(&format!("User::\"{id}\"")),
        HashMap::from([("owner".to_string(), expression(&format!("User::\"{id}\"")))]),
        HashSet::new(),
    )
    .expect("user entity")
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
    let alice_owned = user_entity_owned("alice");
    assert!(
        decide(
            &uid("User::\"alice\""),
            "User::Read",
            &uid("User::\"alice\""),
            vec![alice_owned],
        )
        .expect("self view")
    );

    let alice = user_entity("alice", HashSet::new());
    let bob_owned = user_entity_owned("bob");
    assert!(
        !decide(
            &uid("User::\"alice\""),
            "User::Read",
            &uid("User::\"bob\""),
            vec![alice, bob_owned.clone()],
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
            vec![admin, admin_role, action, bob_owned],
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
fn admin_without_a_grant_is_denied() {
    let admin = Entity::new_no_attrs(uid("Role::\"admin\""), HashSet::new());
    let principal = user_entity("alice", HashSet::from([uid("Role::\"admin\"")]));
    let resource = article_entity("article-1", "bob");

    assert!(
        !decide(
            &uid("User::\"alice\""),
            "User::Delete::Hard",
            &uid("Virtual::\"admin-console\""),
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
    let resource = article_entity("article-1", "bob");

    assert!(
        decide(
            &uid("User::\"alice\""),
            "User::Delete::Hard",
            &uid("Virtual::\"admin-console\""),
            vec![principal, admin, resource],
        )
        .expect("admin with grant")
    );
}
