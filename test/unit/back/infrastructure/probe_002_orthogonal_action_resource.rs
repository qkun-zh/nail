// Probe 002 — Cedar action/resource orthogonality + DB-owned owner attribute.
//
// Purpose: pin the claim that in Cedar an action and a resource are orthogonal
//   dimensions. The same action (`Article::Read`) must decide correctly across
//   different resource kinds (Article, Version, Comment), and the owner check
//   (`resource.owner == principal`) is satisfied only by the *resource entity's*
//   `owner` attribute — which in nail is assembled from the graph DB by
//   `repository/authorization.rs::assemble_resource`, never derivable from the
//   action string alone.
//
// Source evidence: `cedar-policy-4.12.0/src/api.rs` (`Authorizer::is_authorized`,
//   `Request::new`, `Decision`); project `schema.cedar` line `action
//   "Article::Read" appliesTo { principal: [User], resource: [Article, Version,
//   Comment] }`; project `repository/authorization.rs` (`Resource` enum +
//   `assemble_resource` filling `owner` and the Version->Article / Comment->
//   Version->Article chains).
//
// Acceptance question: "is a design where the authorization layer knows only an
//   action (e.g. `Authorized<ArticleRead>` deriving its resource from the action)
//   unsound, because it ignores the DB-assembled resource (owner attribute +
//   inheritance chain)?" Expect: yes — one action, three resource kinds, and the
//   decision turns on each resource entity's DB-derived `owner`.
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use cedar_policy::{
    Authorizer, Decision, Entities, Entity, EntityUid, PolicySet, Request, RestrictedExpression,
};

fn uid(text: &str) -> EntityUid {
    text.parse::<EntityUid>().expect("entity uid")
}

fn expr(text: &str) -> RestrictedExpression {
    RestrictedExpression::from_str(text).expect("restricted expression")
}

fn decide(
    policy_text: &str,
    principal: EntityUid,
    action: &str,
    resource: EntityUid,
    mut entities: Vec<Entity>,
) -> Decision {
    let policies = PolicySet::from_str(policy_text).expect("policy set");
    let action_uid = uid(&format!("Action::\"{action}\""));
    if !entities.iter().any(|entity| entity.uid() == action_uid) {
        entities.push(Entity::new_no_attrs(action_uid.clone(), HashSet::new()));
    }
    let entities = Entities::from_entities(entities, None).expect("authorization entities");
    let request = Request::new(
        principal,
        action_uid,
        resource,
        cedar_policy::Context::empty(),
        None,
    )
    .expect("authorization request");
    Authorizer::new()
        .is_authorized(&request, &policies, &entities)
        .decision()
}

fn owned_entity(ty: &str, id: &str, owner: &str, parents: HashSet<EntityUid>) -> Entity {
    Entity::new(
        uid(&format!("{ty}::\"{id}\"")),
        HashMap::from([("owner".to_string(), expr(&format!("User::\"{owner}\"")))]),
        parents,
    )
    .expect("owned entity")
}

#[test]
fn one_action_decides_across_three_resource_kinds() {
    let policy = r#"
        permit (principal, action in [Action::"Article::Read"], resource)
          when { resource.owner == principal };
    "#;

    let owner_entity = Entity::new_no_attrs(uid("User::\"alice\""), HashSet::new());

    // The same action: Article::Read. Each resource is a different Cedar entity
    // carrying its own `owner` attr — as `assemble_resource` produces from the DB.
    let article = owned_entity("Article", "a1", "alice", HashSet::new());
    let version = owned_entity(
        "Version",
        "v1",
        "alice",
        HashSet::from([uid("Article::\"a1\"")]),
    );
    let comment = owned_entity(
        "Comment",
        "c1",
        "alice",
        HashSet::from([uid("Version::\"v1\"")]),
    );

    let cases: Vec<(EntityUid, Vec<Entity>)> = vec![
        (
            uid("Article::\"a1\""),
            vec![owner_entity.clone(), article.clone()],
        ),
        (
            uid("Version::\"v1\""),
            vec![owner_entity.clone(), article.clone(), version.clone()],
        ),
        (
            uid("Comment::\"c1\""),
            vec![
                owner_entity.clone(),
                article.clone(),
                version.clone(),
                comment.clone(),
            ],
        ),
    ];

    for (resource_uid, entities) in cases {
        let decision = decide(
            policy,
            uid("User::\"alice\""),
            "Article::Read",
            resource_uid.clone(),
            entities,
        );
        assert_eq!(
            decision,
            Decision::Allow,
            "action=Article::Read must Allow on owned resource {resource_uid}"
        );
    }
}

#[test]
fn a_non_owner_is_denied_across_all_kinds() {
    let policy = r#"
        permit (principal, action in [Action::"Article::Read"], resource)
          when { resource.owner == principal };
    "#;

    let outsider = Entity::new_no_attrs(uid("User::\"bob\""), HashSet::new());
    let article = owned_entity("Article", "a1", "alice", HashSet::new());
    let version = owned_entity(
        "Version",
        "v1",
        "alice",
        HashSet::from([uid("Article::\"a1\"")]),
    );
    let comment = owned_entity(
        "Comment",
        "c1",
        "alice",
        HashSet::from([uid("Version::\"v1\"")]),
    );

    let cases: Vec<(EntityUid, Vec<Entity>)> = vec![
        (
            uid("Article::\"a1\""),
            vec![outsider.clone(), article.clone()],
        ),
        (
            uid("Version::\"v1\""),
            vec![outsider.clone(), article.clone(), version.clone()],
        ),
        (
            uid("Comment::\"c1\""),
            vec![
                outsider.clone(),
                article.clone(),
                version.clone(),
                comment.clone(),
            ],
        ),
    ];

    for (resource_uid, entities) in cases {
        let decision = decide(
            policy,
            uid("User::\"bob\""),
            "Article::Read",
            resource_uid.clone(),
            entities,
        );
        assert_eq!(
            decision,
            Decision::Deny,
            "action=Article::Read must Deny non-owner on {resource_uid}"
        );
    }
}
