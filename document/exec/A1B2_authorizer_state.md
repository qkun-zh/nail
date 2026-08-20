# Exec Doc: Authorizer State

## 1. Requirement

设计一个 `Authorizer` struct 作为 Axum State，封装 Cedar 授权引擎，提供统一的 `.authorize(user_id, action, resource)` 方法。

**验收标准**：
- `Authorizer` 放入 `AppState`
- 所有授权调用使用 `state.authorizer.authorize()`
- 577 个测试全部通过
- `cargo clippy` 零警告
- `cargo fmt` 通过

## 2. Scope

**In-scope**：
- 创建 `Authorizer` struct
- 将 Cedar 集成到 `Authorizer`
- 放入 `AppState`
- 更新 `logic/authorize.rs`
- 更新所有 handler
- 清理旧的 `infrastructure/cedar.rs`

**Out-of-scope**：
- 缓存策略
- ABAC/ReBAC 扩展
- 中间件集成

## 3. Design Decisions

- `Authorizer` 封装 `cedar_policy::Authorizer`、`PolicySet`、`DbHandle`
- 统一接口：`authorize(user_id, action, resource) -> Result<(), AuthorizationError>`
- `AuthorizationError` 枚举：`Denied`、`ResourceNotFound`、`Internal(String)`
- `Resource` 枚举保持不变，用于传递资源类型和 ID
- 每次授权都从数据库重新组装实体（无缓存）

## 4. Slice Breakdown

### Slice 1: 创建 Authorizer struct
- **Goal**: 创建 `infrastructure/authorizer.rs`，实现 `Authorizer` struct
- **Files**: `code/back/src/infrastructure/authorizer.rs`
- **Red**: 无（新文件）
- **Green**: `cargo build` 通过
- **Exit test**: `cargo test`

### Slice 2: 放入 AppState
- **Goal**: 将 `Authorizer` 添加到 `AppState`
- **Files**: `code/back/src/infrastructure/state.rs`, `code/back/src/infrastructure/server.rs`
- **Red**: 无
- **Green**: `cargo build` 通过
- **Exit test**: `cargo test`

### Slice 3: 更新 logic/authorize.rs
- **Goal**: 使用 `state.authorizer.authorize()` 替换旧的授权调用
- **Files**: `code/back/src/logic/authorize.rs`
- **Red**: 无
- **Green**: `cargo test` 通过
- **Exit test**: `cargo test`

### Slice 4: 更新所有 handler
- **Goal**: 更新 interface 层，使用新的授权方式
- **Files**: `code/back/src/interface/*.rs`
- **Red**: 无
- **Green**: `cargo test` 通过
- **Exit test**: `cargo test`

### Slice 5: 清理旧代码
- **Goal**: 移除 `infrastructure/cedar.rs` 中的旧函数
- **Files**: `code/back/src/infrastructure/cedar.rs`
- **Red**: 无
- **Green**: `cargo build` 通过
- **Exit test**: `cargo test`

## 5. Open Unknowns

- 无

## 6. Verification Plan

| Dimension | Check |
|-----------|-------|
| Correctness | 所有 577 个测试通过 |
| Behavior change | 授权行为不变，只是重构 |
| Time complexity | O(n) 组装实体，n 为角色+权限数量 |
| Space complexity | 每次请求分配实体 Vec |
| Performance | 无缓存，每次从数据库读取 |

## 7. Risks

- 测试可能需要更新（如果接口变化）
- 回滚：git revert

## 8. Constraints

- 不修改 Cedar 策略
- 不修改数据库 schema
- 保持所有现有测试通过

## 9. Questions

- 无

## Change log

- 2026-08-20: 初始版本
- 2026-08-20: Slice 1 开始 - 重构 infrastructure/cedar.rs
