use std::sync::OnceLock;

use anyhow::Context;
use cedar_policy::{Authorizer, Decision, Entities, Entity, EntityUid, PolicySet, Request};

#[cfg(test)]
pub const SCHEMA: &str = include_str!("cedar/schema.cedar");
pub const POLICY: &str = include_str!("cedar/policy.cedar");

static POLICY_SET: OnceLock<Result<PolicySet, String>> = OnceLock::new();

fn policies() -> anyhow::Result<&'static PolicySet> {
    let result = POLICY_SET.get_or_init(|| {
        POLICY
            .parse::<PolicySet>()
            .map_err(|error| error.to_string())
    });
    result
        .as_ref()
        .map_err(|error| anyhow::anyhow!("invalid authorization policy: {error}"))
}

pub fn action_uid(action: &str) -> anyhow::Result<EntityUid> {
    format!("Action::\"{action}\"")
        .parse::<EntityUid>()
        .with_context(|| format!("invalid action uid for {action:?}"))
}

pub fn decide(
    principal: &EntityUid,
    action: &str,
    resource: &EntityUid,
    mut entities: Vec<Entity>,
) -> anyhow::Result<bool> {
    let policies = policies()?;
    let action_uid = action_uid(action)?;
    if !entities
        .iter()
        .any(|entity| entity.uid() == action_uid.clone())
    {
        entities.push(Entity::new_no_attrs(
            action_uid.clone(),
            std::collections::HashSet::default(),
        ));
    }
    let entities = Entities::from_entities(entities, None)
        .map_err(|error| anyhow::anyhow!("invalid authorization entities: {error}"))?;
    let request = Request::new(
        principal.clone(),
        action_uid,
        resource.clone(),
        cedar_policy::Context::empty(),
        None,
    )
    .map_err(|error| anyhow::anyhow!("invalid authorization request: {error}"))?;
    Ok(Authorizer::new()
        .is_authorized(&request, policies, &entities)
        .decision()
        == Decision::Allow)
}
