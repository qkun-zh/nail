// Authorization policies (Cedar): the single handwritten policy set. Changing
// permissions is done through role/permission edges in data, never by editing
// these policies.

// 1. Owner bypass: the resource owner may read/update, create new versions,
//    and transfer (soft-delete) their own content. Published content is a
//    platform asset, so the owner can never hard-delete it: the Delete actions
//    listed here are the Transfer and Soft variants only. Version::Update/Delete
//    and Comment::Update keep the author stories in FR-20/FR-21; version
//    hard-delete is admin-only. Members hold no global Transfer/Soft grants:
//    the owner bypass is the only member path to delete/transfer content.
permit (principal, action in [Action::"Article::Read", Action::"Version::Read",
                              Action::"Comment::Read",
                              Action::"Comment::Delete::Transfer",
                              Action::"Comment::Delete::Soft",
                              Action::"Article::Update", Action::"Article::Delete::Transfer",
                              Action::"Article::Delete::Soft",
                              Action::"Version::Create", Action::"Version::Update",
                              Action::"Version::Delete::Transfer",
                              Action::"Version::Delete::Soft",
                              Action::"Comment::Update"],
        resource)
  when { resource.owner == principal };

// 2. Read-open: any authenticated principal may read article/version/comment.
permit (principal, action in [Action::"Article::Read", Action::"Version::Read",
                              Action::"Comment::Read"],
        resource);

// 3. Role permission: the principal's role ancestor closure contains the
//    action. Roles are pure sets of users; the scope axis was removed (D1).
permit (principal, action, resource)
  when { principal in action };

// 4. Admin-console meta-permission: User::Read/Update/Delete and Role::Manage
//    are only judged against Virtual::"admin-console".
permit (principal, action in [Action::"User::Read", Action::"User::Update",
                              Action::"User::Delete::Hard",
                              Action::"User::Delete::Transfer",
                              Action::"Role::Manage"],
        resource)
  when { resource == Virtual::"admin-console" && principal in action };

// 5. Recycler is a terminal sink: a recycler can never transfer content. This
//    principal-side forbid also applies to user_zero, who holds the recycler
//    role alongside admin, so the admin moderation path is hard-delete only.
forbid (principal in Role::"recycler",
        action in [Action::"Article::Delete::Transfer",
                   Action::"Comment::Delete::Transfer",
                   Action::"Version::Delete::Transfer"],
        resource);

// 6. Admin override: holding the admin role allows everything.
permit (principal, action, resource)
  when { principal in Role::"admin" };

// 7. Admin role is terminal for revocation (D7-b): nobody — not even admin via
//    the override — may revoke from the admin role. Forbid beats permit.
forbid (principal, action == Action::"Role::Revoke", resource == Role::"admin");
