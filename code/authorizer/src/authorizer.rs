use std::collections::{BTreeSet, HashMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use cedar_policy::{
    Authorizer as CedarAuthorizer, Decision, Entities, Entity, EntityId, EntityTypeName, EntityUid,
    PolicyId, PolicySet, Request, RestrictedExpression, Schema, SlotId, Template, ValidationMode,
    Validator,
};

use crate::error::Error;
use crate::principal::Principal;
use crate::resource::Resource;

pub(crate) const POLICY: &str = include_str!("../cedar/policy.cedar");
pub(crate) const SCHEMA: &str = include_str!("../cedar/schema.cedar");

include!(concat!(env!("OUT_DIR"), "/cedar_entities.rs"));

const TEMPLATE_PREFIX: &str = "tpl_";
const LINK_PREFIX: &str = "link_";

/// A durable role-to-permission edge. Grants live in the database; at startup
/// and on every grant/revoke they are projected into template links so each
/// effective rule is an ordinary Cedar policy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Grant {
    pub role: String,
    pub permission: String,
}

#[derive(Clone)]
pub struct Authorizer {
    schema: Arc<Schema>,
    cedar: Arc<CedarAuthorizer>,
    policies: Arc<RwLock<Arc<PolicySet>>>,
}

impl Authorizer {
    pub fn new(grants: &[Grant]) -> Result<Self, Error> {
        let schema = SCHEMA
            .parse::<Schema>()
            .map_err(|error| Error::Internal(format!("invalid authorization schema: {error}")))?;
        let policies = Arc::new(RwLock::new(Arc::new(build_policy_set(&schema, grants)?)));
        Ok(Self {
            schema: Arc::new(schema),
            cedar: Arc::new(CedarAuthorizer::new()),
            policies,
        })
    }

    /// Re-derives all template links from the given grants and swaps the
    /// active policy set atomically. In-flight requests keep their snapshot.
    pub fn reload(&self, grants: &[Grant]) -> Result<(), Error> {
        let policies = build_policy_set(&self.schema, grants)?;
        *self
            .policies
            .write()
            .map_err(|_| Error::Internal("policy lock poisoned".to_string()))? = Arc::new(policies);
        Ok(())
    }

    pub fn authorize(
        &self,
        principal: &Principal,
        action: &str,
        resource: &Resource,
    ) -> Result<(), Error> {
        let principal_entity = build_principal(principal)?;
        let (resource_uid, resource_entities) = build_resource(resource)?;

        let mut positions: HashMap<EntityUid, usize> = HashMap::new();
        let mut merged: Vec<Entity> = Vec::new();
        for entity in std::iter::once(principal_entity).chain(resource_entities) {
            if let Some(index) = positions.get(&entity.uid()) {
                merged[*index] = entity;
            } else {
                positions.insert(entity.uid(), merged.len());
                merged.push(entity);
            }
        }

        let entities = Entities::from_entities(merged, Some(&self.schema)).map_err(|error| {
            Error::Internal(format!("authorization entities rejected: {error}"))
        })?;

        let action_uid =
            action_uid(action).map_err(|error| Error::InvalidRequest(error.to_string()))?;
        let request = Request::new(
            user_uid(&principal.id)?,
            action_uid,
            resource_uid,
            cedar_policy::Context::empty(),
            Some(&self.schema),
        )
        .map_err(|error| Error::InvalidRequest(error.to_string()))?;

        let policies = self
            .policies
            .read()
            .map_err(|_| Error::Internal("policy lock poisoned".to_string()))?
            .clone();

        match self
            .cedar
            .is_authorized(&request, &policies, &entities)
            .decision()
        {
            Decision::Allow => Ok(()),
            Decision::Deny => Err(Error::Denied),
        }
    }
}

/// Builds the full policy set: handwritten static policies, one template per
/// declared schema action, one link per grant, then strict validation of the
/// combined set against the schema.
fn build_policy_set(schema: &Schema, grants: &[Grant]) -> Result<PolicySet, Error> {
    let mut policies: PolicySet = POLICY
        .parse()
        .map_err(|error| Error::Internal(format!("invalid authorization policies: {error}")))?;

    for action in sorted_action_ids(schema) {
        let source =
            format!("permit (principal in ?principal, action == Action::\"{action}\", resource);");
        let template = Template::parse(
            Some(PolicyId::new(format!("{TEMPLATE_PREFIX}{action}"))),
            source,
        )
        .map_err(|error| Error::Internal(format!("invalid template for {action}: {error}")))?;
        policies.add_template(template).map_err(|error| {
            Error::Internal(format!("cannot add template for {action}: {error}"))
        })?;
    }

    for grant in unique_grants(grants) {
        let template_id = format!("{TEMPLATE_PREFIX}{}", grant.permission);
        let declared = sorted_action_ids(schema);
        if !declared.iter().any(|name| name == &grant.permission) {
            return Err(Error::Internal(format!(
                "grant references unknown permission {}",
                grant.permission
            )));
        }
        let values = HashMap::from([(SlotId::principal(), role_uid(&grant.role)?)]);
        policies
            .link(
                PolicyId::new(template_id),
                PolicyId::new(format!("{LINK_PREFIX}{}_{}", grant.role, grant.permission)),
                values,
            )
            .map_err(|error| {
                Error::Internal(format!(
                    "cannot link {} to {}: {error}",
                    grant.permission, grant.role
                ))
            })?;
    }

    let validation = Validator::new(schema.clone()).validate(&policies, ValidationMode::Strict);
    if !validation.validation_passed() {
        let messages: Vec<String> = validation
            .validation_errors()
            .map(std::string::ToString::to_string)
            .collect();
        return Err(Error::Internal(format!(
            "policy set does not validate against schema: {messages:?}"
        )));
    }
    Ok(policies)
}

fn unique_grants(grants: &[Grant]) -> BTreeSet<&Grant> {
    grants.iter().collect()
}

fn sorted_action_ids(schema: &Schema) -> Vec<String> {
    let mut ids: Vec<String> = schema
        .actions()
        .map(|action| action.id().unescaped().to_string())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

fn build_principal(principal: &Principal) -> Result<Entity, Error> {
    let mut parents = HashSet::new();
    for role in &principal.roles {
        parents.insert(role_uid(role)?);
    }
    Ok(Entity::new_no_attrs(user_uid(&principal.id)?, parents))
}

fn build_resource(resource: &Resource) -> Result<(EntityUid, Vec<Entity>), Error> {
    match resource {
        Resource::Article { id, owner } => {
            let uid = article_uid(id)?;
            let entity = article_entity(uid.clone(), owner, HashSet::new())?;
            Ok((uid, vec![entity]))
        }
        Resource::Version {
            id,
            article_id,
            owner,
        } => {
            let article_uid = article_uid(article_id)?;
            let version_uid = version_uid(id)?;
            let article = article_entity(article_uid.clone(), owner, HashSet::new())?;
            let version = Entity::new(
                version_uid.clone(),
                resource_attrs(owner)?,
                HashSet::from([article_uid]),
            )
            .map_err(internal_error)?;
            Ok((version_uid, vec![article, version]))
        }
        Resource::Comment {
            id,
            version_id,
            article_id,
            article_owner,
            owner,
        } => {
            let article_uid = article_uid(article_id)?;
            let version_uid = version_uid(version_id)?;
            let comment_uid = comment_uid(id)?;
            let article = article_entity(article_uid.clone(), article_owner, HashSet::new())?;
            let version = Entity::new(
                version_uid.clone(),
                resource_attrs(article_owner)?,
                HashSet::from([article_uid]),
            )
            .map_err(internal_error)?;
            let comment = Entity::new(
                comment_uid.clone(),
                resource_attrs(owner)?,
                HashSet::from([version_uid]),
            )
            .map_err(internal_error)?;
            Ok((comment_uid, vec![article, version, comment]))
        }
        Resource::Role { name } => {
            let uid = role_uid(name)?;
            Ok((uid.clone(), vec![Entity::new_no_attrs(uid, HashSet::new())]))
        }
        Resource::User(user_id) => {
            let uid = user_uid(user_id)?;
            Ok((uid.clone(), vec![Entity::new_no_attrs(uid, HashSet::new())]))
        }
        Resource::Tag(tag_id) => {
            let uid = tag_uid(tag_id)?;
            Ok((uid.clone(), vec![Entity::new_no_attrs(uid, HashSet::new())]))
        }
        Resource::Virtual(name) => {
            let uid = virtual_uid(name)?;
            Ok((uid.clone(), vec![Entity::new_no_attrs(uid, HashSet::new())]))
        }
    }
}

fn article_entity(
    uid: EntityUid,
    owner: &str,
    parents: HashSet<EntityUid>,
) -> Result<Entity, Error> {
    Entity::new(uid, resource_attrs(owner)?, parents).map_err(internal_error)
}

fn typed_uid(entity_type: &str, id: &str) -> Result<EntityUid, Error> {
    let type_name = EntityTypeName::from_str(entity_type).map_err(|error| {
        Error::Internal(format!("invalid entity type {entity_type:?}: {error}"))
    })?;
    let entity_id = EntityId::new(id);
    Ok(EntityUid::from_type_name_and_id(type_name, entity_id))
}

fn user_uid(user_id: &str) -> Result<EntityUid, Error> {
    typed_uid(CEDAR_ENTITY_USER, user_id)
}

fn role_uid(role_name: &str) -> Result<EntityUid, Error> {
    typed_uid(CEDAR_ENTITY_ROLE, role_name)
}

fn action_uid(action: &str) -> Result<EntityUid, Error> {
    typed_uid("Action", action)
}

fn article_uid(article_id: &str) -> Result<EntityUid, Error> {
    typed_uid(CEDAR_ENTITY_ARTICLE, article_id)
}

fn version_uid(version_id: &str) -> Result<EntityUid, Error> {
    typed_uid(CEDAR_ENTITY_VERSION, version_id)
}

fn comment_uid(comment_id: &str) -> Result<EntityUid, Error> {
    typed_uid(CEDAR_ENTITY_COMMENT, comment_id)
}

fn tag_uid(tag_id: &str) -> Result<EntityUid, Error> {
    typed_uid(CEDAR_ENTITY_TAG, tag_id)
}

fn virtual_uid(name: &str) -> Result<EntityUid, Error> {
    typed_uid(CEDAR_ENTITY_VIRTUAL, name)
}

fn resource_attrs(owner_id: &str) -> Result<HashMap<String, RestrictedExpression>, Error> {
    if owner_id.is_empty() {
        return Err(Error::Internal("resource owner is missing".to_string()));
    }
    let owner = RestrictedExpression::new_entity_uid(user_uid(owner_id)?);
    Ok(HashMap::from([("owner".to_string(), owner)]))
}

fn internal_error<E: std::error::Error>(error: E) -> Error {
    Error::Internal(format!("authorization error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Grant;

    fn authorizer(grants: &[(&str, &str)]) -> Authorizer {
        let grants = grants
            .iter()
            .map(|(role, permission)| Grant {
                role: (*role).to_string(),
                permission: (*permission).to_string(),
            })
            .collect::<Vec<_>>();
        Authorizer::new(&grants).expect("authorizer")
    }

    fn actor(id: &str, roles: &[&str]) -> Principal {
        Principal {
            id: id.to_string(),
            roles: roles.iter().map(ToString::to_string).collect(),
        }
    }

    fn article(id: &str, owner: &str) -> Resource {
        Resource::Article {
            id: id.to_string(),
            owner: owner.to_string(),
        }
    }

    const MEMBER_SEED: &[(&str, &str)] = &[
        ("member", "Article::Create"),
        ("member", "Article::Read"),
        ("member", "Comment::Create"),
    ];

    #[test]
    fn new_validates_policy() {
        assert!(Authorizer::new(&[]).is_ok());
    }

    #[test]
    fn reload_rejects_unknown_permissions() {
        let authorizer = authorizer(&[]);
        assert!(
            authorizer
                .reload(&[Grant {
                    role: "member".to_string(),
                    permission: "Bogus::Action".to_string()
                }])
                .is_err()
        );
    }

    #[test]
    fn member_article_create_on_virtual_allow() {
        let authorizer = authorizer(MEMBER_SEED);
        let principal = actor("alice", &["member"]);
        assert!(
            authorizer
                .authorize(
                    &principal,
                    "Article::Create",
                    &Resource::Virtual("any".to_string())
                )
                .is_ok()
        );
    }

    #[test]
    fn create_denied_without_grant() {
        let authorizer = authorizer(&[]);
        assert_eq!(
            authorizer.authorize(
                &actor("zoe", &[]),
                "Article::Create",
                &Resource::Virtual("any".to_string())
            ),
            Err(Error::Denied)
        );
    }

    #[test]
    fn member_reads_article_via_grant_link() {
        let authorizer = authorizer(MEMBER_SEED);
        assert!(
            authorizer
                .authorize(
                    &actor("bob", &["member"]),
                    "Article::Read",
                    &article("a1", "carol")
                )
                .is_ok()
        );
    }

    #[test]
    fn owner_bypass_allow_without_role() {
        let authorizer = authorizer(&[]);
        assert!(
            authorizer
                .authorize(
                    &actor("alice", &[]),
                    "Article::Update",
                    &article("a1", "alice")
                )
                .is_ok()
        );
    }

    #[test]
    fn non_owner_denied_without_grant() {
        let authorizer = authorizer(&[]);
        assert_eq!(
            authorizer.authorize(
                &actor("bob", &[]),
                "Article::Update",
                &article("a1", "alice")
            ),
            Err(Error::Denied)
        );
    }

    #[test]
    fn owner_cannot_hard_delete_own_article() {
        let authorizer = authorizer(&[]);
        assert_eq!(
            authorizer.authorize(
                &actor("alice", &[]),
                "Article::Delete::Hard",
                &article("a1", "alice")
            ),
            Err(Error::Denied)
        );
    }

    #[test]
    fn admin_hard_delete_allows_via_grant_link() {
        let authorizer = authorizer(&[("admin", "Article::Delete::Hard")]);
        assert!(
            authorizer
                .authorize(
                    &actor("root", &["admin"]),
                    "Article::Delete::Hard",
                    &article("a1", "alice")
                )
                .is_ok()
        );
    }

    #[test]
    fn recycler_forbid_beats_owner_and_admin_permits() {
        let authorizer = authorizer(&[("admin", "Article::Delete::Transfer")]);
        assert_eq!(
            authorizer.authorize(
                &actor("u0", &["admin", "recycler"]),
                "Article::Delete::Transfer",
                &article("a1", "u0")
            ),
            Err(Error::Denied)
        );
        assert!(
            authorizer
                .authorize(
                    &actor("plain", &["admin"]),
                    "Article::Delete::Transfer",
                    &article("a1", "someone")
                )
                .is_ok()
        );
    }

    #[test]
    fn nobody_revokes_from_the_admin_role() {
        let authorizer = authorizer(&[("admin", "Role::Revoke"), ("editor", "Role::Revoke")]);
        for (who, roles) in [("root", vec!["admin"]), ("outsider", vec!["editor"])] {
            assert_eq!(
                authorizer.authorize(
                    &actor(who, &roles),
                    "Role::Revoke",
                    &Resource::Role {
                        name: "admin".to_string()
                    }
                ),
                Err(Error::Denied)
            );
        }
    }

    #[test]
    fn version_owner_is_the_article_owner() {
        let authorizer = authorizer(&[]);
        let resource = Resource::Version {
            id: "v1".to_string(),
            article_id: "a1".to_string(),
            owner: "alice".to_string(),
        };
        assert!(
            authorizer
                .authorize(&actor("alice", &[]), "Version::Update", &resource)
                .is_ok()
        );
        assert_eq!(
            authorizer.authorize(&actor("carol", &[]), "Version::Update", &resource),
            Err(Error::Denied)
        );
    }

    #[test]
    fn comment_author_updates_own_but_not_others_comments() {
        let authorizer = authorizer(&[]);
        let resource = Resource::Comment {
            id: "c1".to_string(),
            version_id: "v1".to_string(),
            article_id: "a1".to_string(),
            article_owner: "alice".to_string(),
            owner: "carol".to_string(),
        };
        assert!(
            authorizer
                .authorize(&actor("carol", &[]), "Comment::Update", &resource)
                .is_ok()
        );
        assert_eq!(
            authorizer.authorize(&actor("alice", &[]), "Comment::Update", &resource),
            Err(Error::Denied)
        );
    }

    #[test]
    fn user_self_service_and_registration_are_open() {
        let authorizer = authorizer(&[]);
        assert!(
            authorizer
                .authorize(
                    &actor("bob", &[]),
                    "User::Update",
                    &Resource::User("bob".to_string())
                )
                .is_ok()
        );
        assert_eq!(
            authorizer.authorize(
                &actor("bob", &[]),
                "User::Read",
                &Resource::User("alice".to_string())
            ),
            Err(Error::Denied)
        );
        assert!(
            authorizer
                .authorize(
                    &actor("anonymous", &[]),
                    "User::Create",
                    &Resource::Virtual("any".to_string())
                )
                .is_ok()
        );
    }

    #[test]
    fn reload_grants_and_revokes_dynamically() {
        let authorizer = authorizer(&[]);
        let editor = actor("ed", &["editor"]);
        let grant = Grant {
            role: "editor".to_string(),
            permission: "Article::Delete::Transfer".to_string(),
        };
        authorizer
            .reload(std::slice::from_ref(&grant))
            .expect("reload");
        assert!(
            authorizer
                .authorize(
                    &editor,
                    "Article::Delete::Transfer",
                    &article("a1", "alice")
                )
                .is_ok()
        );
        authorizer.reload(&[]).expect("revoke");
        assert_eq!(
            authorizer.authorize(
                &editor,
                "Article::Delete::Transfer",
                &article("a1", "alice")
            ),
            Err(Error::Denied)
        );
    }

    #[test]
    fn malformed_requests_are_rejected_loudly() {
        let authorizer = authorizer(MEMBER_SEED);
        assert!(matches!(
            authorizer.authorize(
                &actor("bob", &["member"]),
                "Nonsense::Hack",
                &Resource::Virtual("any".to_string())
            ),
            Err(Error::InvalidRequest(_))
        ));
        assert!(matches!(
            authorizer.authorize(
                &actor("bob", &["member"]),
                "Article::Create",
                &article("a1", "alice")
            ),
            Err(Error::InvalidRequest(_))
        ));
    }

    #[test]
    fn missing_resource_owner_is_a_loud_error() {
        let authorizer = authorizer(&[]);
        assert!(matches!(
            authorizer.authorize(&actor("bob", &[]), "Article::Read", &article("a1", "")),
            Err(Error::Internal(_))
        ));
    }
}
