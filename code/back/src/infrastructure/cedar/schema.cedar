// Authorization action vocabulary (Cedar schema): 16 actions and the entity
// type declarations. This is the single handwritten source of the action names;
// the permission nodes are seeded from these names. Static, requires restart.

entity User in [Role] { global_role: Bool, scopes: Set<Tag> };
entity Role;
entity Tag;
entity Article  { owner: User, required_scopes: Set<Tag> };
entity Version  { owner: User, required_scopes: Set<Tag> };
entity Comment  { owner: User, required_scopes: Set<Tag> };
entity System;

action "Article::Create"  appliesTo { principal: [User], resource: [Article] };
action "Article::Read"    appliesTo { principal: [User], resource: [Article, Version, Comment] };
action "Article::Update"  appliesTo { principal: [User], resource: [Article, Version, Comment] };
action "Article::Delete"  appliesTo { principal: [User], resource: [Article] };
action "Version::Create"  appliesTo { principal: [User], resource: [Article] };
action "Version::Read"    appliesTo { principal: [User], resource: [Version] };
action "Version::Update"  appliesTo { principal: [User], resource: [Version] };
action "Version::Delete"  appliesTo { principal: [User], resource: [Version] };
action "Comment::Create"  appliesTo { principal: [User], resource: [Version, Comment] };
action "Comment::Read"    appliesTo { principal: [User], resource: [Comment] };
action "Comment::Update"  appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete"  appliesTo { principal: [User], resource: [Comment] };
action "User::Read"       appliesTo { principal: [User], resource: [System] };
action "User::Update"     appliesTo { principal: [User], resource: [System] };
action "User::Delete"     appliesTo { principal: [User], resource: [System] };
action "Role::Manage"     appliesTo { principal: [User], resource: [System] };
