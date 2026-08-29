// Authorization action vocabulary (Cedar schema): 39 actions and the entity
// type declarations. This is the single handwritten source of the action names;
// the permission nodes are seeded from these names. Static, requires restart.
//
// Container hierarchy is declared here so resource membership (`in`) is a
// documented schema construct: Version hangs off its Article, replies hang off
// their parent Comment chain.
//
// Collection gates anchor their read action on the collection desk (Virtual)
// or the parent container, so those actions declare the extra applicable
// resource types here; instance reads keep the concrete type.

entity User in [Role];
entity Role;
entity Article  { owner: User };
entity Version in [Article]  { owner: User };
entity Comment in [Version, Comment]  { owner: User };
entity Tag;
entity Virtual;

action "Article::Create"            appliesTo { principal: [User], resource: [Virtual] };
action "Article::Read"              appliesTo { principal: [User], resource: [Article, Version, Comment, Virtual] };
action "Article::Update"            appliesTo { principal: [User], resource: [Article, Version, Comment] };
action "Article::Delete::Hard"      appliesTo { principal: [User], resource: [Article] };
action "Article::Delete::Transfer"  appliesTo { principal: [User], resource: [Article] };
action "Article::Delete::Soft"      appliesTo { principal: [User], resource: [Article] };
action "Article::Undelete::Soft"      appliesTo { principal: [User], resource: [Article] };
action "Version::Create"            appliesTo { principal: [User], resource: [Article] };
action "Version::Read"              appliesTo { principal: [User], resource: [Version, Article] };
action "Version::Update"            appliesTo { principal: [User], resource: [Version] };
action "Version::Delete::Hard"      appliesTo { principal: [User], resource: [Version] };
action "Version::Delete::Soft"       appliesTo { principal: [User], resource: [Version] };
action "Version::Undelete::Soft"      appliesTo { principal: [User], resource: [Version] };
action "Comment::Create"            appliesTo { principal: [User], resource: [Virtual] };
action "Comment::Read"              appliesTo { principal: [User], resource: [Comment, Version] };
action "Comment::Update"            appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Hard"      appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Transfer"  appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Soft"      appliesTo { principal: [User], resource: [Comment] };
action "Comment::Undelete::Soft"      appliesTo { principal: [User], resource: [Comment] };
action "User::Create"               appliesTo { principal: [User], resource: [Virtual] };
action "User::Read"                 appliesTo { principal: [User], resource: [User, Virtual] };
action "User::Update"               appliesTo { principal: [User], resource: [User] };
action "User::Delete::Hard"         appliesTo { principal: [User], resource: [User] };
action "User::Delete::Transfer"     appliesTo { principal: [User], resource: [User] };
action "User::Delete::Soft"         appliesTo { principal: [User], resource: [User] };
action "User::Undelete::Soft"       appliesTo { principal: [User], resource: [User] };
action "Role::Create"               appliesTo { principal: [User], resource: [Virtual] };
action "Role::Read"                 appliesTo { principal: [User], resource: [Role, Virtual] };
action "Role::Update"               appliesTo { principal: [User], resource: [Role] };
action "Role::Delete"               appliesTo { principal: [User], resource: [Role] };
action "Role::Grant"                appliesTo { principal: [User], resource: [Role] };
action "Role::Revoke"               appliesTo { principal: [User], resource: [Role] };
action "Tag::Create"                appliesTo { principal: [User], resource: [Virtual] };
action "Tag::Read"                  appliesTo { principal: [User], resource: [Tag, Virtual] };
action "Tag::Update"                appliesTo { principal: [User], resource: [Tag] };
action "Tag::Delete"                appliesTo { principal: [User], resource: [Tag] };
action "Tag::Apply"                 appliesTo { principal: [User], resource: [Tag] };
action "Tag::Unapply"               appliesTo { principal: [User], resource: [Tag] };
