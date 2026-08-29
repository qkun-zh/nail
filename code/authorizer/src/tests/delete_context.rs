// Slice s1 (udel): deletion-confirmation context. Red phase — asserts the
// post-change decisions through the real authorizer API.
use crate::Grant;
use crate::Principal;
use crate::authorizer::Authorizer;
use crate::request_context::RequestContext;
use crate::resource::Resource;

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

fn user(id: &str) -> Resource {
    Resource::User(id.to_string())
}

const NO_GRANTS: &[(&str, &str)] = &[];
const ADMIN_DELETE_GRANTS: &[(&str, &str)] = &[
    ("admin", "User::Delete::Soft"),
    ("admin", "User::Delete::Transfer"),
    ("admin", "User::Delete::Hard"),
];

#[test]
fn member_self_soft_denied_without_confirmation_flag() {
    let authorization = authorizer(NO_GRANTS);
    assert_eq!(
        authorization.authorize(
            &actor("alice", &["member"]),
            "User::Delete::Soft",
            &user("alice"),
        ),
        Err(crate::Error::Denied)
    );
}

#[test]
fn member_self_soft_allowed_with_confirmation_flag() {
    let authorization = authorizer(NO_GRANTS);
    authorization
        .authorize_ctx(
            &actor("alice", &["member"]),
            "User::Delete::Soft",
            &user("alice"),
            &RequestContext {
                delete_token_confirmed: true,
            },
        )
        .expect("self-service deregister with confirmation must be allowed");
}

#[test]
fn member_self_soft_denied_with_false_flag() {
    let authorization = authorizer(NO_GRANTS);
    assert_eq!(
        authorization.authorize_ctx(
            &actor("alice", &["member"]),
            "User::Delete::Soft",
            &user("alice"),
            &RequestContext {
                delete_token_confirmed: false,
            },
        ),
        Err(crate::Error::Denied)
    );
}

#[test]
fn member_soft_others_denied() {
    let authorization = authorizer(NO_GRANTS);
    assert_eq!(
        authorization.authorize(
            &actor("alice", &["member"]),
            "User::Delete::Soft",
            &user("bob"),
        ),
        Err(crate::Error::Denied)
    );
}

#[test]
fn member_self_transfer_denied_without_a_grant() {
    let authorization = authorizer(NO_GRANTS);
    assert_eq!(
        authorization.authorize(
            &actor("alice", &["member"]),
            "User::Delete::Transfer",
            &user("alice"),
        ),
        Err(crate::Error::Denied)
    );
}

#[test]
fn admin_deletes_others_without_context_flag() {
    let authorization = authorizer(ADMIN_DELETE_GRANTS);
    for action in [
        "User::Delete::Soft",
        "User::Delete::Transfer",
        "User::Delete::Hard",
    ] {
        authorization
            .authorize(&actor("root", &["admin"]), action, &user("alice"))
            .expect("admin grant must not depend on context");
    }
}

#[test]
fn admin_soft_self_allowed_via_grant() {
    let authorization = authorizer(ADMIN_DELETE_GRANTS);
    authorization
        .authorize_ctx(
            &actor("root", &["admin"]),
            "User::Delete::Soft",
            &user("root"),
            &RequestContext {
                delete_token_confirmed: false,
            },
        )
        .expect("admin grant beats the flag requirement");
}

#[test]
fn self_service_read_and_update_still_open_without_flag() {
    let authorization = authorizer(NO_GRANTS);
    for action in ["User::Read", "User::Update"] {
        authorization
            .authorize(&actor("alice", &[]), action, &user("alice"))
            .expect("self read/update stays grant-free");
    }
}
