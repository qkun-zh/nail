// Authorization action vocabulary (Cedar schema): 27 actions and the entity
// type declarations. This is the single handwritten source of the action names;
// the permission nodes are seeded from these names. Static, requires restart.

entity User in [Role];
entity Role;
entity Article  { owner: User };
entity Version  { owner: User };
entity Comment  { owner: User };
entity Virtual;

action "Article::Create"            appliesTo { principal: [User], resource: [Virtual] };
action "Article::Read"              appliesTo { principal: [User], resource: [Article, Version, Comment] };
action "Article::Update"            appliesTo { principal: [User], resource: [Article, Version, Comment] };
action "Article::Delete::Hard"      appliesTo { principal: [User], resource: [Article] };
action "Article::Delete::Transfer"  appliesTo { principal: [User], resource: [Article] };
action "Article::Delete::Soft"      appliesTo { principal: [User], resource: [Article] };
action "Article::Undelete::Soft"      appliesTo { principal: [User], resource: [Article] };
action "Version::Create"            appliesTo { principal: [User], resource: [Article] };
action "Version::Read"              appliesTo { principal: [User], resource: [Version] };
action "Version::Update"            appliesTo { principal: [User], resource: [Version] };
action "Version::Delete::Hard"      appliesTo { principal: [User], resource: [Version] };
action "Version::Delete::Soft"       appliesTo { principal: [User], resource: [Version] };
action "Version::Undelete::Soft"      appliesTo { principal: [User], resource: [Version] };
action "Comment::Create"            appliesTo { principal: [User], resource: [Virtual] };
action "Comment::Read"              appliesTo { principal: [User], resource: [Comment] };
action "Comment::Update"            appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Hard"      appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Transfer"  appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Soft"      appliesTo { principal: [User], resource: [Comment] };
action "Comment::Undelete::Soft"      appliesTo { principal: [User], resource: [Comment] };
action "User::Create"               appliesTo { principal: [User], resource: [Virtual] };
action "User::Read"                 appliesTo { principal: [User], resource: [User] };
action "User::Update"               appliesTo { principal: [User], resource: [Virtual] };
action "User::Delete::Hard"         appliesTo { principal: [User], resource: [Virtual] };
action "User::Delete::Transfer"     appliesTo { principal: [User], resource: [User] };
action "User::Delete::Soft"         appliesTo { principal: [User], resource: [User] };
action "User::Undelete::Soft"       appliesTo { principal: [User], resource: [User] };
action "Role::Manage"               appliesTo { principal: [User], resource: [Virtual] };
action "Role::Revoke"               appliesTo { principal: [User], resource: [Role] };
