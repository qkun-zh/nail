# agdb 替代 SurrealDB 可行性调研 + 迁移知识库

- 调研对象：`agdb` 0.13.2（crates.io，2026-07-28 发布）+ 源码 `probe/agdb/`（浅克隆 agnesoft/agdb）
- 实测方式：`cargo add agdb` 独立探针 crate `probe/agdb_probe/`（纯 Rust，从零编译 9.82s）
- 探针覆盖：repo 层 9 项核心语义，9/9 全绿（`cargo run --manifest-path probe/agdb_probe/Cargo.toml`）
- 日期：2026-08

---

## 0. 结论速览

1. **agdb 是嵌入式纯 Rust 图数据库，是四个候选（samyama / CypherLite / Kuzu / agdb）中唯一实测覆盖 nail repo 层全部核心语义的**。
2. 代码量/编译量级差一个数量级：surrealdb-core 3.2.4 源码 15MB / 1163 个 `.rs`（另有 collections/protocol/strand/types 外围 + rocksdb/surrealkv/rustls 传递依赖）；agdb 0.13.2 源码 1.6MB / 98 个 `.rs`。
3. **写路径天然消除 nail 的整套并发补丁**：`exec_mut(&mut self)` 独占写 → `repo/atomic.rs`（retry_write / guard_record_exists / 冲突识别）整块可删，约 600–800 行 SurrealDB 补丁代码随迁移消失。
4. 三个行为变化需在迁移时落地：DB 级 `DEFINE EVENT` 约束转应用层、并发用 `Arc<RwLock<Db>>` 包装、FTS 交 SeekStorm（见 `seekstorm_report.md`，替代原 Meilisearch 计划）。
5. 无字符串查询方言：查询全部走类型化 `QueryBuilder`，编译期检查，错误信息可读（`[Db:NotFound] ... (at db.rs:547)`），对 AI 编程友好。

---

## 1. 候选对比（为什么是 agdb）

| 候选 | 否决/通过理由 |
|---|---|
| samyama | 不在 crates.io（需 git/path 依赖）；`CREATE CONSTRAINT ... IS UNIQUE` 语法可解析但**执行器未实现**；SDK `query()` 无参数绑定（CREATE 子句 `$param` 不替换）；MVCC 事务存在但未接入 SDK 路径，`abort_transaction` 注释明说 in-place 写入不真回滚；node id 为自增 u64 无显式字符串主键；持久化是后台写穿非查询级同步。2026-01 创建，37 open issues。 |
| CypherLite | 2 stars、4 个月无 commit、许可证 NOASSERTION（非标准）。比 samyama 更不成熟。 |
| Kuzu | 用户明确排除（C++ 内核、Rust 绑定薄封装）。 |
| **agdb** | crates.io 发布（0.13.2 两周前仍发版）、Apache-2.0、持续发布 3 年 44 个版本、纯 Rust 无原生依赖、事务/WAL/回滚实测正确。 |

---

## 2. 核心 API 知识（迁移必读）

### 2.1 库形态

```rust
use agdb::{Db, DbMemory, QueryBuilder};

let mut db = DbMemory::new("name")?;        // 内存库
let mut db = Db::new("db.agdb")?;           // 文件库（memory-mapped，重开即恢复）
```

- `db.exec(&self, query)`：只读查询（并发安全，`&self`）。
- `db.exec_mut(&mut self, query)`：**独占写** + 单条查询即一个事务（提交/回滚内建）。
- `db.transaction_mut(|txn| -> Result<(), E> { ... })`：多语句原子事务；闭包返回 `Ok` → commit，`Err` → **整体回滚**（undo stack + WAL，进程崩溃也能恢复一致状态）。`E` 必须实现 `From<DbError>`。

### 2.2 数据模型

- **节点**：`DbId`（i64，正数为节点 id），可挂任意 `(key, value)` 键值对（key/value 都是 `DbValue`，支持 String/i64/u64/f64/向量等）。
- **边**：`DbId`（负数为边 id），`DbElement { id, from, to, values }`——`from`/`to` 即边端点节点 id，**无需额外查询**。
- **alias**：字符串名，可当 id 用（`QueryId::from("alias")` / `DbId`）。建议 `user:{uuid}` 这种带表前缀的命名约定。

> ⚠️ **alias 不是唯一约束**：重复插入 alias 是"迁移"语义（把别名从旧节点静默拿走，见 `db.rs:606 insert_alias`），不报错。唯一性必须靠索引查重（独占写保证原子）。

### 2.3 写入

```rust
// 插入节点（带 alias 当字符串 id）
db.exec_mut(QueryBuilder::insert().nodes()
    .aliases(["article:abc"])
    .values([[(K_TYPE, T_ARTICLE).into(), (K_TITLE, "t").into()]])
    .query())?;

// 更新已存在节点（ids 非空 = 更新语义）
db.exec_mut(QueryBuilder::insert().nodes()
    .ids([user_id])
    .values([[(K_EMAIL_HASH, new_hash).into()]])
    .query())?;

// 建边
db.exec_mut(QueryBuilder::insert().edges().from(user_id).to([article_id]).query())?;

// 删元素（边/节点）
db.exec_mut(QueryBuilder::remove().ids([edge_id]).query())?;

// 建索引（对 key 建立查找索引；索引可事后建，自动回填存量）
db.exec_mut(QueryBuilder::insert().index("email_address_hash").query())?;
```

### 2.4 读取

```rust
// 按 id/alias 查
let res = db.exec(QueryBuilder::select().ids(["user:abc"]).query())?;

// 按索引精确查（唯一性检查、find_or_create 用）
let res = db.exec(QueryBuilder::select()
    .values([key_value(K_TITLE)])
    .search().index(K_TITLE).value("Duplicate Title")
    .query())?;

// 图搜索 + 条件 + 排序 + 分页（链式顺序固定：origin → order_by → offset → limit → where_）
let res = db.exec(QueryBuilder::search()
    .elements()                                    // 全库遍历（或 .from(id) 起点遍历）
    .order_by([DbKeyOrder::Desc(K_TYPE.into())])
    .offset(1)
    .limit(2)
    .where_()
    .key(K_TYPE).value(T_VERSION)                  // 默认 Equal；也可 Comparison::GreaterThan 等
    .query())?;
```

结果 `QueryResult { elements: Vec<DbElement> }`；**元素不含属性值**。

> ⚠️ **查询返回的 elements 恒不带 values（实测，权限探针 P2）**：search/select 只给结构（id/from/to），`values` 字段恒为空。
> 读属性必须显式 `select().values([...]).ids([id])`（可一次多 key）：
>
> ```rust
> // 读单个/多个属性（属性缺失时：单 key 报 Db:NotFound，多 key 只要一个缺就整组报错）
> let res = db.exec(QueryBuilder::select()
>     .values([key_value(K_TYPE), key_value(K_SCOPE_ID)])
>     .ids([edge_id]).query())?;
> // 可选属性（如 scope_id）必须单 key 分读，NotFound 当 None
> let scope = db.exec(QueryBuilder::select()
>     .values([key_value(K_SCOPE_ID)]).ids([edge_id]).query())
>     .map(|r| r.elements.first().and_then(|el| el.values.first()))
>     .unwrap_or(None);
> ```

### 2.5 条件与遍历语义（实测重点）

| 条件 | 语义 |
|---|---|
| `where_().key(k).value(v)` | key == v（默认 Equal；支持 GreaterThan/LessThan/NotEqual/Contains/StartsWith/EndsWith） |
| `where_().edge()` / `.node()` | 当前元素是边 / 是节点 |
| `where_().distance(CountComparison::Equal(n))` | **距 origin 的距离**：边 = 1，边对端节点 = 2 |
| `where_().neighbor()` | 快捷 = distance Equal(2)（即 origin 的直接邻居节点） |
| `where_().edge_count(1)` 等 | 出入边总数/出/入边数比较 |
| `where_().beyond().key(k).value(v)` | **剪枝遍历**：只沿满足 key==v 的边/节点继续，不影响元素选择（权限探针 P4 实证：可限定"只沿 user_hold_role 边走"） |
| `search().to(id)` | **反向遍历**：沿入边（to←from）反查（权限探针 P5 实证：反查 owner） |
| `search().elements()` + `edge_count_to(Equal(0))` | 孤儿清理：无入边的节点（权限探针 P8 实证：无人持有的角色） |

条件组合用 `.and()` / `.or()` 连接：`where_().key(a).value(x).and().key(b).value(y)`。

> ⚠️ **BFS 默认不限制深度**：`search().from(x).where_().edge()` 会返回从 x 可达的**所有层**边。只取一层必须显式 `.distance(Equal(1))`（探针 T8 因此误删过 article→version 边）。
> ⚠️ **不存在的 alias 执行 select 报 `[Db:NotFound]`**，不是返回空。存在性检查用搜索/索引查重。
> ⚠️ **`select().values()` 读不存在的 key 报 `[Query:NotFound]`**（权限探针 P7 实证），不是返回空——可选属性（scope_id）单 key 分读。
> ⚠️ `where_()` 在链式末尾，`order_by/offset/limit` 必须在它之前。

### 2.6 无 schema / 无触发器

- 无 `DEFINE TABLE/FIELD`：键值对自由挂，字段校验由应用层（现有 `from_value` 反序列化层可沿用）。
- 无 `DEFINE EVENT` / 触发器：原 `article_min_one_tag/version` 的 DB 级约束转应用层检查。
- 无 FTS：全文检索交 SeekStorm（嵌入式库，替代原 Meilisearch 计划，见 `seekstorm_report.md`）。

---

## 3. 实测验证（探针 9/9 全绿）

| 探针 | 验证内容 | 结果 |
|---|---|---|
| T2 | find_or_create_user：索引查重 + 独占写原子 + 幂等 | PASS |
| T3 | create_article：2 节点 + 3 边单事务原子 | PASS |
| T4 | 事务中途失败 → 整体回滚零残留 | PASS |
| T5 | title 唯一（应用层索引查重） | PASS |
| T6 | update_user_email CAS 读-改-写 | PASS |
| T7 | 版本分页（order_by + offset/limit）+ 总数 | PASS |
| T8 | 注销转移：删边 + 重建（distance 1 限定） | PASS |
| T9 | 文件库持久化：重开库数据保留 | PASS |

---

## 3.1 权限模型验证（M1 `probe/permission_probe/`，10/10 全绿）

- 探针：`cargo run --manifest-path probe/permission_probe/Cargo.toml`（agdb 0.13.2，与 agdb_probe 同版本同风格）
- 验证对象：`permission_system_design.md` §2 图模型（RBAC 图化：role 节点 + `user_hold_role`/`role_grant_permission` 边 + 边属性 scope）在 agdb 上完整落地

| 探针 | 验证内容 | 结果 |
|---|---|---|
| P1 | role/permission 节点 get-or-create（专用索引查重 + 幂等 + 类型隔离） | PASS |
| P2 | 边属性读写（`user_hold_role` 带 scope_type/scope_id 示例属性）+ 读属性 | PASS |
| P3 | 同图多边类型共存，按 type 属性过滤遍历 | PASS |
| P4 | 多跳遍历 user→role→permission（beyond 剪枝 + 两跳分查） | PASS |
| P5 | 反向遍历 `to(article)` 反查 owner | PASS |
| P6 | entity store 组装（角色 + 边属性 scope 示例 + 权限列表） | PASS |
| P7 | 作用域过滤（global / tag:tech 示例属性） | PASS |
| P8 | 孤儿角色清理 `edge_count_to == 0` | PASS |
| P9 | 事务原子：建角色+挂权限+挂用户，中途失败整体回滚 | PASS |
| P10 | 文件库持久化：重开库角色/权限/边仍在 | PASS |

> 注：P2/P6/P7 的 scope_type/scope_id 是**边属性读写能力的示例数据**（权限设计终版已不用边属性存 scope——角色作用域改为 `role_apply_tag` 直连 tag 节点，见 `cedar_report.md` §3.1.1）；这些探针验证的"agdb 边属性能力"本身仍有效。

**对 nail_new 的实现约束（agdb 心智差异，全部实证）**：

1. **边类型区分靠属性，不靠表名**：agdb 无边表概念，`user_hold_role`/`role_grant_permission`/`user_own_article` 是边上的 `type` 值（`DbKeyValue`），遍历/过滤用 `key(K_TYPE).value(边名)`；
2. **entity store 组装用"两跳分查"**（`neighbor()` + type 过滤，Rust 层组装），`beyond().key().value()` 剪枝可行但语义绕，不作为主路径；
3. **唯一性索引 key 专用化**：`role_name`/`permission_name` 而非共用 `name`，避免跨类型索引污染（与 agdb_probe 惯例一致）；
4. **边属性读取**：属性必须 `select().values()` 显式读（见 §2.4 ⚠️），可选属性（scope_id）单 key 分读、NotFound 当 None；
5. **孤儿角色清理**：`search().elements()` + `edge_count_to(Equal(0))`，与 tag 孤儿清理同模式。

**联调（`probe/integration_probe/`，8/8 全绿，详见 `cedar_report.md` §3.1）**：本探针的图模型 + `cedar_probe` 的判定模型已串联验证——边 → Cedar parents 链、字段 → attrs、角色作用域（`role_apply_tag` 边，与 `article_apply_tag` 平行）→ Tag 实体集合 + `containsAny` 交集、判定翻转（插边即生效）。组装实证要点：反向遍历含起点（distance 0），须按 type 过滤（I4 踩坑）；集合交集用 `.containsAny()`/`.contains()` 非 `in`（I9 踩坑）。

---

## 4. 迁移映射表（nail repo 层 → agdb）

| nail 现状（SurrealDB） | agdb 实现 | 备注 |
|---|---|---|
| `db.begin()/commit()` + guard 写 + retry_write | `transaction_mut`（Err 自动回滚） | 并发补丁全删 |
| `is_write_conflict` / `is_unique_index_conflict` 错误识别 | 无写冲突；唯一性走先查后插 | `atomic.rs` 整删 |
| UNIQUE 索引（title/content_hash/email_hash/name/tag_name） | `insert().index(key)` + 应用层查重 | 独占写保证原子 |
| `meta::id(id)` / `type::record(...)` 记录 id 体操（92 处） | alias 或 DbId | 全消失 |
| `RETURN BEFORE/AFTER` + 缺失记录 `[Null]` 判断（5 处） | select 报 NotFound / 搜索返回空 | 判断逻辑简化 |
| `DEFINE EVENT` 文章 ≥1 tag/version | 应用层检查 | 行为变化 ① |
| FULLTEXT 索引 ×6 + analyzer | SeekStorm（单索引 BM25F） | 行为变化 ②，见 `seekstorm_report.md` |
| `DEFINE FIELD TYPE string ASSERT` | 无 schema，应用层校验 | `from_value` 沿用 |
| `precheck_unique_index` 存量查重 fail-loud | 删除 | 不再有 UNIQUE 索引定义 |
| `INFO FOR DB` 表存在探测（migrate） | 搜索/索引查重 | 方言替换 |
| 地址归一化 mem/path（db.rs） | `DbMemory::new` / `Db::new` 二选一 | 简化 |
| 并发多任务共享 `Surreal<Db>` | `Arc<RwLock<Db>>`（写独占 `&mut`，与官方 agdb_server 同模式） | 行为变化 ③ |

**可删除补丁清单**（约 600–800 行）：`repo/atomic.rs` 整模块；`schema.rs` 的 DEFINE 全套 + precheck + 事件；92 处 `meta::id`/`type::record`；3 处 `strip_record_id`；5 处 `RETURN BEFORE/AFTER` 判断；comment.rs 的 `array::map` 防扫表；types.rs 的 SurrealValue 中转；Cargo.toml 的 surrealdb 重依赖。

**不可删（业务本身）**：`migrate.rs` 的 content_hash/name 回填逻辑；错误枚举映射（TitleAlreadyExists 等，检测方式变但枚举不变，api 层零改动）。

---

## 5. 风险与待办

1. **规模**：agdb 单作者项目、社区小；nail 为嵌入式单进程小数据量，恰是其目标场景。若未来需要多机/RPC，agdb 没有——但 nail 现在也没用 SurrealDB 的这些能力。**权限系统已由 `permission_probe`（§3.1）验证可行**：RBAC 图化（role 节点 + 边属性 scope + 多跳遍历）在 agdb 上 10/10 落地。
2. **写吞吐**：独占写串行化。axum 层 `Arc<RwLock<Db>>` 后写并发 = 锁串行，个人项目规模无压力。
3. **性能**：memory-mapped 存储 + BFS 遍历；大数据量（百万级节点）性能未实测，nail 规模无需担心。
4. **迁移验证**：`repo/` 层重写后需跑既有测试基座（`test/unit/back`）；`end_to_end` feature 的测试依赖真实 DB 行为，一并回归。
5. **探针保留**：`probe/agdb_probe/` 作为语义基线；`probe/agdb/`（源码克隆）与 `probe/llvm/`（agdb 不需要，属 samyama 遗留）可清理。
