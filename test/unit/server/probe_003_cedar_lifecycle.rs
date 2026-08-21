use std::collections::HashSet;

use cedar_policy::{
    Authorizer as CedarAuthorizer, Decision, Entities, Entity, EntityUid, PolicySet, Request,
    ValidationMode, Validator,
};

use crate::infrastructure::cedar::{POLICY, SCHEMA};

fn uid(text: &str) -> EntityUid {
    text.parse::<EntityUid>().expect("uid")
}

#[test]
fn policy_and_schema_parse_and_validate_once() {
    let pset: PolicySet = POLICY.parse().expect("policy parse");
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema parse");
    let v = Validator::new(schema).validate(&pset, ValidationMode::Strict);
    assert!(v.validation_passed());
}

#[test]
fn malformed_policy_returns_error_not_panic() {
    let bad = "permit(principal, action, resource) when { resource.owner == }";
    let err = bad.parse::<PolicySet>().err().expect("must fail");
    assert!(!err.to_string().is_empty());
}

#[test]
fn second_authorizer_reuses_same_decision() {
    let pset: PolicySet = POLICY.parse().expect("pset");
    let principal = uid("User::\"alice\"");
    let action = uid("Action::\"User::Create\"");
    let resource = uid("Virtual::\"user-create\"");
    let entities = Entities::from_entities(
        vec![
            Entity::new_no_attrs(principal.clone(), HashSet::new()),
            Entity::new_no_attrs(action.clone(), HashSet::new()),
        ],
        None,
    )
    .expect("entities");
    let req = Request::new(
        principal.clone(),
        action.clone(),
        resource.clone(),
        cedar_policy::Context::empty(),
        None,
    )
    .expect("req");
    let d1 = CedarAuthorizer::new()
        .is_authorized(&req, &pset, &entities)
        .decision();
    let d2 = CedarAuthorizer::new()
        .is_authorized(&req, &pset, &entities)
        .decision();
    assert_eq!(d1, Decision::Allow);
    assert_eq!(d1, d2);
}
