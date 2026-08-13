# 后端 API 重构与权限系统设计（定稿）

- 适用范围：`nail` 后端 API 重构 + 授权域激活
- 日期：2026-08-13
- 状态：**定稿**（经多轮评审收敛，待实施）
- 关联：`permission_system_design.md`（授权域原稿）、`cedar_report.md`（判定层）、`knowledge_base.md`（系统知识库）

---

## 0. 目标与原则

将现有后端 API（业务名词端点：`/authenticate/*`、`/user/logout`、`/email/*`、`/article/search` 等）重构为**统一的对象 + 动作 CRUD 面**，同时激活权限系统（Cedar 授权域：角色/授予/分配全部数据化）。

1. **URL = `/{对象}[/{id}]/{动作}`**，动作 ∈ {create, read, update, delete}，业务名词不出现在 URL；
2. **响应信封统一** `{code, data, message}`；
3. **权限模型一切围绕 Cedar**：动作词汇表在 `schema.cedar`，策略唯一手写处 `policy.cedar`，角色/授予/分配是运行时数据；
4. 一个业务目标 = 一个端点（原子事务），前端不拼装；
5. 协议保留：PoW、邮箱 token、session——安全机制不动，只换形态。

---

## 1. 权限模型（Cedar 为中心）

### 1.1 分层

```
schema.cedar    ← 动作词汇表（16 个 action）+ 实体类型声明（静态，改需重启）
policy.cedar    ← 策略（唯一手写处）
agdb 图数据     ← 角色节点 + 授予/分配/作用域边（运行时动态，纯插边删边）
entity_store + gate ← 组装 + 判定（每请求）
```

### 1.2 权限点矩阵（16 个）

| 节点 | Create | Read | Update | Delete |
|---|---|---|---|---|
| Article | `Article::Create` | `Article::Read` | `Article::Update` | `Article::Delete` |
| Version | `Version::Create` | `Version::Read` | `Version::Update` | `Version::Delete` |
| Comment | `Comment::Create` | `Comment::Read` | `Comment::Update` | `Comment::Delete` |
| User | —（注册匿名自助，无 principal，见 §1.2.1） | `User::Read` | `User::Update` | `User::Delete` |
| Role | `Role::Manage`（单点元权限，管角色 CRUD） | | | |

- **已删除**：`Pdf::Download`（读 ⊃ 下载，并入 `Article::Read`）、`Visibility::Manage`（可见性是 Read 的输入条件，不是权限）。
- **Tag / Permission** 无独立操作面：tag 随引用管理；permission 是静态枚举（schema 反推种子），不进 CRUD。
- **Role::Manage 单点**为定稿（官方 bp-meta-permissions 模式）；如需拆分 CRUD 四点可后置扩展。

#### 1.2.1 为什么 User 没有 Create

权限判定（`gate::authorize`）必须有一个 principal（已认证用户）才能评估。注册发生在登录之前——发起者是匿名访客，无 session、无 user_id、无身份，Cedar 无从判定"谁有权限创建用户"。注册受的保护是 PoW（防 DoS）+ 邮箱证明（防冒用）+ 域名白名单，而非 RBAC。RBAC 管"登录后能做什么"，注册是"进入系统"的边界，边界外无身份可判。`user/create` 端点仍然存在（公开流程），只是不经过 gate。

### 1.3 schema.cedar（动作词汇表唯一来源）

```cedar
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
```

**种子改造**：`seed_permissions` 改为 `Schema::from_str` 解析 → `schema.actions()` 枚举 → 幂等 get-or-create permission 节点（替换手写 `ALL_PERMISSIONS`）。

### 1.4 policy.cedar（最终形态）

```cedar
// 1. 作者本人：读/写/管全含（owner 规则）
permit (principal, action in [Article::Read, Article::Update, Article::Delete,
                              Version::Create, Version::Read, Comment::Read,
                              Comment::Delete],
        resource)
  when { resource.owner == principal };

// 2. 显式公开：可读（含 PDF 下载，已并入 Article::Read）
permit (principal, action in [Article::Read, Version::Read, Comment::Read], resource)
  when { resource.visibility == Visibility::"public" };

// 3. 角色 + 作用域：通配
permit (principal, action, resource)
  when { principal in action
      && (principal.global_role || principal.scopes.containsAny(resource.required_scopes)) };

// 4. 管理元权限：管理控制台资源（User::*/Role::Manage）
permit (principal, action in [User::Read, User::Update, User::Delete, Role::Manage], resource)
  when { resource == System::"admin-console" && principal in action };

// 5. admin 全放行：持有 admin 角色 = 所有操作（现在 + 未来）
permit (principal, action, resource)
  when { principal in Role::"admin" };
```

- `gate::Resource` 枚举加 `System` 变体（`System::"admin-console"`）。
- 管理动作（version/comment 的 Update、Delete-hard）在**具体资源**上判定（走策略 1/3/5）；User::*/Role::Manage 走 System 资源（策略 4/5）。

### 1.5 角色体系

| 角色 | 授予方式 | 能力 |
|---|---|---|
| `admin` | user zero 种子持有 | 策略 5 全放行（所有操作含未来） |
| `member` | 注册自动挂 | `Article::Create` + `Comment::Create`（"登录即可发"） |
| `recycler` | user zero 种子持有，管理可授 | 资产回收接收者（数据层决策，非 gate） |
| 自定义 | 管理创建（Role::Manage） | 按需授予权限点 + 作用域 |

**user zero**：配置 `conf/back/server.toml` 的 `user_zero_email`（现为 `qkun-zh@qq.com`），启动 get-or-create + 挂全部必需角色（`REQUIRED_ROLES` = admin/recycler）。system user 概念已彻底删除（含旧策略 5 特判、`SYSTEM_USER_*` 常量、`ensure_system_user`）。

### 1.6 删除分类

| 分类 | 语义 | 使用场景 |
|---|---|---|
| `transfer`（转移软删） | 所有权转移给回收者，内容保留 | 用户侧（注销/删文/删评论），owner 规则判定 |
| `hard`（递归硬删） | 节点 + 下属 + 边 + PDF（按引用清理）递归删 | 管理侧（version 删 = 版本+评论树+PDF；user 删 = 账号+内容） |
| `clear`（清空软删） | 节点保留、内容清空 | 仅 Comment（后置：正文占位、树不断） |

回收目标选择：名下所有权边（article+comment）最少的回收者，并列取最年轻（uuidv7 id 字典序最大）；注销排除被删账号自身。

---

## 2. API 设计

### 2.1 通用契约

| 项 | 约定 |
|---|---|
| URL 形态 | `/{object}[/{id}]/{action}`，action ∈ {create, read, update, delete} |
| 会话凭证 | 请求头 `session-token`（替代 nail-token；凭证永不进 URL/body） |
| 响应信封 | `{code, data, message}`，code = HTTP 状态码；成功 data 承载业务负载，失败 data=null、message=reason 文案 |
| 分页 | `page`（1-based）+ `limit`；响应 `{…, has_next, total}`（has_next 替代 has_more） |
| 搜索 | `key_word`（自由文本，匹配 title/summary/author/tag/id）、`ranges`、`sort`、`from`、`to`、`page`、`limit` |
| author check | 资源 read 带 `?check_if_is_author=true` → data 加 `is_author` |
| PoW 范围 | 仅 user 自身敏感操作（注册/登录/改名/登出/注销/改邮箱）；管理操作 = session + 权限点门卫，不带 PoW |
| 原子性 | 一个业务目标 = 一个端点（内部单事务）；批量操作后置 |

### 2.2 认证流程（两级对象：user → session）

```
GET  /challenge/read             → 拿 PoW challenge（服务端签发记账）
POST /email/read                 → 请求邮箱验证信（统一入口：单邮箱 body:{pow}，payload=邮箱 → 发邮件；
                                   双邮箱 body:{old_email_pow,new_email_pow} = 改邮箱；带 session 头单 pow = 注销）
POST /user/create                → 注册/登录：body:{pow}（payload=邮箱 token）
                                   验证 → user 不存在建/存在复用 → 内部调 session/create
                                   → {data:{session_token}}
GET  /session/read               → 验证会话（session-token 头；字段选择参数 ?id=&name=）
POST /session/delete             → 登出（头 + body:{pow}）
POST /session/create             → 【内部端点】签发会话，仅后端自调
```

### 2.3 端点表（定稿）

**认证/账号**

```
GET  /challenge/read
POST /email/read                            body: {pow}（单邮箱）/ {old_email_pow,new_email_pow}（改邮箱双邮箱）；带 session 头单 pow = 注销
POST /user/create                            body: {pow:邮箱token} → {data:{session_token}}
POST /session/create                         【内部】
GET  /session/read                           头: session-token; ?id=&name=（字段选择）
POST /session/delete                         头: session-token; body: {pow}
GET  /user/read                              ?page&limit → {user_list, has_next, total}（含 name/email_hash）
GET  /user/{id}/read                         ?name=&email_hash= → {name, email_hash}
POST /user/{id}/update                       改名 body:{pow}; 改邮箱换绑 body:{pow,old_email_token,new_email_token}
POST /user/{id}/delete                       body:{mode:"transfer",pow:邮箱token}（确认步）/ {mode:"hard"}
```

**文章**

```
POST /article/create                         multipart: title/summary/tags/version/note/file
GET  /article/read                           ?key_word&ranges&sort&from&to&page&limit → 搜索结构（列表=空条件搜索）
GET  /article/{id}/read                      ?check_if_is_author=true
POST /article/{id}/update                    body: {title,summary,tags}
POST /article/{id}/delete                    body: {mode}
```

**版本**

```
POST /article/{id}/version/create            multipart: version/note/file
GET  /article/{id}/version/read              ?page&limit
GET  /version/{id}/read                      ?article_id&check_if_is_author=true
GET  /article/{id}/version/{v}/content/read  无参=内联直读(Article::Read) / ?download=1=mint→{url:…?token=} / ?token=consume
POST /version/{id}/update                    body: {note}（管理，权限点门卫）
POST /version/{id}/delete                    body: {mode:"hard"}（版本+评论树+PDF 递归删）
```

**评论**

```
POST /version/{id}/comments/create           body: {content}
POST /comments/{id}/replies/create           body: {content}
GET  /version/{id}/comments/read             ?page&limit（顶层分页）&check_if_is_author=true
POST /comment/{id}/update                    body: {content}（管理）
POST /comment/{id}/delete                    body: {mode}
```

**角色/配置**

```
POST /role/create                            body: {name}
GET  /role/read                              ?page&limit
GET  /role/{name}/read                       → {name,permissions,scopes,members}
POST /role/{name}/update                     body: {permissions/tags/users:{add,remove}}（单事务）
POST /role/{name}/delete                     body: {mode:"hard"}
GET  /config/read                            限额 JSON（原 /meta/limits）
```

---

## 3. 既有端点替代对照

| 旧 | 新 | 变更类型 |
|---|---|---|
| `/authenticate/challenge\|pow\|token\|verify` | `/challenge/read`、`/email/read`、`/user/create`、`/session/read` | 拆分+改名 |
| `/user/logout` | `/session/delete` | 改名 |
| `/user/name`(GET/POST) | `/user/{id}/read`、`/user/{id}/update` | 改名 |
| `/email/check`、`/email/update/send` | 并入 `/email/read`（双邮箱 body） | 合并 |
| `/email/update/confirm` | `/user/{id}/update` | 改名 |
| `/user/deregister[\/confirm]` | 第一步发信 `/email/read`；确认步 `/user/{id}/delete`（mode=transfer） | 改名+分流 |
| `/article`、`/article/{id}`、`/article/{id}/delete` | `/article/read`、`/article/create`、`/article/{id}/read`、`/article/{id}/update`、`/article/{id}/delete` | 改名+动作化 |
| `/article/search` | 并入 `/article/read`（key_word） | 合并（省） |
| `/article/{id}/version`(GET/POST) | `/article/{id}/version/read`、`/article/{id}/version/create` | 改名 |
| `/version/{id}` | `/version/{id}/read` | 改名 |
| `/article/{id}/version/{v}/pdf`、`/download`、`/article/download` | `/article/{id}/version/{v}/content/read`（download=1/token=） | 合并 |
| `/version/{id}/comments`(GET/POST)、`/comments/{id}/replies`、`/comments/{id}/delete` | `/version/{id}/comments/read`、`.../comments/create`、`.../replies/create`、`/comment/{id}/delete` | 改名 |
| `/author/check` | 并入各资源 read（check_if_is_author） | 合并（省） |
| `/meta/limits` | `/config/read` | 改名 |

---

## 4. 落地清单

### 4.1 后端（文件级）

| 文件 | 改动 |
|---|---|
| `common/src/response.rs` | 新增信封 `{code,data,message}`；移除旧 ok/reason 型 |
| `common/src/request.rs` | 请求类型调整（mode、check_if_is_author、key_word 等） |
| `common/src/*` | `session-token` 常量；`has_next` |
| `back/src/api.rs` | 路由表重建；`require_session` 读新头 |
| `back/src/api/*.rs` | 13 个 handler 改名 + 信封 + 参数调整 |
| `back/src/api/article_search.rs`、`author.rs` | 删除（并入 read） |
| `back/src/api/meta.rs` | → `config/read` |
| `back/src/logic/*` | 复用核心；新增角色 CRUD、user/version/comment 管理、递归硬删、key_word 搜索 |
| `back/src/authorization/` | `schema.cedar`、`Resource::System`、管理门卫、admin 策略、schema 反推种子 |
| `back/src/repo/*` | 管理数据层、递归硬删、PDF 引用清理 |

### 4.2 前端（文件级）

| 文件 | 改动 |
|---|---|
| `front/src/req/request/*.rs` | handler 换端点 + 解包 data + `session-token` |
| `front/src/req.rs` | re-export 更新 |
| 页面调用点 | `resp.xxx` → `data.xxx`；`has_more` → `has_next` |

### 4.3 实施顺序（每步可独立验证）

1. **契约层**：响应信封 + session-token 头 + has_next（不动端点，回归）
2. **端点改名**：机械替换 URL（逻辑不动），回归
3. **认证域**：challenge/token/user/session 重构
4. **内容域**：article/version/comment + 搜索合并 + 评论分页 + author check 参数化
5. **管理域**：role/user 管理 + schema.cedar + admin 策略 + 硬删
6. **前端同步** + 测试全量（380+ 用例重写端点/断言）

### 4.4 后置项（不进本期）

- 批量操作（`ids` 数组单事务）
- Comment `clear`（正文占位、树不断）
- Tag 合并 / Role 合并（转移语义）
- Cedar schema validator 启动校验（可选增强）
