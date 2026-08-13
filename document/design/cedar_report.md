# Cedar 权限判定层 可行性调研 + 迁移知识库

- 调研对象：`cedar-policy` 4.12.0（crates.io，AWS Cedar 官方 Rust crate）+ 独立探针 `probe/cedar_probe/`
- 实测方式：`cargo add cedar-policy`（161 个依赖包，首次编译 ~1m）
- 探针覆盖：权限设计文档 §4 判定模型 10 项核心语义，10/10 全绿（`cargo run --manifest-path probe/cedar_probe/Cargo.toml`）
- 日期：2026-08

---

## 0. 结论速览

1. **cedar-policy 是 nail 授权判定层的匹配替代**：嵌入式 Rust 库，`Authorizer::is_authorized(request, policies, entities)` 单次调用完成判定；策略文件与代码分离；默认拒绝（fail-closed）。
2. **权限设计文档 §4 的策略草案经实测需三处修正**（详见 §3）：
   - 角色权限判定用**纯层级** `principal in action`（`User in Role in Action` 传递闭包），文档原 `action in principal.roles.permissions` 属性链**不可行**（Cedar 的 Set 不支持链式属性访问）；
   - owner 规则必须**补读操作**（否则私有文章的 owner 自己读不了）；
   - **权限点即 Action 实体**：DB 的 `permission` 节点在 entity store 组装为 `Action` 实体（Role 的 parents），`Permission`/`Action` 两个概念合并。
3. 实体层级（`in` 传递闭包）、可见性、授予（`visible_to`）、context、forbid 优先、缺失属性容错全部实测通过。
4. 判定层与存储层完全解耦：Cedar 只吃 `Entities`（内存图），数据来自 agdb 组装（`permission_probe` 已验证图模型），两层可独立演进。

---

## 1. 核心 API 知识（迁移必读）

### 1.1 库形态

```rust
use cedar_policy::{
    Authorizer, Context, Decision, Entities, Entity, EntityUid, PolicySet, Request,
    RestrictedExpression,
};

// 策略集：从字符串解析（策略文件与代码分离，可 include_str! 或独立文件）
let policies: PolicySet = include_str!("policy.cedar").parse()?;

// 实体集：组装（agdb 图遍历结果 → Cedar 实体，见 §2）
let entities = Entities::from_entities(vec![/* Entity */], None)?;

// 请求 + 判定（无 schema 模式，实体类型自由）
let request = Request::new(principal_uid, action_uid, resource_uid, context, None)?;
let resp = Authorizer::new().is_authorized(&request, &policies, &entities);
// resp.decision(): Decision::Allow / Deny
// resp.diagnostics().errors(): 判定过程错误（缺失属性/实体等，fail-closed 可观测）
```

### 1.2 实体构造

```rust
// Entity::new(uid, attrs: HashMap<String, RestrictedExpression>, parents: HashSet<EntityUid>)
// parents = 层级祖先（in 的传递闭包基础）
let user = Entity::new(
    "User::\"u1\"".parse()?,
    HashMap::from([("roles".into(), RestrictedExpression::from_str(r#"[Role::"operator"]"#)?)]),
    HashSet::from(["Role::\"operator\"".parse()?]),
)?;
// attrs 值用 RestrictedExpression 字符串：字符串/数字/实体引用/集合
//   r#""public""#  r#"Visibility::"public""#  r#"[Role::"reviewer"]"#
```

### 1.3 关键语义（实测）

| 语义 | 写法 | 说明 |
|---|---|---|
| 实体层级 | `a in b` = **a 是 b 的后代或相等** | `principal in Role::"admin"` |
| 传递闭包 | `User in Role in Action` | parents 链自动闭包（`Entities::from_entities` 计算） |
| 角色权限 | `principal in action` | principal 祖先闭包含 action（§3-1 实证） |
| 资源父链 | `resource in Article::"a1"` | Comment in Version in Article |
| 可见性 | `resource.visibility == Visibility::"public"` | 实体属性比较 |
| 授予 | `principal in resource.visible_to` | visible_to 是资源上的实体集合属性 |
| context | `context.flag == true` | `Context::from_pairs` 构造 |
| 默认拒绝 | 无匹配 → Deny | fail-closed，不产生诊断错误 |
| forbid 优先 | forbid 命中覆盖 permit | 显式拒绝优先 |
| 缺失属性 | Deny + 诊断错误（非 panic） | 可观测，生产需测试锁死 |

> ⚠️ **策略文件不需要 `entity` 声明**（那是 schema 文件的事）：`entity User, Role;` 写在策略里会解析报错。无 schema 模式下实体类型自由。
> ⚠️ **action 是实体 uid**：策略里写 `Action::"Article::Update"`，Request 里同形，不是裸标识符。
> ⚠️ **Set 不支持链式属性**：`principal.roles.permissions`（roles 是集合）报类型错误——集合属性只能单层访问。

---

## 2. 实体组装映射（agdb → Cedar，与 permission_probe 衔接）

| agdb（权限探针已验证） | Cedar 实体 | 组装方式 |
|---|---|---|
| `user` 节点 | `User::"u1"` | attrs: owner 相关字段；parents 见下 |
| `user_hold_role` 边 | `User in Role` | **parents**（不是属性！） |
| `role_grant_permission` 边 | `Role in Action` | **parents** |
| `permission` 节点 | `Action` 实体 | 权限点即 action，统一命名（§3-3） |
| `article.visibility` 字段 | `resource.visibility` 属性 | attrs |
| `user_own_article` 边 | `resource.owner` 属性 | attrs（entity 引用） |
| `article_share_viewer` 边（扩展点） | `resource.visible_to` 属性 | attrs（entity 集合） |
| 版本/评论父链 | `Version in Article`、`Comment in Version` | parents + owner/visibility 冗余填充（设计文档 §4 要点） |

判定是纯内存操作（微秒级）；组装是每次授权一次 agdb 图遍历（`permission_probe` P6 已验证形态）。

---

## 3. 探针实测（10/10 全绿）

| 探针 | 验证内容 | 结果 |
|---|---|---|
| C1 | 策略解析 + owner 基础判定 | PASS |
| C2 | 默认拒绝（fail-closed，无匹配 → Deny 且无诊断错误） | PASS |
| C3 | owner 判定边界（非 owner 拒绝 / owner 删除放行） | PASS |
| C4 | 可见性判定（public 任何人读 / private 非 owner 拒 / owner 可读） | PASS |
| C5 | 角色权限 `principal in action` 传递闭包（有权限 Allow / 无权限 Deny） | PASS |
| C6 | 资源父链（Comment in Version in Article，`resource in Article::"a1"`） | PASS |
| C7 | 授予判定（`principal in resource.visible_to`，角色传播） | PASS |
| C8 | context 参数（true/false 分支） | PASS |
| C9 | forbid 优先（显式拒绝覆盖 permit） | PASS |
| C10 | 缺失属性/实体 → Deny + 诊断错误，不 panic | PASS |

### 探针发现的设计修正（对应 permission_system_design.md §4）

1. **角色权限规则改写**：`when { action in principal.roles.permissions }` → `when { principal in action }`。原因：① `principal.roles.permissions` 是属性链，roles 是 Set，Cedar 不支持 Set 链式属性（TypeError 实测）；② 纯层级 `User in Role in Action` + `principal in action`（in = 左侧是右侧后代）语义正确且更简单，entity store 组装 = 全部走 parents。
2. **owner 规则补读操作**：原 action 列表只有写/管理操作，导致私有文章 owner 自己不可读（C4 实测暴露）。修正后列表 = 读操作（Article::Read/Version::Read/Comment::Read）+ 写操作（Article::Update/Article::Delete/Version::Create）+ Visibility::Manage + Pdf::Download。
3. **权限点即 Action 实体**：DB `permission` 节点（`Article::Update` 等）组装为 `Action` 实体，`role_grant_permission` 边 → `Role in Action` parents。`Permission`/`Action` 概念合并，消除设计文档 §3/§4 的两套命名。

### 最终策略（修正版，与设计文档 §4 对照）

```cedar
// 1. 作者本人：读写管全含（修正点 2）
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

// 3. 角色权限：principal 祖先闭包含 action（修正点 1，权限点即 action 见修正点 3）
permit (principal, action, resource)
  when { principal in action };

// 4.（扩展点）私密授予
// permit (principal, action in [...读...], resource)
//   when { principal in resource.visible_to };

// 5. 系统用户：资产接管后管理动作单独放行
permit (principal == User::"system", action, resource);
```

---

## 3.1 联调验证（M1c `probe/integration_probe/`，8/8 全绿）

- 探针：`cargo run --manifest-path probe/integration_probe/Cargo.toml`（依赖 agdb 0.13.2 + cedar-policy 4.12）
- 链路：**agdb 建图 → entity store 组装（parents 链 + attrs）→ Cedar 判定**，即未来 `authorization/entity_store.rs` + `gate.rs` 的雏形

| 探针 | 验证内容 | 结果 |
|---|---|---|
| I4 | owner 可更新自己的私有文章（owner 属性从 `user_own_article` 入边组装） | PASS |
| I5 | 可见性：公开任意用户可读 / 私有非 owner 拒 / owner 可读 | PASS |
| I6 | 角色权限跨 owner：operator 持 `Article::Update` 可更新他人私有文章；无读权限则拒 | PASS |
| I7 | fail-closed：无权限无归属 → Deny | PASS |
| I8 | **动态改权限**：agdb 插 `role_grant_permission` 边 → 重组装 → 判定翻转（权限数据化生效） | PASS |
| I9 | **全局角色（无 tag 边）**：跨 tag 下载私有文章（不受 tag 限制） | PASS |
| I10 | **tag 作用域（`role_apply_tag`）**：tag 交集放行 / 无交集拒绝 / 多 tag 交集命中 | PASS |
| I11 | scope 不影响公开文章（走 visibility 规则，无限制） | PASS |

**联调实证的两个组装要点**：

1. **边 → parents 链**：`user_hold_role` → `User in Role`，`role_grant_permission` → `Role in Action`（组装函数 `assemble_user_auth`）；
2. **反向遍历含起点**：agdb `search().to(article)` 返回起点自身（distance 0），组装 owner 必须按 `type` 过滤出真用户（I4 踩坑实证）——已写入设计文档 §4 要点 ⚠️。

### 3.1.1 作用域（scope）——角色直接挂 tag 边（I9-I11 实证）

> 设计修正（终版）：不引入 scope 概念、不存 scope_type/scope_id 字符串属性——**角色的作用域就是 `role_apply_tag` 边指向的 tag 节点**（与 `article_apply_tag` 完全平行）。无 tag 边的角色 = 全局角色。判定 = 角色 tag 集合 ∩ 文章 tag 集合（Tag 实体集合交集非空）：

```cedar
// 策略 3 最终版：角色权限 + 作用域限制
permit (principal, action, resource)
  when {
      principal in action
      && (principal.global_role || principal.scopes.containsAny(resource.required_scopes))
  };
```

> ⚠️ **实测发现：集合交集用 `.containsAny()`，成员判断用 `.contains()`，不是 `in`**——Cedar 的 `in` 只用于实体层级（`a in b` = a 是 b 的后代）。
> 组装：`scopes`/`global_role` 恒为属性（无角色/无 tag 也有值），避免缺属性诊断错误；`&&` 短路保证无权限用户不评估作用域分支。
> **tag 清理统一**：tag 无 `article_apply_tag` 边 **且** 无 `role_apply_tag` 边时才可删（孤儿判定扩展）；`role_apply_tag` 边删除不影响 tag 其它引用。

---

## 4. 与其它报告的衔接

- **agdb 报告 §3.1**（permission_probe）：数据层图模型 10/10 验证；本文验证判定层 10/10；§3.1 联调 5/5 全链路。三层组合 = **agdb 存图（节点/边/属性）→ entity store 组装（图遍历 → parents/attrs）→ Cedar 判定（内存）**。
- **权限设计文档**（permission_system_design.md）：§4 策略已按 §3 三处修正同步更新（含组装映射表 + 反向遍历 ⚠️）；§6 代码结构 `authorization/` 模块的 `entity_store.rs` 组装语义 = parents 链 + attrs 冗余填充（本文 §2 映射表）。
- **读写分离不变**：读路径（列表/搜索）仍走 agdb 查询过滤候选集，Cedar 只做单资源权威判定——Cedar 无法下推列表过滤（业界一致）。

---

## 5. 风险与待办

1. **作用域（scope）已解决（I9-I11 实证）**：角色直接挂 `role_apply_tag` 边（与 `article_apply_tag` 平行），无 tag 边 = 全局角色；判定 = Tag 实体集合 `containsAny` 交集（§3.1.1）。**未决**：tag 孤儿判定需同时检查 `article_apply_tag` 与 `role_apply_tag` 两条边（无任何引用才可删）——现有 tag 清理逻辑扩展，无需探针。
2. **缺失属性静默收紧**：组装遗漏属性 → Deny + 诊断错误（fail-closed 安全方向，但难排查）。生产组装函数需配套测试锁死（permission_probe P6 形态 + 判定层用例 + integration_probe I4 的 type 过滤）。
3. **无 schema 模式**：实体类型自由、无编译期校验。可选后续引入 `Schema`（`Schema::from_str`）做策略类型检查 + 启动时验证，与"权限点单一来源"的启动三重校验（设计文档 §6）结合。
4. **判定性能**：纯内存微秒级；`(user, action, resource)` 结果可短 TTL 缓存（设计文档 §10 已有）。
5. **策略文件管理**：单文件 `policy.cedar`，改权限只改数据不改策略（§3 规则 3 通配）；策略本身变更走代码评审（版本管理）。
6. **上游跟进**：`principal.roles.permissions` 属性链报错为预期行为（非 bug），但文档化差异值得记录；Set 链式属性若未来支持可简化模型，关注 Cedar release notes。
7. **联调探针保留**：`probe/integration_probe/` 作为组装链路语义基线（含反向遍历 type 过滤坑）；三探针（permission/cedar/integration）共同锁定设计文档的图模型 + 判定模型。
