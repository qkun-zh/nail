# Handoff

## 任务组织要求（每次写入/更新 handoff 必须遵守）

1. 任何任务必须以细分的 slice 为单位，呈现三层分级：**slice → stage → task**
   - slice 用阿拉伯数字编号（如 `1.`, `2.`）
   - stage 用大写字母编号（如 `A.`, `B.`）
   - task 用罗马数字编号（如 `I.`, `II.`）
2. task 完成后必须**及时**从 handoff 中清除，防止 handoff 熵爆炸——只保留未完成与进行中的条目。
3. 各任务在 handoff 中必须有清晰的分界（按任务分区、标注任务归属），防止混淆和干扰。
4. 不得修改、删除或干扰不属于自己职责的 task；如需改动他人任务须先获得明确许可。

## Current state

- 任务「修复 10 项代码质量缺陷」：**已完成并清除**（见提交历史 03d1c7c..2707dd8）。back 覆盖率 89.10%，三 crate fmt/clippy 零警告，back 499 测试、front 69 测试全绿。
- 当前唯一待办：**Permission System Overhaul**（见下，归属待认领）。

## Remaining risks（继承自已完成任务，供参考）

覆盖率 89.10% 封顶，剩余未覆盖均为非用户输入路径（需真实 SMTP/服务器/mock，或 DB 故障/竞态防御分支）。

---

# 任务：Permission System Overhaul

**归属**：待认领。**状态**：未开始。此任务专属区域，他人不得修改。

## 决策（已定，勿改）

1. `Restore` → `Undelete::Soft`
2. 删除 `User::Delete::Transfer`，保留 `Version::Delete::Transfer`
3. `Role::Manage` 拆分为 6 个权限（Create/Read/Update/Delete/Grant/Revoke）
4. 统一 Virtual
5. User 支持软删除
6. 权限数 31（原 27）

## Slice 1 — Cedar 授权层

- **Stage A** — schema.cedar
  - Task I. `Article/Version/Comment::Restore` → `Undelete::Soft`
  - Task II. 删 `User::Delete::Transfer`，加 `User::Delete::Soft`、`User::Create`
  - Task III. 删 `Role::Manage`，加 6 个细粒度权限
  - Task IV. 统一 `resource: [Virtual]`
- **Stage B** — policy.cedar
  - Task I. owner bypass 更新（Soft 替换 Transfer）
  - Task II. Policy 4 改 action set 匹配
  - Task III. Policy 5 recycler 限制更新
- **Stage C** — build.rs
  - Task I. 更新 test_only 列表

## Slice 2 — 后端实现层

- **Stage A** — repository
  - Task I. role.rs 权限常量自动生成
  - Task II. delete.rs 新增 `soft_delete_user`
  - Task III. authorization.rs 更新 `Resource::Virtual`
- **Stage B** — logic
  - Task I. 三个 restore 改 undelete_soft
  - Task II. user.rs 移除 transfer、加 soft delete
  - Task III. role.rs 用 6 个细粒度权限
- **Stage C** — interface
  - Task I. router 路由改名
  - Task II. 各 handler 改名 + 权限检查更新

## Slice 3 — 操作与测试

- **Stage A** — operations.rs
  - Task I. ROUTE_*_RESTORE → UNDELETE_SOFT
  - Task II. ROLE_* 权限映射更新
- **Stage B** — tests
  - Task I. 更新旧权限/路由名的测试
  - Task II. 新增 `User::Delete::Soft` 测试
  - Task III. 全量测试通过

## Slice 4 — 授权强化（可选）

- **Stage A** — 修复 Virtual 滥用（User/Role 用具体资源）
- **Stage B** — full explicit authorization（policy 重写 + owner bypass 验证 + admin 角色策略）
- **Stage C** — verification（fmt/clippy/test/trunk build）