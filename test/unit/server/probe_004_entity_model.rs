use std::collections::HashSet;

use crate::infrastructure::cedar::SCHEMA;

#[test]
fn schema_entity_types_are_fixed_set() {
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema");
    let mut names: Vec<String> = schema.entity_types().map(|ty| ty.to_string()).collect();
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
    let count = schema.actions().count();
    assert_eq!(count, 39);
}

#[test]
fn user_in_role_and_virtual_has_no_attrs() {
    let schema: cedar_policy::Schema = SCHEMA.parse().expect("schema");
    // User entity has Role as ancestor, validated via schema text; probe by checking principal hierarchy at runtime elsewhere.
    // Here we ensure Virtual and Tag carry no owner attribute via cedar probe (attribute-less).
    // Cedar authorizer allows attribute-less entity without panic (tested in cedar_probe).
    let _needs = HashSet::<String>::new();
    assert!(SCHEMA.contains("entity User in [Role]"));
    assert!(SCHEMA.contains("entity Virtual;"));
    assert!(SCHEMA.contains("entity Tag;"));
}
