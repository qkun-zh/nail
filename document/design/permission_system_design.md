# 权限系统设计（授权域重构）

- 适用范围：`nail_new` 完全重构项目
- 日期：2026-08
- 状态：**已确认稿**（角色 RBAC 图化 + fail-closed 可见性 + Cedar 策略判定 + 读路径查询过滤）
- 探针验证：`permission_probe`（agdb 图模型 10/10）+ `cedar_probe`（判定层 10/10）+ `integration_probe`（联调 8/8，含 scope），全绿
- 关联：`rule.md`（nail_new 编码规则）；`agdb_migration_report.md` §3.1（数据层）；`cedar_report.md`（判定层 + 实测修正）

---

## 0. 设计总纲（原则）

1. **fail-closed（默认拒绝）**：无匹配策略即 Deny；新文章默认 `private`。数据损坏/误删边只会让权限收紧，不会意外公开。
2. **读写分离**：写路径 = Cedar 权威判定；读路径（列表/搜索）= SQL 图遍历过滤候选集 + Cedar 兜底单资源。
3. **策略与代码分离**：权限规则只在 `.cedar` 一个文件；角色、授予、可见性全部是数据。加角色/改权限 = 插边删边，永不改代码。
4. **单点权威**：前端显示层门面与后端强制层调用同一策略，语义永不漂移。
5. **认证不动**：PoW / session / 邮箱 token 流程原样保留，不在授权域范围内。
6. **纯图模型**：节点 + 边，无第二个授权概念；边命名主谓宾式，完全无歧义。

---

## 1. 决策记录（为什么这样做）

| 决策 | 结论 | 理由 |
|---|---|---|
| 角色模型 | `role` 节点动态增删（同 `tag` 模式），`user_hold_role` / `role_grant_permission` 两条边 | RBAC 图化（NIST 三层模型）；角色=数据，加角色零代码 |
| 权限点 | `permission` 静态枚举节点；角色→权限用边挂载；**权限点即 Cedar Action 实体**（组装为 `Role in Action` parents） | 加权限点不改策略（通配 permit），只改数据；消除 Permission/Action 两套命名（cedar_report §3） |
| 角色权限判定 | **纯层级 `principal in action`**（`User in Role in Action` 传递闭包） | `action in principal.roles.permissions` 属性链实测不可行（Set 不支持链式属性）；层级组装 = 边 → parents |
| 作用域（scope） | **角色直接挂 tag 边**：`role_apply_tag`（与 `article_apply_tag` 平行）；无 tag 边 = 全局角色；判定 = Tag 实体集合**交集非空**（`.containsAny`） | 不引新概念、不存字符串属性；联调 I9-I11 实证（全局跨 tag / tag 交集放行外拒 / 公开不受限） |
| 可见性默认 | `visibility` 字段默认 `private` | fail-closed 业界实践（云存储/Google Drive/GitHub 均默认私有） |
| ~~无边=公开~~ | **否决** | fail-open：误删边/漏插边会让私有内容公开，违背默认拒绝 |
| 可见性表达 | `visibility` 字段（状态）+ 授予边（授权，扩展点） | 字段管默认、可索引快路径；边管细粒度动态放行 |
| 边命名 | 主谓宾式：`user_own_article` | 三元组唯一性 ⇒ 绝对无歧义；同端点多边、同动词多端点均不冲突 |
| 多态边 | 用语义宾语：`article_share_viewer` | 目标类型不写死（user/role 均可） |
| 授予边 `article_share_viewer` | **扩展点，可延后** | 无"私密分享给指定人"需求时整条边不存在；fail-closed 下延后无风险 |
| 权限点单一来源 | `.cedar` 文件 → build.rs 生成 Rust 枚举 → 启动时同步 DB 种子 | 加权限点只改一处，其余自动对齐，杜绝漂移 |
| 评论删除规则 | 评论作者可删自己的（`user_write_comment` 边）+ 角色权限 | 修掉旧规则"只有文章作者能删评论"过粗的问题 |
| 系统用户 | `SYSTEM_USER_ID` 不走角色系统，策略单独放行 | 资产接管后的管理动作与普通用户体系隔离 |

---

## 2. 图模型

### 2.1 节点

| 节点 | 类型 | 说明 |
|---|---|---|
| `user` | 已有 | 用户 |
| `article` / `version` / `comment` | 已有 | 资源；`version` / `comment` 沿父链继承可见性 |
| `tag` | 已有 | 标签 |
| `role` | **新增** | 动态角色，同 tag 模式（唯一索引 + get-or-create + 孤儿清理） |
| `permission` | **新增** | 静态权限点枚举，种子创建，运行时只读 |

### 2.2 边（主谓宾命名，无时态无复数，动词原形）

| 边 | 定义 | 语义 | 来源 |
|---|---|---|---|
| `user_own_article` | `FROM user TO article` | 用户拥有文章 | 已有（旧名 `user_to_article`） |
| `user_write_comment` | `FROM user TO comment` | 用户撰写评论 | 已有（旧名 `user_to_comment`） |
| `article_contain_version` | `FROM article TO version` | 文章包含版本 | 已有（旧名 `article_to_version`） |
| `article_apply_tag` | `FROM article TO tag` | 文章应用标签 | 已有（旧名 `article_to_tag`） |
| `user_hold_role` | `FROM user TO role` | 用户持有角色 | **新增** |
| `role_grant_permission` | `FROM role TO permission` | 角色授予权限 | **新增** |
| `article_share_viewer` | `FROM article TO user/role` | 文章分享给可见者 | **扩展点**（有私密分享需求才建） |

命名规则：**`主语_动词原形_宾语`**，主语宾语与 FROM/TO 端点严格一致；多态目标用语义宾语（`viewer`）。

### 2.3 属性字段（仅"状态"，非授权粒度）

```surql
DEFINE FIELD visibility ON article TYPE string DEFAULT "private"
  ASSERT $value IN ["private", "public"];
DEFINE FIELD description ON role TYPE option<string>;
```

---

## 3. 权限点（`permission` 静态枚举，即 Cedar Action 实体）

> 实测修正（`cedar_report.md` §3）：**权限点即 Action 实体**——DB 的 `permission` 节点在 entity store 组装为 Cedar `Action` 实体（`Role in Action` parents），`Permission`/`Action` 概念合并，消除两套命名。

```
Article::Read         Article::Create        Article::Update        Article::Delete
Version::Read         Version::Create
Comment::Read         Comment::Create        Comment::Delete
Pdf::Download
Visibility::Manage                           -- 改可见性/授予（作者本人恒有）
Role::Manage           User::Manage          -- 管理类（预留）
```

---

## 4. Cedar 策略（唯一手写处，永不因数据变化而改）

> 实测修正（`cedar_report.md` §3，联调探针 `probe/integration_probe/` 5/5 全绿）：
> ① 策略文件不需要 `entity` 声明（那是 schema 的事）；
> ② action 是实体 uid `Action::"..."`；
> ③ 角色权限用纯层级 `principal in action`（`in` = 左侧是右侧后代；`User in Role in Action` 传递闭包），
>    原 `action in principal.roles.permissions` 属性链不可行（Cedar 的 Set 不支持链式属性）；
> ④ owner 规则含读操作（否则私有文章 owner 自己不可读）。

```cedar
// 1. 作者本人：读/写/管全含（owner 属性组装期从 user_own_article 边冗余填充）
permit (principal, action in [Action::"Article::Read", Action::"Version::Read",
                              Action::"Comment::Read", Action::"Article::Update",
                              Action::"Article::Delete", Action::"Version::Create",
                              Action::"Visibility::Manage", Action::"Pdf::Download"],
        resource)
  when { resource.owner == principal };

// 2. 显式公开：可读
permit (principal, action in [Action::"Article::Read", Action::"Version::Read",
                              Action::"Comment::Read", Action::"Pdf::Download"],
        resource)
  when { resource.visibility == Visibility::"public" };

// 3. 角色权限 + 作用域：principal 祖先闭包含 action，且作用域命中
//    （角色作用域 = role_apply_tag 边；无 tag 边 = 全局角色；交集判定；联调 I9-I11 实证）
permit (principal, action, resource)
  when {
      principal in action
      && (principal.global_role || principal.scopes.containsAny(resource.required_scopes))
  };

// 4.（扩展点）私密授予：principal 在文章的可见者集合里
// permit (principal, action in [Action::"Article::Read", Action::"Version::Read",
//                               Action::"Comment::Read", Action::"Pdf::Download"],
//         resource)
//   when { principal in resource.visible_to };

// 5. 系统用户：资产接管后的管理动作单独放行
permit (principal == User::"system", action, resource);
```

要点：**entity store 组装映射**（联调探针实证，`integration_probe` 的 `assemble_user_auth`/`assemble_article`）：

| agdb | Cedar | 组装 |
|---|---|---|
| `user_hold_role` 边 | `User in Role` | parents |
| `role_grant_permission` 边 | `Role in Action` | parents |
| `permission` 节点 | `Action` 实体 | 权限点即 action |
| `user_own_article` 边 | `resource.owner` 属性 | attrs（entity 引用） |
| `article.visibility` 字段 | `resource.visibility` 属性 | attrs（`Visibility::"public"`） |
| `role_apply_tag` 边（新增，与 `article_apply_tag` 平行） | `principal.scopes` 属性 + `principal.global_role` | attrs（Tag 实体集合；无 tag 边的角色 → `global_role=true`；恒有值防缺属性） |
| `article_apply_tag` 边（现有） | `resource.required_scopes` 属性 | attrs（文章 Tag 实体集合） |
| 版本/评论父链 | `Version in Article` / `Comment in Version` | parents + owner/visibility 冗余填充 |

> 实测：字符串集合成员判断用 `.contains()`（`in` 只用于实体层级，见 `cedar_report.md` §3.1.1）。
> 组装时 scopes 恒为属性（避免缺属性诊断错误）；`&&` 短路保证无权限用户不评估 scope 分支。

⚠️ agdb 反向遍历返回**包含起点本身**（distance 0）——组装 owner 时须按 `type` 过滤出真用户（联调 I4 踩坑实证）。

---

## 5. SurrealDB Schema（增量）

```surql
-- 节点
DEFINE TABLE role TYPE NODE SCHEMAFULL;
DEFINE FIELD name ON role TYPE string;
DEFINE INDEX role_name_unique ON role FIELDS name UNIQUE;

DEFINE TABLE permission TYPE NODE SCHEMAFULL;
DEFINE FIELD name ON permission TYPE string;
DEFINE INDEX permission_name_unique ON permission FIELDS name UNIQUE;

-- 边
DEFINE TABLE user_hold_role TYPE RELATION FROM user TO role SCHEMAFULL;
-- 角色作用域：直接挂 tag 节点（与 article_apply_tag 平行）；无 tag 边 = 全局角色
DEFINE TABLE role_apply_tag TYPE RELATION FROM role TO tag SCHEMAFULL;

DEFINE TABLE role_grant_permission TYPE RELATION FROM role TO permission SCHEMAFULL;

-- 扩展点（有私密分享需求时启用）
-- DEFINE TABLE article_share_viewer TYPE RELATION FROM article TO record;

-- 字段（fail-closed：默认私有）
DEFINE FIELD visibility ON article TYPE string DEFAULT "private"
  ASSERT $value IN ["private", "public"];
```

既有边（`user_own_article` 等）在 nail_new 重构中随 schema 一起以新名建立；`user_hold_role` / `role_grant_permission` 的写入必须沿用 tag 的事务模式：guard 写防悬空边、孤儿角色清理（`count(<-user_hold_role) == 0`）。

---

## 6. 代码结构（遵循 nail_new 规则：无 mod.rs、无缩写、行长限制）

```
back/src/authorization/          ← 新：授权域
  policy.cedar                   ← 策略文件（include_str! 或独立加载）
  action.rs                      ← Action 枚举（build.rs 从 policy.cedar 生成）
  gate.rs                        ← authorize(state, user_id, action, resource) -> Result<(), LogicError>
  entity_store.rs                ← 组装 Cedar 实体（一次图遍历 + 父链冗余填充）
back/src/repository/authorization.rs  ← 角色/边/授予的增删查（grant/revoke/visible_to）
```

`authorize` 统一闸门替换旧逻辑层的 4 处手写 `if author_id != user_id { 403 }` 与 `author.rs` 门面。

**权限点单一来源**：`policy.cedar` → build.rs 生成 Rust `Action` 枚举（编译期一致）→ 启动时 ensure DB `permission` 节点（缺则建、多余则报警）。三重对齐一次启动完成。

---

## 7. 读侧查询（候选集过滤，写路径之外的唯一授权触点）

```surql
-- 可见文章 = 公开 OR 我拥有 OR 我/我的角色在授予边里（扩展点）
SELECT * FROM article
WHERE visibility = "public"
   OR id IN (SELECT VALUE out FROM user_own_article WHERE in = $me)
   -- 扩展点：
   -- OR id IN (SELECT VALUE in FROM article_share_viewer WHERE out = $me)
   -- OR id IN (SELECT VALUE in FROM article_share_viewer WHERE out IN
   --     (SELECT VALUE out FROM user_hold_role WHERE in = $me))
```

搜索（`search_articles`）、版本列表、评论列表、`get_public_pdf_path` 全部收敛到同一过滤函数。Cedar 不下推列表过滤（逐条判定太慢），但详情/写操作由 Cedar 兜底——SQL 粗筛 + Cedar 精判，双层防线。

---

## 8. 请求流程

| 场景 | 流程 |
|---|---|
| 写操作（改/删/建版本/删评论） | session → user_id → 组装实体 → `authorize()` → Allow 继续 / Deny 403 |
| 列表/搜索 | SQL 图遍历过滤候选集 → 返回；不逐条跑 Cedar |
| 打开详情 | 候选集已过滤，Cedar 再判定一次（防御纵深） |
| 建角色 | 插 `role` 节点 + `role_grant_permission` 边（`Role::Manage` 权限） |
| 挂角色 | 插 `user_hold_role` 边（带 scope 属性） |
| 发布/撤回 | 置 `visibility = public/private` |
| 私密分享（扩展点） | 插/删 `article_share_viewer` 边 |

---

## 9. 迁移顺序（每步可独立验证，行为不变优先）

| 步骤 | 内容 | 行为影响 |
|---|---|---|
| 1 | schema：role/permission 节点 + 2 条新边 + `visibility` 字段；**回填存量文章 `visibility = "public"`**（保持现状）；既有边以新名建立 | 无 |
| 2 | 引入 `cedar-policy` crate，authorization 模块 + entity store 组装 + 启动时三重校验 | 无 |
| 3 | 4 处写操作 + `/author/check` 换 `authorize()`（策略暂只含 owner 规则，行为等价） | 无（跑既有 e2e 验证无回归） |
| 4 | 可见性生效：建文默认 private、`POST /article/{id}/visibility` 发布接口、列表过滤 | 新能力 |
| 5 | 角色管理接口（`Role::Manage`）+ 评论删除规则变细（评论作者可删自己的）+ 下载资格走策略 | 规则细化，单独发布 |
| 6 | 扩展点：授予边 `article_share_viewer`（有需求时） | 新能力 |

---

## 10. 风险与对策

| 风险 | 对策 |
|---|---|
| 老文章回填漏了变全私有 | 回填 `UPDATE article SET visibility = "public" WHERE visibility IS NONE`，e2e 验证 |
| 评论删除规则变化（文章作者删 → 评论者删自己的） | 行为变更，列在迁移第 5 步单独发布，不混入无回归阶段 |
| entity store 组装性能 | 每次授权一次图查询 + Cedar 判定（微秒级）；可对 `(user, action, resource)` 结果做短 TTL 缓存 |
| 系统用户资产 | `SYSTEM_USER_ID` 单独策略放行，不走角色体系 |
| 权限点三处漂移（Cedar/Rust/DB） | 单一来源 + 生成 + 启动校验，见 §6 |
| 角色动态增删产生悬空边 | 沿用 tag 的 guard 写 + 孤儿清理事务模式 |
