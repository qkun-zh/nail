# 002 — Fix code quality defects

## 1. Requirement

修复10个代码质量缺陷。

| # | R | Acceptance |
|---|---|---|
| 2 | 前端 config panic! → 非 panic 处理 | config.rs 无 panic!/unwrap/expect |
| 3 | 删除 dead_code 压制 | notify.rs 无 #[allow(dead_code)] |
| 4 | 删除未使用参数 | document.rs 无 `let _ =` |
| 5 | ctx → comment_view_context | 无缩写变量名 |
| 10 | 调研 clippy allow 修复方案 | 记录调研结果 |

## 2. Scope

In scope: #2, #3, #4, #5, #10
Out of scope: #1(CSS), #6(CI), #7+8(tests), #9(too_many_lines)

## 3. Design decisions

- #2: tracing 在前端不可用，使用 `web_sys::console::error_1` 替代
- #3: 删除 Info/Warning 变体和对应函数，同时清理 kind_class 中的匹配
- #4: 删除 effective_ranges 参数，更新所有调用方
- #5: 仅改名，不改逻辑
- #10: 调研后记录，不立即修改

## 4. Slice breakdown

| Slice | Goal | Files | Exit |
|---|---|---|---|
| 1 | #2: 修复 config panic! | config.rs | cargo check (front) |
| 2 | #3: 删除 dead code | notify.rs | cargo check (front) |
| 3 | #4: 删除未使用参数 | document.rs + 调用方 | cargo test (back) |
| 4 | #5: 重命名 ctx | comment.rs | cargo check (front) |
| 5 | #10: 调研 clippy allow | 记录到 handoff | N/A |

## 5. Open unknowns

- #2: 前端是否可用 web_sys？需 probe 验证

## 6. Verification plan

| Dimension | Status |
|---|---|
| Correctness | verified — 纯重构/删除 |
| Behavior change | verified — #2 仅改错误处理方式 |
| Time/Space/Perf | N/A |

## 7. Risks

- #2: 如果 web_sys 不可用，fallback 到 eprintln!
- #3: 如果有代码引用 Info/Warning，编译会报错 → 编译验证

## 8. Constraints

- 不修改公共 API 行为
- 每个 slice 独立可编译

## 9. Questions

无。
