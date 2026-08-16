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
fn schema_actions_equal_the_seed_vocabulary() {
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema");
    let mut declared: Vec<String> = schema
        .actions()
        .map(|action| action.id().unescaped().to_string())
        .collect();
    declared.sort();

    let mut seeded: Vec<String> = crate::repository::role::ALL_PERMISSIONS
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    seeded.sort();

    assert_eq!(declared, seeded);
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
fn read_open_policy_allows_any_authenticated_principal() {
    let principal = user_entity("alice", HashSet::new());
    let resource = article_entity("article-1", "bob");

    assert!(
        decide(
            &uid("User::\"alice\""),
            "Article::Read",
            &uid("Article::\"article-1\""),
            vec![principal, resource],
        )
        .expect("decide")
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
fn admin_role_allows_everything() {
    let admin = Entity::new_no_attrs(uid("Role::\"admin\""), HashSet::new());
    let principal = user_entity("alice", HashSet::from([uid("Role::\"admin\"")]));
    let resource = article_entity("article-1", "bob");

    assert!(
        decide(
            &uid("User::\"alice\""),
            "User::Delete::Hard",
            &uid("System::\"admin-console\""),
            vec![principal.clone(), admin.clone(), resource.clone()],
        )
        .expect("admin all")
    );

    assert!(
        decide(
            &uid("User::\"alice\""),
            "Role::Manage",
            &uid("System::\"admin-console\""),
            vec![principal, admin, resource],
        )
        .expect("admin role manage")
    );
}
