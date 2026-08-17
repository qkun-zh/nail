# Handoff

## Current state

10 项代码质量缺陷全部修复。测试全绿，back 覆盖率 89.10%。三 crate fmt/clippy 零警告。

## Done

- 缺陷 #1: search.rs 拆分（772→359 行）→ `search/{schema,query,db,comments,versions}.rs`；CSS 提取到 `code/front/search.css`；`search/mod.rs` 改名 `search.rs`（遵守无 mod.rs 宪法）
- 缺陷 #2: config.rs panic! → `web_sys::console::error_1` + `FrontendConfig::default()`
- 缺陷 #3: 删除 Info/Warning 变体及 `notify_info`/`notify_warning`
- 缺陷 #4: 删除 `read_comment_outcome` 的 `effective_ranges` 参数
- 缺陷 #5: `ctx` → `comment_view_context`
- 缺陷 #6: 新增 `.github/workflows/ci.yml`（fmt/clippy/test/audit/build，按 crate 分别运行）
- 缺陷 #9: 删除所有 Cargo.toml 的 `too_many_lines` allow；新增各 crate `clippy.toml`（阈值 256，与宪法一致）
- 缺陷 #10: clippy allow 调研 — `principal.rs` 的 `unused_async_trait_impl` 在新版 clippy 已重命名，allow 移除；`js.rs`（cast_possible_truncation/sign_loss，wasm f64→u64）保留
- 缺陷 #7+8: 覆盖率 87.74% → 89.10%（4143/4650 行）；测试 454 → 499（back），common 109 全绿
- 新测试覆盖的真实用户输入路径：管理员改名撞名、`/session/read?name=true`、文章/版本 multipart 未知字段、评论子列表 HTTP 成功路径、搜索 `from` 非法/空边界
- 收尾修复：front `Search`（357→252 行）拆出 `search/{form,results}.rs`；`CommentSection`（346→180 行）拆出 `comment/state.rs`（CommentSignals + build_load/submit 系列）；`SearchComments` 的 needless_pass_by_value 修复

## Decisions

- `js.rs` 的 clippy allow 保留（有正当理由）
- `tarpaulin-report.html` gitignore，不提交
- 覆盖率到不了 100%，接受（见 Remaining risks）
- 三个 crate 各自独立（无 workspace 根），CI 按 crate 分步运行

## Remaining risks

覆盖率到不了 100%，剩余未覆盖行均为非用户输入路径：
1. 基础设施：`config/logging/server/smtp/email/cedar/main.rs/seed_demo.rs` — 需真实 SMTP 服务、服务器、环境 mock
2. 内部错误分支：DB 写入失败（`.map_err` 路径）、删除/更新竞态、磁盘 I/O 错误、内部 ID 冲突 — 防御性代码，正常输入无法触发

要继续提升需引入 DB 故障 mock 层（改动较大，仅能再加 1-2%），已决定不收。

## Next

- 无待办。10 项缺陷全部完成。

---

# Permission System Overhaul（待办，未开始）

## 决策

1. `Restore` → `Undelete::Soft`
2. 删除 `User::Delete::Transfer`，保留 `Version::Delete::Transfer`
3. `Role::Manage` 拆分为 6 个权限
4. 统一 Virtual
5. User 支持软删除

## 待办阶段

- Phase 1-11：Cedar schema/policy、build.rs、repository、logic、interface、operations、tests、Virtual 修复、full explicit authorization、verification

详见 git 历史中早期 handoff 版本。
