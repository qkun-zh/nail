# 发信业务重构实施细则

- 对象：`back/src/other/email`（发送层）、`back/src/logic/{authenticate,email,user}.rs`（三个发信点）、`back/src/repo/token`（token 缓存清理）
- 依据：2026-08 缓存重构第二轮讨论结论（职责边界、速率独立、不依赖 moka）
- 状态：**已执行**（commit `c71d74c` 落地主重构；后续裁定：authenticate 复用窗口与 `auth_send_locks` 一并删除，见 §2.3/§3.3/§4）
- 前置：token 缓存重构（commit `b4acaad`）已落地，本任务在其之上

---

## 1. 目标

发信业务从 token 缓存解耦为独立 `EmailService`，速率限制脱离 moka，token 缓存回归纯粹（只存 token 有效性）。

## 2. 最终设计

### 2.1 职责边界（三层）

| 层 | 职责 | 状态归属 |
|---|---|---|
| EmailService（发信业务，独立模块） | 投递 `to+subject+body`（lettre）+ 按 `to` 宽松速率限制 | `Mutex<HashMap<String, Instant>>`，标准库，**零 moka 依赖** |
| 验证流程（authenticate / email_update / deregister） | token 铸造/记账/消费/防重放；调 EmailService 发信 | 各业务域 + `repo::token` |
| repo::token | 主缓存（TTL+Expiry、LRU、listener 投影，已落地）+ 反向索引 | moka |

### 2.2 EmailService 形态

- 接口：`send_email(to, subject, body) -> Result`，内部：速率检查（锁内）→ SMTP 投递（锁外）
- 速率：按收件邮箱（to）限速，窗口 = conf `email_cooldown_seconds`；**宽松**——不追求精确（窗口边界误差、偶发并发、过期条目不清理、重启清零均可容忍）
- 不生成 token、不管验证、不依赖 moka；SMTP 发送逻辑（spawn_blocking + 墙钟超时）沿用现状 `email_core`

### 2.3 验证流程调用点

- 三个发信函数（auth 邮件 / email-update 双信 / deregister 确认信）删掉各自冷却样板，改调 `EmailService::send_email`
- authenticate 无冷却复用（2026-08 执行时裁定删除）：窗口内重复请求由 EmailService 按 to 限速拒绝 → `SendEmailError::RateLimited` → 4xx「已发过，请查收」；无验证域查询、无 per-邮箱锁
- 发信顺序统一为**先发信、成功才记账**，删除现状"先记账后发信 + 失败回滚"分支

## 3. 实施步骤

1. **EmailService**（`other/email`）：新建 `EmailService`（含速率状态 + smtp 配置 + cooldown）；`send_email` 封装现有 `email_core::send_email`；`EmailService::new` 从 conf 构造
2. **AppState**（`other/app_state.rs`）：加 `email: EmailService` 字段；`other/server.rs` 与 `test/unit/back/context_fixtures.rs` 的构造点同步加字段
3. **logic/authenticate.rs**：删 `reuse_window` / 复用查询 / per-邮箱锁 `auth_send_locks`（并发双发由限速锁串行化）；发信改调 `email_service.send_email`，`RateLimited` → 400、`Smtp` → 500
4. **logic/email.rs**：删 `send_cooldown` 检查；铸造 token 对 → 两次 `send_email`（旧/新邮箱，各自按 to 限速）→ 记账；删回滚分支
5. **logic/user.rs**：删 `send_cooldown` 检查与 `has_recent_token`；铸造 token → `send_email` → 记账；删回滚分支
6. **repo::token 清理**：删除 entry 的 `created_at` 字段（authenticate/deregister/email_update 三个 entry，`repo/types.rs`）、`has_recent_token`、`delete_deregister_token`、`find_unconsumed_by_email_address_hash`（整函数删除，反向索引仅服务批量作废）；反向成员 `expires_at` 保留（排序/死候选）
7. **测试**：更新冷却/复用相关断言（见 §5）；新增速率表行为测试（`logic_email_service`）与 authenticate 限速 400 用例
8. **文档同步**：`knowledge_base.md` A3 节（速率归属 EmailService、entry 删 created_at）；`token_cache_refactor_design.md` 状态与 §3.3/§6 涉及 created_at 的表述

## 4. 必须知道的行为变化与约束

- 速率语义变严格：窗口内一律拒发（现状 token 被消费后窗口内会重发）——方向在安全侧，接受
- email-update 双发：第一封成功、第二封失败 → 不记账（token 对不落缓存，第一封链接失效，用户可重试；注意速率窗口副作用：重试可能被拒至窗口结束）
- email-update 冷却维度从"按用户"变"按收件邮箱"：旧邮箱 = 账号当前邮箱，重复触发时旧邮箱 key 相同，防 spam 效果不变
- `auth_send_locks`（per-邮箱锁）已删除：并发首请求双发由 EmailService 限速锁（Mutex 原子）天然串行化，不再需要验证域锁
- 速率表 key 用收件邮箱原文（内存态，不落日志/不持久化）

## 5. 测试要求

- 现有 `test/unit/back/repository/token.rs` 中 created_at / has_recent_token / find_unconsumed 相关断言同步删除
- 新增：EmailService 速率窗口内拒绝、窗口外放行、按邮箱隔离；authenticate 限速 400「已发过，请查收」
- 全量 `cargo test --bin nail_back` 通过（当前 375 例，改动后应保持全绿）

## 6. 验证与归档

- `cargo check` 零警告；`cargo clippy --bin nail_back` 无新增警告
- 按 `how_to_complete_task.md` 2.3 归档：一次 commit（英文 message），遗留工作区改动（search 相关 5 文件）不并入、保持原样
