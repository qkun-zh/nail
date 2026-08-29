// Authorization policies (Cedar): the handwritten static half of the policy
// set. Role permissions are NOT written here: at startup each schema action
// becomes a policy template and each role/permission edge in data becomes a
// template link, so granting or revoking stays a data operation while every
// effective rule remains an ordinary inspectable Cedar policy.

// 1. Owner bypass: the resource owner may read/update, create new versions,
//    and transfer (soft-delete) their own content. Published content is a
//    platform asset, so the owner can never hard-delete it: the Delete actions
//    listed here are the Transfer and Soft variants only.
//    Version::Update/Delete and Comment::Update keep the author stories in
//    FR-20/FR-21; version hard-delete is admin-only. Members hold no global
//    Transfer/Soft grants: the owner bypass is the only member path to
//    delete/transfer content.
permit (principal, action in [Action::"Article::Read", Action::"Version::Read",
                              Action::"Comment::Read",
                              Action::"Comment::Delete::Transfer",
                              Action::"Comment::Delete::Soft",
                              Action::"Article::Update", Action::"Article::Delete::Transfer",
                              Action::"Article::Delete::Soft",
                              Action::"Version::Create", Action::"Version::Update",
                              Action::"Version::Delete::Soft",
                              Action::"Comment::Update"],
        resource)
  when { resource has owner && resource.owner == principal };

// 1b. User self-service: a user may always read, update and deregister their
//     own account (soft or transfer mode). The User entity carries no
//     attributes (schema `entity User in [Role];`), so self-service is
//     expressed directly as `principal == resource` instead of an owner
//     attribute — User stays attribute-less in both the principal and the
//     resource position.
permit (principal, action in [Action::"User::Read",
                              Action::"User::Update",
                              Action::"User::Delete::Soft",
                              Action::"User::Delete::Transfer"],
        resource)
  when { principal == resource };

// 2 was the read-open rule, removed in B1: reads now flow through policy 1
// (owner) or the role grant in policy 3.

// 2. Role permissions: for every schema action, a template of the shape
//    "permit (principal in ?principal, action == that action, resource);" is
//    generated at startup and linked once per role/permission edge, so the
//    scope is always populated and the engine can index every rule. This
//    replaces the former unconstrained mega-permit
//    (permit principal/action/resource when principal in action).

// 4. was the admin-console meta-permission rule, removed in S7: User::Update/
//    Delete::Hard now ride the concrete User resource (self-service through
//    policy 1b, admin through the grant in policy 3), Role operations ride the
//    concrete Role resource, and Role::Create rides a Virtual resource through
//    policy 3. No Virtual::"admin-console" hardcode remains.

// 5. Recycler is a terminal sink: a recycler can never transfer content. This
//    principal-side forbid also applies to user_zero, who holds the recycler
//    role alongside admin, so the admin moderation path is hard-delete only.
forbid (principal in Role::"recycler",
        action in [Action::"Article::Delete::Transfer",
                   Action::"Comment::Delete::Transfer"],
        resource);

// 6. Admin role is terminal for revocation (D7-b): nobody — not even an admin
//    with a grant — may revoke from the admin role. Forbid beats permit.
forbid (principal, action == Action::"Role::Revoke", resource == Role::"admin");

// 7. Registration is open: User::Create is judged only against a synthetic
//    anonymous principal, so no session or grant can ever gate it. Later
//    restrictions (e.g. by domain) go here as conditions, not in the caller.
permit (principal, action == Action::"User::Create", resource);
