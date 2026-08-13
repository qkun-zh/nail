// nail 授权策略（Cedar）：唯一手写处，改权限只改数据（角色/授予边），永不改策略。
// 判定模型见 api_rebuild_design.md §1.4 / cedar_report.md §3（含实测修正）。

// 1. 作者本人：读写管全含（owner 属性组装期从 user_to_article 边冗余填充；
//    Comment::Delete 让评论作者可删自己的评论——与既有行为一致）。
//    管理动作（Version/Comment 的 Update、Delete-hard）不在 owner 白名单里，
//    走策略 3（角色授予）或策略 5（admin）。
permit (principal, action in [Action::"Article::Read", Action::"Version::Read",
                              Action::"Comment::Read", Action::"Comment::Delete",
                              Action::"Article::Update", Action::"Article::Delete",
                              Action::"Version::Create"],
        resource)
  when { resource.owner == principal };

// 2. 显式公开：可读（含 PDF 下载，已并入 Article::Read）。
permit (principal, action in [Action::"Article::Read", Action::"Version::Read",
                              Action::"Comment::Read"],
        resource)
  when { resource.visibility == Visibility::"public" };

// 3. 角色权限 + 作用域：principal 祖先闭包含 action（User in Role in Action
//    传递闭包），且作用域命中（角色 tag 集 ∩ 文章 tag 集非空；无 tag 边的角色
//    = 全局角色）。集合成员判定用 containsAny（in 只用于实体层级）。
permit (principal, action, resource)
  when {
      principal in action
      && (principal.global_role || principal.scopes.containsAny(resource.required_scopes))
  };

// 4. 管理元权限：管理控制台资源（User::Read/Update/Delete、Role::Manage 只对
//    System::"admin-console" 判定，principal 祖先闭包含 action）。
permit (principal, action in [Action::"User::Read", Action::"User::Update",
                              Action::"User::Delete", Action::"Role::Manage"],
        resource)
  when { resource == System::"admin-console" && principal in action };

// 5. admin 全放行：持有 admin 角色 = 所有操作（现在 + 未来）。
permit (principal, action, resource)
  when { principal in Role::"admin" };
