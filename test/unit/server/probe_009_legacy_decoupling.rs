use crate::infrastructure::cedar::{POLICY, SCHEMA};
use cedar_policy::{
    Authorizer as CedarAuthorizer, Decision, Entities, Entity, EntityUid, PolicySet, Request,
    RestrictedExpression,
};
use std::collections::HashSet;
use std::str::FromStr;

fn uid(s: &str) -> EntityUid {
    s.parse().expect("uid")
}
#[allow(dead_code)]
fn expr(s: &str) -> RestrictedExpression {
    RestrictedExpression::from_str(s).expect("expr")
}

#[test]
fn authorizer_validates_without_database() {
    let pset: PolicySet = POLICY.parse().expect("pset");
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema");
    let v =
        cedar_policy::Validator::new(schema).validate(&pset, cedar_policy::ValidationMode::Strict);
    assert!(v.validation_passed());
}

#[test]
fn snapshot_based_authorize_matches_db_backed() {
    // Simulate snapshot: user alice with member role holding Article::Create, resource Virtual::"any"
    let pset: PolicySet = POLICY.parse().expect("pset");
    let role = Entity::new_no_attrs(
        uid("Role::\"member\""),
        HashSet::from([uid("Action::\"Article::Create\"")]),
    );
    let action = Entity::new_no_attrs(uid("Action::\"Article::Create\""), HashSet::new());
    let principal = Entity::new_no_attrs(
        uid("User::\"alice\""),
        HashSet::from([uid("Role::\"member\"")]),
    );
    let resource = Entity::new_no_attrs(uid("Virtual::\"any\""), HashSet::new());
    let entities = Entities::from_entities(vec![principal.clone(), role, action.clone()], None)
        .expect("entities");
    let req = Request::new(
        principal.uid().clone(),
        uid("Action::\"Article::Create\""),
        uid("Virtual::\"any\""),
        cedar_policy::Context::empty(),
        None,
    )
    .expect("req");
    let decision = CedarAuthorizer::new()
        .is_authorized(&req, &pset, &entities)
        .decision();
    assert_eq!(decision, Decision::Allow);
}

#[test]
fn uid_helper_collapses_scattered_format() {
    let user = format!("User::\"{}\"", "alice");
    assert_eq!(
        user.parse::<EntityUid>().expect("uid").to_string(),
        "User::\"alice\""
    );
    let article = format!("Article::\"{}\"", "id-1");
    assert_eq!(
        article.parse::<EntityUid>().expect("uid").to_string(),
        "Article::\"id-1\""
    );
}
