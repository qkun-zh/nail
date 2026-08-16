// Authorization action vocabulary (Cedar schema): 23 actions and the entity
// type declarations. This is the single handwritten source of the action names;
// the permission nodes are seeded from these names. Static, requires restart.

entity User in [Role] { global_role: Bool, scopes: Set<Tag> };
entity Role;
entity Tag;
entity Article  { owner: User, required_scopes: Set<Tag> };
entity Version  { owner: User, required_scopes: Set<Tag> };
entity Comment  { owner: User, required_scopes: Set<Tag> };
entity System;

action "Article::Create"            appliesTo { principal: [User], resource: [Article] };
action "Article::Read"              appliesTo { principal: [User], resource: [Article, Version, Comment] };
action "Article::Update"            appliesTo { principal: [User], resource: [Article, Version, Comment] };
action "Article::Delete::Hard"      appliesTo { principal: [User], resource: [Article] };
action "Article::Delete::Transfer"  appliesTo { principal: [User], resource: [Article] };
action "Article::Delete::Soft"      appliesTo { principal: [User], resource: [Article] };
action "Article::Restore"           appliesTo { principal: [User], resource: [Article] };
action "Version::Create"            appliesTo { principal: [User], resource: [Article] };
action "Version::Read"              appliesTo { principal: [User], resource: [Version] };
action "Version::Update"            appliesTo { principal: [User], resource: [Version] };
action "Version::Delete::Hard"      appliesTo { principal: [User], resource: [Version] };
action "Version::Delete::Transfer"  appliesTo { principal: [User], resource: [Version] };
action "Version::Delete::Soft"      appliesTo { principal: [User], resource: [Version] };
action "Version::Restore"           appliesTo { principal: [User], resource: [Version] };
action "Comment::Create"            appliesTo { principal: [User], resource: [Version, Comment] };
action "Comment::Read"              appliesTo { principal: [User], resource: [Comment] };
action "Comment::Update"            appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Hard"      appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Transfer"  appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Soft"      appliesTo { principal: [User], resource: [Comment] };
action "Comment::Restore"           appliesTo { principal: [User], resource: [Comment] };
action "User::Read"                 appliesTo { principal: [User], resource: [System] };
action "User::Update"               appliesTo { principal: [User], resource: [System] };
action "User::Delete::Hard"         appliesTo { principal: [User], resource: [System] };
action "User::Delete::Transfer"     appliesTo { principal: [User], resource: [System] };
action "Role::Manage"               appliesTo { principal: [User], resource: [System] };
