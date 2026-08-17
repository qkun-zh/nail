# 001 — Split search.rs

## 1. Requirement

R1: 将 `code/back/src/repository/search.rs` 从 772 行拆分到 ≤512 行。
遵循 README 禁止 `mod.rs` 的规则，使用 `search.rs` + `search/` 目录模式。
纯重构，不改变任何外部行为。

Acceptance criteria:
- search.rs ≤ 512 行
- search/schema.rs ≤ 512 行
- search/query.rs ≤ 512 行
- 全部 563 个测试通过
- clippy 零警告
- fmt 通过

## 2. Scope

In scope:
- 提取字段常量和 schema 配置到 search/schema.rs
- 提取查询构建辅助函数到 search/query.rs
- 更新 search.rs 的模块声明和导入

Out of scope:
- 修改 SearchIndex 的公共 API
- 修改任何业务逻辑
- 修改 document.rs（已独立）
- unwrap_or 调整（已确认无需修改）

## 3. Design decisions

- **schema.rs 职责**：定义"索引什么" — 12个 FIELD_* 常量 + schema_fields() + index_meta()
- **query.rs 职责**：定义"怎么搜索" — range_field_name() + request_field_names() + effective_ranges()
- **search.rs 保留**：SearchIndex 核心 + DB 辅助函数 + read/write_schema_version
- **可见性**：schema 常量使用 `pub(crate)` 以便 query.rs 和 search.rs 访问
- **mod.rs 禁止**：遵循 README，使用 search.rs + search/ 目录模式

## 4. Slice breakdown

### Slice 1: schema 模块
- Goal: 提取字段常量和 schema 配置
- Files: search.rs, search/schema.rs (新建)
- Red: 编译失败（模块未找到）
- Green: 全部测试通过
- Exit: `cargo test`

### Slice 2: query 模块
- Goal: 提取查询构建辅助函数
- Files: search.rs, search/query.rs (新建)
- Red: 编译失败（模块未找到）
- Green: 全部测试通过
- Exit: `cargo test`

### Slice 3: Final gate
- Goal: 验证代码质量
- Files: 无
- Exit: `cargo clippy -D warnings && cargo fmt --check && cargo test`

## 5. Open unknowns

无。所有依赖行为已从源码确认。

## 6. Verification plan

| Dimension | Status |
|---|---|
| Correctness | verified — 纯重构，测试覆盖 |
| Behavior change | verified — 无外部 API 变更 |
| Time complexity | N/A — 无算法变更 |
| Space complexity | N/A — 无内存变更 |
| Performance | N/A — 无性能变更 |

## 7. Risks

- **模块可见性错误**：导入路径错误导致编译失败 → 逐 slice 验证
- **循环依赖**：schema/query 之间无依赖，无风险
- **行数超限**：每个目标文件已验证 ≤512 行

## 8. Constraints

- 不使用 mod.rs
- 不修改公共 API
- 不添加注释（README §Comments）
- 不使用 unwrap/expect

## 9. Questions

无。
