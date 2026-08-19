# 屎山风险审查报告

审查范围：全部 ~18.5k 行 Rust（back / front / common 三层）加未提交改动。
审查日期：2026-08-19。

文件尺寸基本守住了 README 的 512 行上限，真正的问题不是"大"，而是大量复制粘贴、约定漂移、死代码和跨层泄漏。

---

## 一、系统性复制粘贴（最致命，是屎山的根）

### 1. 距离1的 typed-edge 查询被复制约 30 次（repository 层）

`search().from().distance(Equal(1)).and().edge().and().key().value()` 手写散布在：
- `code/back/src/repository/article.rs:228,299,336,351,475,519`
- `code/back/src/repository/comment.rs:171,204,290,322,392,421,473,494`
- `code/back/src/repository/delete.rs:24,39,93,282,306,330,409,433,459,482`
- `code/back/src/repository/role.rs:115,146,184,204,240,344,421,446`
- `code/back/src/repository/version.rs:87,251,281`
- `code/back/src/repository/tag.rs:147,172,199,236,260`
- `code/back/src/repository/transfer.rs:121,148,187,252`
- `code/back/src/repository/search/document.rs:123,175,305,325,351`
- `code/back/src/repository/search/db.rs:56,134,160`
- `code/back/src/repository/authorization.rs:72,114,342`

任何 schema/查询改动要改 50 个地方。应加共享助手
`graph::outgoing_edges(guard, from, edge_type)` / `incoming_edges(guard, to, edge_type)` / `edge_count`。

### 2. 分页逻辑复制 7 + 6 处

- `clamp_page_limit(...)` 块复制在 `code/back/src/interface/user.rs:26`、`comment.rs:65`、`comment.rs:93`、`version.rs:73`、`role.rs:38`、`tag.rs:38`
- 5 个相同的 `{ page, limit }` 参数结构体（user/comment/version/role/tag）
- offset/slice/`has_next` 三段式又手写在 `code/back/src/logic/user.rs:132`、`comment.rs:92`、`comment.rs:136`、`version.rs:182`、`search.rs:56`、`role.rs:66`、`tag.rs:45`
- 而 `logic/pagination.rs` 只被一处调用

应做成一个 `AppPaged<T>` 提取器 + 一个 `paginate()`。

### 3. 软删 / undelete / transfer 骨架跨文件复制（logic 层）

`Some(DeleteMode::Soft)/Hard/Transfer` 三档骨架在 `article.rs`、`code/back/src/logic/version.rs:240`、`comment.rs:269` 重复；
"already soft-deleted" / "not soft-deleted" 字符串哨兵多次出现（`article.rs:214,246`、`version.rs:245,301`、`comment.rs:274,308`）；
transfer 错误映射在 `comment.rs:344` 是命名 mapper、在 `article.rs:167` 是内联 match。

### 4. 前后端各自有一套 CRUD 骨架模板

- 后端：`_sync` / `_in_txn` 每个图助手定义两遍（`graph.rs`、`delete.rs:117/131`）
- 前端：`code/front/src/page/article/version/comment/state.rs` 的 `build_submit_*` 一个函数复制 5 遍（128/167/214/276/325），约 246 行；约 12 个页面的 `Effect+Option+render` 异步加载骨架几乎相同
- 前端多页面的 `Effect::new + spawn_local + match` 异步加载模式重复（role/tag/user/article/version 等 detail/list 页）

### 5. "最新版本"有三种实现，一种还是错的

- `code/back/src/repository/version.rs:99`（semver 解析取 max）
- `code/back/src/repository/article.rs:471` `live_latest_version`（semver max）
- `code/back/src/repository/delete.rs:478` `refresh_latest_version_in_txn`：`delete.rs:501` 用字符串 `.max()`，对 `10.0.0 < 9.9.9` 语义错误

同一领域概念应只有一个 `highest_version_number(rows)` 助手。

### 6. 内容哈希去重逻辑复制（logic 层）

`reject_duplicate_content_hash`（`article.rs:257`）与 `create_version` 内联块（`version.rs:93`）功能相同，
"identical PDF already exists" 字符串及 `ContentHashTaken` 变体多次出现。

### 7. 令牌生命周期复制约 12 处（logic 层）

normalize → token_key → hash → cache 流程在 `user.rs`、`email.rs`、`download.rs`、`session.rs` 重复，
错误字符串各自漂移（"email token"/"delete token"/"session token"/"download token"）。
`user.rs` 的 transfer/soft-delete 尾部复制了 5 行 cache 清理。

---

## 二、公共层漂移（common 是扩散源）

### A1. 三个几乎相同的验证器

`code/common/src/name.rs:11`、`tag.rs:18`、`text.rs:8` 各自实现"trim→拒空→扫字符→拒禁→限长"，
`{Empty, TooLong, ContainsForbiddenChar}` 错误枚举复制三份，字符策略已开始漂移
（name/tag 接受 `-`/`_`，text 接受 printable ASCII + newline；错误消息 `{ch:?}` vs `'{ch}'` 不一致）。
应统一成一个 `validate.rs` + `CharPolicy`。

### A2. 五个 `XListPage` 形状字段名不一致

- `tag.rs:18`、`role.rs:20`、`user.rs:37`：`{ x_list, has_next, total }`
- `comment.rs:17`：`{ comments, has_next }`（无 total）
- `version.rs:18`：`{ version_list, page, has_next }`（无 total）
- `search.rs:42`：`{ article_list, page, has_next }`

应改泛型 `ListPage<T>{ items, has_next, total }`。

### A3. `{id, name}` 三个同形结构体

`TagRef`（tag.rs:7）、`TagNameView`（tag.rs:25）、`RoleNameView`（role.rs:27）完全相同。
应合并为 `NamedRef`。

### A4. `TagView == TagListItem` 逐字段相同

`tag.rs:4` 与 `tag.rs:11` 字段完全一致；`ArticleListItem` 是 `ArticleView` 的严格子集。
"View vs ListItem" 拆分被机械套用，即使形状相同也无收益。仅当确实不同才保留独立类型（如 `RoleListItem`）。

### A5. 死代码 / 未执行的不变式

`code/common/src/request.rs:44 has_consistent_email_pow_pair` 无人调用且无 allow，后端从不执行它。

### A6. `SearchRange` 三张平行表

`search.rs:4` 的 serde rename、`search.rs:28` 的 `label()`、`logic/search.rs:250` 的反解析，三处手维护可漂移。
应单一 `Display`/`FromStr` 源 + 唯一的 `label()`。

---

## 三、边界 / 契约问题

### 1. repository 层在做 logic 的活

- `enrich_articles`（`repository/article.rs:319`，130 行 God 函数）拼 `ArticleView`
- `articles_of_user`（`article.rs:511`）直接构造 `ArticleListItem`
- `pick_recycler_target`（`repository/transfer.rs:222`）实现负载均衡策略
- `assemble`（`authorization.rs:286`）做 Cedar/policy 组装（属 infrastructure）

### 2. logic 层泄漏

- `logic/download.rs:59` 拼 HTTP 路由字符串
- `logic/version.rs:45` 直接调 `tokio::fs`（create_dir_all / remove_file）
- `interface/article.rs` 跨模块 import 通用 multipart 助手 `read_text_field/stream_pdf_field/map_multipart_error`

### 3. 错误映射三种风格并存

- 命名 mapper：`map_create_comment_error`、`map_transfer_error`、`name_update_error`
- 内联 match：`article.rs:167`
- `LogicError::internal(format!(...))`：散布在 role.rs/tag.rs/version.rs
- `LogicError` 无 `From<RepoError>`，每处手写 `map_err`
- `database_error`（`logic/error.rs:71`）把所有 DB 失败折叠成 500，破坏 404 语义，调用方被迫用预检查绕过

### 4. 三套 HTTP / envelope 栈（前端）

`request/http.rs`、`infrastructure/limits.rs:75`、`request/download.rs` 各自维护 timeout/status/envelope，
还绕过已有 `envelope::is_success`。

### 5. 路由契约不统一（`interface/router.rs`）

- 占位符 `{id}` vs `{role_id}`
- 单复/复混杂：`/comment/{id}/read` vs `/comments/{id}/replies/create`
- 路径反解构：tuple vs struct vs String
- 怪兽常量名 `ROUTE_ARTICLE_ID_VERSION_VERSION_ID_CONTENT_READ`
- 硬编码 body-limit 公式 `*5 + 64KiB`

---

## 四、前端专项

### 1. URL 同步有 4 种实现

`search.rs:126`、`comment.rs:59`、`delete.rs:45`、`draft.rs:31`，而 `draft.rs` 已有现成抽象没被复用。

### 2. `CommentSection` God 组件

`comment.rs:33`：6 模式 + 5 handler + 2 effect + 内联 auth。
根因是 `router.rs:88` 用 `/*comment_path` catch-all 吞掉所有子路由，`comment/url.rs` 再用字符串手术反解析。

### 3. `search.rs` 334 行单组件

`RANGE_KEYS`/`RANGE_LABELS`（search.rs:19）平行数组靠下标对齐。

### 4. `request/*` 层是约 30 个薄包装

body 构建不统一（typed struct / `json!` / `&()` / Option 字符串），返回类型不统一（`delete_tag` 返回 `()` 其他返回 view）。

### 5. 页面状态约定不统一

- 在途 flag 命名：`working`/`submitting`/`posting`/`sending`/`confirming`
- 回调风格：`Callback::new` vs 裸 `move |ev|`
- 参数读取：`query_signal`+`Memo` vs `use_query_map`
- 错误处理：仅 toast / 仅内联 / 两者 / 都无

---

## 五、死代码 / 脚手架残留

- 后端：`#[allow(dead_code)] read_tag_articles`（repository/tag.rs:254）、`read_tag_detail`（logic/tag.rs:147）；
  **6 个空目录** `code/back/src/logic/{authenticate,challenge,email,error,pow,session}/`（与同名 `.rs` 模块混淆）
- 前端：`validation.rs:41 validate_tags` 死代码；`infrastructure/pow.rs`、`js.rs` 纯透传
- `interface/principal.rs:19-20` 两个 `#[allow]` 编译脚手架
- `infrastructure/logging.rs:22 OffsetTime.offset` 恒为 UTC 的死字段；`logging.rs:70` 硬编码 `"log/back"` 回退
- `email.rs:56` 双锁 + cooldown-before-success bug（失败也启动冷却窗口）+ 每封重建 SMTP transport

---

## 六、当下最紧迫：未提交的脏改动

`git status` 显示一个未提交批次，混合了两件无关事：
- 真正的 bug 修复：`request/tag.rs:20` `["tag","list"]` → `["tag","read"]`（对齐后端 `ROUTE_TAG_READ`）
- 新 feature：`code/front/src/page/user/list.rs`（UserList 页，`read_users(1,200)` 硬编码、忽略 `has_next`）

问题：
1. 该 feature 无 exec doc 覆盖（在途 `S7d4` 明确把 Frontend 划为 out-of-scope）
2. `read_users(u64,u64)` vs `read_tags(Option<u64>,Option<u64>)` 签名不一致
3. `request/user.rs:8` 用 `pub use` 再导出，而 `request/role.rs` 不用（两种风格并存）
4. `router.rs:54` 的 `/user` 列表路由放在参数父路由之前，`/tag`、`/role` 放在之后（无约定）
5. 硬编码 `1,200`、忽略 `has_next`，未用已有 `page/pagination.rs` 助手
6. 该 feature 叠在 common 层复制型 `UserListPage` 上，又添一个副本

---

## 建议优先级（"code judo" 优先）

1. **common 层先统一**：`validate.rs` + `ListPage<T>` + `NamedRef`，一个改动消掉 A1–A4 一整类。
2. **repository 建图抽象**：`outgoing_edges/incoming_edges/edge_count` + 消除 `_sync/_in_txn` 双份（宏或 trait），消掉约 300 行复制。
3. **一个 `highest_version_number`**，修掉 `delete.rs` 的字符串 max bug。
4. **logic 统一错误映射**（`From<RepoError> for LogicError`），并让 `database_error` 按 kind 映射而非一律 500。
5. **前端 `use_remote<T>` + `use_session_status` + 单一 URL 同步助手**，删掉 4 份 URL 同步和 3 套 HTTP 栈。
6. **立即清理**：删 6 个空目录、`#[allow(dead_code)]` 死代码、未使用字段；把未提交批次拆成"bug fix"和"feature"并补齐 exec doc。
