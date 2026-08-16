// A0 evidence probes (authz-refactor plan, adopted):
//   1. Cedar missing-attribute semantics on an attribute-less `Virtual` resource
//      is a safe Deny, never an error/panic (source: cedar-policy 4.12.0, pinned
//      crate; probe confirms the U1 design assumption for A1/A3).
//   2. `principal in action` authorizes a member holding `Article::Create`
//      against `Virtual::"article-create"` and denies a non-holder (U1).
//   3. The shipped policy 3 (with `global_role || scopes.containsAny(...)`)
//      denies creates on a Virtual desk, so A1 must simplify it (U1).
//   4. `Schema::actions()` at runtime yields the seed vocabulary (26 actions)
//      and every name parses as an `Action::"..."` UID (U2) — the A5 seeding
//      source is proven.
//   Acceptance questions answered: "is the Virtual-desk design safe?" and "can
//   seeding derive from the parsed schema?" — both yes, per `document/authz-refactor.md` A0.
use std::collections::HashSet;
use std::str::FromStr;

use cedar_policy::{Authorizer, Decision, Entities, Entity, EntityUid, PolicySet, Request};

use crate::infrastructure::cedar::{SCHEMA, action_uid, decide};

fn uid(text: &str) -> EntityUid {
    text.parse::<EntityUid>().expect("entity uid")
}

fn evaluate(
    policy_text: &str,
    principal: EntityUid,
    action: EntityUid,
    resource: EntityUid,
    entities: Vec<Entity>,
) -> Decision {
    let policies = PolicySet::from_str(policy_text).expect("policy set");
    let mut entities = entities;
    if !entities.iter().any(|entity| entity.uid() == action) {
        entities.push(Entity::new_no_attrs(action.clone(), HashSet::new()));
    }
    let entities = Entities::from_entities(entities, None).expect("authorization entities");
    let request = Request::new(
        principal,
        action,
        resource,
        cedar_policy::Context::empty(),
        None,
    )
    .expect("authorization request");
    Authorizer::new()
        .is_authorized(&request, &policies, &entities)
        .decision()
}

#[test]
fn missing_attribute_is_a_safe_deny_not_an_error() {
    let policy = "permit(principal, action, resource) when { resource.owner == principal };";
    let decision = evaluate(
        policy,
        uid("User::\"alice\""),
        uid("Action::\"Article::Create\""),
        uid("Virtual::\"article-create\""),
        vec![Entity::new_no_attrs(uid("User::\"alice\""), HashSet::new())],
    );
    assert_eq!(decision, Decision::Deny);
}

#[test]
fn create_holder_is_allowed_on_the_virtual_desk() {
    let policy = "permit(principal, action, resource) when { principal in action };";
    let member = Entity::new_no_attrs(
        uid("Role::\"member\""),
        HashSet::from([uid("Action::\"Article::Create\"")]),
    );
    let principal = Entity::new_no_attrs(
        uid("User::\"alice\""),
        HashSet::from([uid("Role::\"member\"")]),
    );
    let decision = evaluate(
        policy,
        uid("User::\"alice\""),
        uid("Action::\"Article::Create\""),
        uid("Virtual::\"article-create\""),
        vec![principal, member],
    );
    assert_eq!(decision, Decision::Allow);
}

#[test]
fn create_non_holder_is_denied_on_the_virtual_desk() {
    let policy = "permit(principal, action, resource) when { principal in action };";
    let decision = evaluate(
        policy,
        uid("User::\"bob\""),
        uid("Action::\"Article::Create\""),
        uid("Virtual::\"article-create\""),
        vec![Entity::new_no_attrs(uid("User::\"bob\""), HashSet::new())],
    );
    assert_eq!(decision, Decision::Deny);
}

#[test]
fn current_scope_policy_denies_create_on_the_virtual_desk() {
    let member = Entity::new_no_attrs(
        uid("Role::\"member\""),
        HashSet::from([uid("Action::\"Article::Create\"")]),
    );
    let principal = Entity::new_no_attrs(
        uid("User::\"alice\""),
        HashSet::from([uid("Role::\"member\"")]),
    );
    let allowed = decide(
        &uid("User::\"alice\""),
        "Article::Create",
        &uid("Virtual::\"article-create\""),
        vec![principal, member],
    )
    .expect("decide");
    assert!(
        !allowed,
        "the shipped scope policy must deny creates on a Virtual desk until policy 3 \
         is simplified to `principal in action` (A1)"
    );
}

#[test]
fn schema_actions_equal_seed_vocabulary_and_parse_as_uids() {
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

    assert_eq!(declared.len(), 26, "stale test name says twenty_three");
    assert_eq!(declared, seeded, "schema drift vs seed vocabulary");

    for name in &declared {
        action_uid(name).expect("action name parses as an entity uid");
    }
}
