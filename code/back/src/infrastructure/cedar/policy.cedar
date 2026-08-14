// Authorization policies (Cedar): the single handwritten policy set. Changing
// permissions is done through role/permission edges in data, never by editing
// these policies.

// 1. Owner bypass: the resource owner may read/update/delete and create new
//    versions; Comment::Delete lets a comment author delete their own comment;
//    Version::Update/Delete and Comment::Update keep the author stories in
//    FR-20/FR-21 (edit any version note, hard-delete a version, edit my
//    comment content).
permit (principal, action in [Action::"Article::Read", Action::"Version::Read",
                              Action::"Comment::Read", Action::"Comment::Delete",
                              Action::"Article::Update", Action::"Article::Delete",
                              Action::"Version::Create", Action::"Version::Update",
                              Action::"Version::Delete", Action::"Comment::Update"],
        resource)
  when { resource.owner == principal };

// 2. Read-open: any authenticated principal may read article/version/comment.
permit (principal, action in [Action::"Article::Read", Action::"Version::Read",
                              Action::"Comment::Read"],
        resource);

// 3. Role permission + scope: the principal's role ancestor closure contains
//    the action, and the scope matches (a role with no tag edges is global).
permit (principal, action, resource)
  when {
      principal in action
      && (principal.global_role || principal.scopes.containsAny(resource.required_scopes))
  };

// 4. Admin-console meta-permission: User::Read/Update/Delete and Role::Manage
//    are only judged against System::"admin-console".
permit (principal, action in [Action::"User::Read", Action::"User::Update",
                              Action::"User::Delete", Action::"Role::Manage"],
        resource)
  when { resource == System::"admin-console" && principal in action };

// 5. Admin override: holding the admin role allows everything.
permit (principal, action, resource)
  when { principal in Role::"admin" };
