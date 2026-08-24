// TARGET DRAFT (v1, for review) — nail authorization schema.
// Changes vs current:
//   * Application entity replaces Virtual as the anchor for creation/global ops
//     (bp-authorization-patterns: open endpoints authorize against a synthetic
//     Application entity).
//   * Container hierarchy declared: Version in [Article], Comment in [Version,
//     Comment] — enables resource-in-container policies and mirrors the parent
//     chains the code already builds per request.
//   * No action groups: dynamic role->permission grants are expressed with one
//     policy template per action (template-linked policies), the documented
//     mechanism when grants are managed at runtime.

entity Application;
entity Role;
entity User in [Role];
entity Article { owner: User };
entity Version in [Article] { owner: User };
entity Comment in [Version, Comment] { owner: User };
entity Tag;

action "Article::Create"            appliesTo { principal: [User], resource: [Application] };
action "Article::Read"              appliesTo { principal: [User], resource: [Article, Version, Comment] };
action "Article::Update"            appliesTo { principal: [User], resource: [Article] };
action "Article::Delete::Hard"      appliesTo { principal: [User], resource: [Article] };
action "Article::Delete::Transfer"  appliesTo { principal: [User], resource: [Article] };
action "Article::Delete::Soft"      appliesTo { principal: [User], resource: [Article] };
action "Article::Undelete::Soft"    appliesTo { principal: [User], resource: [Article] };
action "Version::Create"            appliesTo { principal: [User], resource: [Article] };
action "Version::Read"              appliesTo { principal: [User], resource: [Version] };
action "Version::Update"            appliesTo { principal: [User], resource: [Version] };
action "Version::Delete::Hard"      appliesTo { principal: [User], resource: [Version] };
action "Version::Delete::Soft"      appliesTo { principal: [User], resource: [Version] };
action "Version::Undelete::Soft"    appliesTo { principal: [User], resource: [Version] };
action "Comment::Create"            appliesTo { principal: [User], resource: [Application] };
action "Comment::Read"              appliesTo { principal: [User], resource: [Comment] };
action "Comment::Update"            appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Hard"      appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Transfer"  appliesTo { principal: [User], resource: [Comment] };
action "Comment::Delete::Soft"      appliesTo { principal: [User], resource: [Comment] };
action "Comment::Undelete::Soft"    appliesTo { principal: [User], resource: [Comment] };
action "User::Create"               appliesTo { principal: [User], resource: [Application] };
action "User::Read"                 appliesTo { principal: [User], resource: [User] };
action "User::Update"               appliesTo { principal: [User], resource: [User] };
action "User::Delete::Hard"         appliesTo { principal: [User], resource: [User] };
action "User::Delete::Transfer"     appliesTo { principal: [User], resource: [User] };
action "User::Delete::Soft"         appliesTo { principal: [User], resource: [User] };
action "User::Undelete::Soft"       appliesTo { principal: [User], resource: [User] };
action "Role::Create"               appliesTo { principal: [User], resource: [Application] };
action "Role::Read"                 appliesTo { principal: [User], resource: [Role, Application] };
action "Role::Update"               appliesTo { principal: [User], resource: [Role] };
action "Role::Delete"               appliesTo { principal: [User], resource: [Role] };
action "Role::Grant"                appliesTo { principal: [User], resource: [Role] };
action "Role::Revoke"               appliesTo { principal: [User], resource: [Role] };
action "Tag::Create"                appliesTo { principal: [User], resource: [Application] };
action "Tag::Read"                  appliesTo { principal: [User], resource: [Tag, Application] };
action "Tag::Update"                appliesTo { principal: [User], resource: [Tag] };
action "Tag::Delete"                appliesTo { principal: [User], resource: [Tag] };
action "Tag::Apply"                 appliesTo { principal: [User], resource: [Tag] };
action "Tag::Unapply"               appliesTo { principal: [User], resource: [Tag] };
