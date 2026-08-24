// TARGET DRAFT (v1, for review) — static policy set.
// Dynamic part: one template per action is added programmatically
// (`permit (principal in ?principal, action == Action::"<A>", resource);`)
// and linked once per (role, permission) grant. Grant/revoke = link/unlink.

// 1. Owner bypass (unchanged semantics from current policy 1).
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

// 1b. User self-service (unchanged).
permit (principal, action in [Action::"User::Read",
                              Action::"User::Update",
                              Action::"User::Delete::Soft",
                              Action::"User::Delete::Transfer"],
        resource)
  when { principal == resource };

// 2-4. Role permissions now live in template-linked policies (one link per
//      role x action). The unconstrained mega-permit is gone.

// 5. Recycler terminal sink (unchanged).
forbid (principal in Role::"recycler",
        action in [Action::"Article::Delete::Transfer",
                   Action::"Comment::Delete::Transfer"],
        resource);

// 6. Admin revoke protection (unchanged).
forbid (principal, action == Action::"Role::Revoke", resource == Role::"admin");

// 7. Registration open, anchored on the application entity.
permit (principal, action == Action::"User::Create",
        resource == Application::"nail");

// 8. Soft-delete visibility stays a COMPOUND authorization (bp-compound-auth):
//    query 1 = the normal permission check; query 2 = authorize(Undelete::Soft)
//    on the same resource, consulted only when the soft-delete flag is set.
//    A single static forbid cannot express "holds the Undelete grant" under
//    template-linked grants (grants are policies, not data), and empirical
//    runs confirmed neither exemption variant reproduces the required
//    hide-from-members / show-to-holders semantics. Composition lives in
//    require_entity_readable, with BOTH decision legs evaluated by Cedar.
