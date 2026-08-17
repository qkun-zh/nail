# 002 — Fix code quality defects

## 1. Requirement

修复10个代码质量缺陷。

| # | R | Acceptance | Status |
|---|---|---|---|
| 1 | 超大 Leptos 组件拆分 | search.rs 拆分到 <=256 行子模块 | done |
| 2 | 前端 config panic! → 非 panic 处理 | config.rs 无 panic!/unwrap/expect | done |
| 3 | 删除 dead_code 压制 | notify.rs 无 #[allow(dead_code)] | done |
| 4 | 删除未使用参数 | document.rs 无 `let _ =` | done |
| 5 | ctx → comment_view_context | 无缩写变量名 | done |
| 6 | 新增 GitHub Actions CI | 存在 ci.yml | done |
| 7+8 | 补测试并提高覆盖率 | 覆盖率 ≥88%，无 panic | done |
| 9 | 删除 too_many_lines 压制 | 各 Cargo.toml 无 allow | done |
| 10 | 调研 clippy allow 修复方案 | 记录调研结果 | done |

## 2. Scope

In scope: 全部 10 项缺陷。
Out of scope: 暂无。

## 3. Design decisions

- #2: tracing 在前端不可用，使用 `web_sys::console::error_1` 替代
- #3: 删除 Info/Warning 变体和对应函数，同时清理 kind_class 中的匹配
- #4: 删除 effective_ranges 参数，更新所有调用方
- #5: 仅改名，不改逻辑
- #10: 调研后记录，不立即修改

## 4. Slice breakdown

| Slice | Goal | Files | Exit | Status |
|---|---|---|---|---|
| 1 | #2: 修复 config panic! | config.rs | cargo check (front) | done |
| 2 | #3: 删除 dead code | notify.rs | cargo check (front) | done |
| 3 | #4: 删除未使用参数 | document.rs + 调用方 | cargo test (back) | done |
| 4 | #5: 重命名 ctx | comment.rs | cargo check (front) | done |
| 5 | #10: 调研 clippy allow | 记录到 handoff | N/A | done |
| 6 | #1: 拆分超大组件 | search.rs → 子模块 + CSS | cargo test/clippy | done |
| 7 | #9: 删 too_many_lines | 各 Cargo.toml | cargo clippy | done |
| 8 | #6: CI | .github/workflows/ci.yml | workflow 运行 | done |
| 9 | #7+8: 补测试+覆盖率 | test/unit/** | 88%+ 覆盖率 | done |
| 10 | 收尾: front 超限组件 | search/form,results; comment/state | clippy 零警告 | done |
| 11 | 收尾: clippy 阈值对齐宪法 | 各 crate clippy.toml (256) | clippy 零警告 | done |
| 12 | 收尾: CI 无 workspace 修复 | ci.yml 按 crate 分步 | workflow 可运行 | done |

## 5. Open unknowns

- #2: 前端是否可用 web_sys？需 probe 验证 — resolved（可用）
- 收尾: unused_async_trait_impl 在新 clippy 已重命名 — resolved（移除 allow）

## 6. Verification plan

| Dimension | Status |
|---|---|
| Correctness | verified — 纯重构/删除 |
| Behavior change | verified — #2 仅改错误处理方式 |
| Time/Space/Perf | N/A |
| Coverage | back 89.10%（4143/4650 行），back 测试 499、common 109 全绿 |

## 7. Risks

- #2: 如果 web_sys 不可用，fallback 到 eprintln! — resolved（web_sys 可用）
- #3: 如果有代码引用 Info/Warning，编译会报错 → 编译验证 — resolved
- 覆盖率到不了 100%：剩余为基础设施（config/logging/server/smtp/email/cedar/main/seed_demo）需真实服务或 mock，以及内部 DB 故障/竞态分支，非用户输入路径 — 接受

## 8. Constraints

- 不修改公共 API 行为
- 每个 slice 独立可编译

## 9. Questions

无。
