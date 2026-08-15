// nail 授权动作词汇表（Cedar schema）：16 个 action + 实体类型声明（静态，
// 改需重启）。动作词汇表的唯一手写处——`seed_permissions` 从本文件反推权限
// 点种子，角色授予只认这里声明的 action 名。
// 判定模型见 api_rebuild_design.md §1.3。

entity User in [Role] { global_role: Bool, scopes: Set<Tag> };
entity Role;
entity Tag;
entity Visibility;
entity Article  { owner: User, visibility: Visibility, required_scopes: Set<Tag> };
entity Version  { owner: User, visibility: Visibility, required_scopes: Set<Tag> };
entity Comment  { owner: User, visibility: Visibility, required_scopes: Set<Tag> };
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
