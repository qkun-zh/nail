// Authorization policies (Cedar): the single handwritten policy set. Changing
// permissions is done through role/permission edges in data, never by editing
// these policies.

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
  when { resource.owner == principal };

// 1b. User self-service: a user may always read their own record and
//     deregister their own account (soft or transfer mode). The User entity
//     carries no attributes (schema `entity User in [Role];`), so self-service
//     is expressed directly as `principal == resource` instead of an owner
//     attribute — User stays attribute-less in both the principal and the
//     resource position.
permit (principal, action in [Action::"User::Read",
                              Action::"User::Delete::Soft",
                              Action::"User::Delete::Transfer"],
        resource)
  when { principal == resource };

// 2 was the read-open rule, removed in B1: reads now flow through policy 1
// (owner) or the role grant in policy 3.

// 3. Role permission: the principal's role ancestor closure contains the
//    action. Roles are pure sets of users; the scope axis was removed (D1).
permit (principal, action, resource)
  when { principal in action };

// 4. Admin-console meta-permission: User::Update/Delete::Hard and the
//    role-management CRUD actions are only judged against
//    Virtual::"admin-console". User::Read and the self-service deletes
//    (Soft/Transfer) left this rule: User::Read rides the User resource
//    self-service (policy 1b), and deregistration rides the same self rule, so
//    this rule keeps the admin-only hard-delete, user update and role CRUD
//    actions. An admin with the grant still reaches them through the generic
//    role rule (policy 3) on Virtual::"admin-console". Role::Grant and
//    Role::Revoke act on the concrete Role resource (policy 3).
permit (principal, action in [Action::"User::Update",
                              Action::"User::Delete::Hard",
                              Action::"Role::Create",
                              Action::"Role::Read",
                              Action::"Role::Update",
                              Action::"Role::Delete"],
        resource)
  when { resource == Virtual::"admin-console" && principal in action };

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
