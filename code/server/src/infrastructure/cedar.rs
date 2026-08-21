#![allow(dead_code)]

#[cfg(test)]
use cedar_policy::EntityUid;

pub const SCHEMA: &str = include_str!("cedar/schema.cedar");
pub const POLICY: &str = include_str!("cedar/policy.cedar");

pub fn schema_actions() -> anyhow::Result<Vec<String>> {
    let schema: cedar_policy::Schema = SCHEMA
        .parse()
        .map_err(|error| anyhow::anyhow!("invalid authorization schema: {error}"))?;
    let mut names: Vec<String> = schema
        .actions()
        .map(|action| action.id().unescaped().to_string())
        .collect();
    names.sort();
    names.dedup();
    Ok(names)
}

#[cfg(test)]
pub fn action_uid(action: &str) -> anyhow::Result<EntityUid> {
    format!("Action::\"{action}\"")
        .parse::<EntityUid>()
        .map_err(|error| anyhow::anyhow!("invalid action uid for {action:?}: {error}"))
}
