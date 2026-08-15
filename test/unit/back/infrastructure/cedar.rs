use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use cedar_policy::{Entity, EntityUid, RestrictedExpression};

use crate::infrastructure::cedar::{SCHEMA, decide};

fn uid(text: &str) -> EntityUid {
    text.parse::<EntityUid>().expect("entity uid")
}

fn expression(text: &str) -> RestrictedExpression {
    RestrictedExpression::from_str(text).expect("expression")
}

fn user_entity(
    id: &str,
    parents: HashSet<EntityUid>,
    global_role: bool,
    scopes: &[&str],
) -> Entity {
    let scopes_expr = if scopes.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            scopes
                .iter()
                .map(|scope| format!("Tag::\"{scope}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    Entity::new(
        uid(&format!("User::\"{id}\"")),
        HashMap::from([
            (
                "global_role".to_string(),
                expression(if global_role { "true" } else { "false" }),
            ),
            ("scopes".to_string(), expression(&scopes_expr)),
        ]),
        parents,
    )
    .expect("user entity")
}

fn article_entity(id: &str, owner: &str, scopes: &[&str]) -> Entity {
    let scopes_expr = if scopes.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            scopes
                .iter()
                .map(|scope| format!("Tag::\"{scope}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    Entity::new(
        uid(&format!("Article::\"{id}\"")),
        HashMap::from([
            (
                "owner".to_string(),
                expression(&format!("User::\"{owner}\"")),
            ),
            ("required_scopes".to_string(), expression(&scopes_expr)),
        ]),
        HashSet::new(),
    )
    .expect("article entity")
}

#[test]
fn schema_declares_exactly_the_twenty_seeded_actions() {
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

#[test]
fn read_open_policy_allows_any_authenticated_principal() {
    let principal = user_entity("alice", HashSet::new(), false, &[]);
    let resource = article_entity("article-1", "bob", &[]);

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
    let principal = user_entity("alice", HashSet::new(), false, &[]);
    let resource = article_entity("article-1", "alice", &[]);

    assert!(
        decide(
            &uid("User::\"alice\""),
            "Article::Update",
            &uid("Article::\"article-1\""),
            vec![principal.clone(), resource.clone()],
        )
        .expect("owner update")
    );

    let outsider = user_entity("bob", HashSet::new(), false, &[]);
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
fn role_permission_grants_only_when_the_scope_intersects() {
    let editor = Entity::new_no_attrs(
        uid("Role::\"editor\""),
        HashSet::from([uid("Action::\"Article::Update\"")]),
    );
    let action = Entity::new_no_attrs(uid("Action::\"Article::Update\""), HashSet::new());
    let principal = user_entity(
        "alice",
        HashSet::from([uid("Role::\"editor\"")]),
        false,
        &["rust"],
    );

    let matching = article_entity("article-1", "bob", &["rust"]);
    assert!(
        decide(
            &uid("User::\"alice\""),
            "Article::Update",
            &uid("Article::\"article-1\""),
            vec![principal.clone(), editor.clone(), action.clone(), matching],
        )
        .expect("matching scope")
    );

    let non_matching = article_entity("article-2", "bob", &["other"]);
    assert!(
        !decide(
            &uid("User::\"alice\""),
            "Article::Update",
            &uid("Article::\"article-2\""),
            vec![principal, editor, action, non_matching],
        )
        .expect("non-matching scope")
    );
}

#[test]
fn admin_role_allows_everything() {
    let admin = Entity::new_no_attrs(uid("Role::\"admin\""), HashSet::new());
    let principal = user_entity("alice", HashSet::from([uid("Role::\"admin\"")]), false, &[]);
    let resource = article_entity("article-1", "bob", &[]);

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
