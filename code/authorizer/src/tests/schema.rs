use std::collections::HashSet;

use crate::ALL_PERMISSIONS;
use crate::authorizer::{POLICY, SCHEMA};

#[test]
fn schema_entity_types_are_fixed_set() {
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema");
    let mut names: Vec<String> = schema
        .entity_types()
        .map(std::string::ToString::to_string)
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "Article", "Comment", "Role", "Tag", "User", "Version", "Virtual"
        ]
    );
}

#[test]
fn action_count_is_39() {
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema");
    assert_eq!(schema.actions().count(), 39);
}

#[test]
fn user_in_role_and_virtual_has_no_attrs() {
    assert!(SCHEMA.contains("entity User in [Role]"));
    assert!(SCHEMA.contains("entity Virtual;"));
    assert!(SCHEMA.contains("entity Tag;"));
}

#[test]
fn all_permissions_equal_the_declared_schema_actions() {
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema");
    let mut declared: Vec<String> = schema
        .actions()
        .map(|action| action.id().unescaped().to_string())
        .collect();
    declared.sort();

    let mut constants: Vec<String> = ALL_PERMISSIONS.iter().map(ToString::to_string).collect();
    constants.sort();

    assert_eq!(declared, constants);
}

#[test]
fn policy_set_validates_against_the_schema() {
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema");
    let policies: cedar_policy::PolicySet = POLICY.parse().expect("policy");
    let result = cedar_policy::Validator::new(schema)
        .validate(&policies, cedar_policy::ValidationMode::Strict);
    let errors: Vec<String> = result
        .validation_errors()
        .map(std::string::ToString::to_string)
        .collect();
    assert!(
        result.validation_passed(),
        "policy does not validate against schema: {errors:?}"
    );
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
