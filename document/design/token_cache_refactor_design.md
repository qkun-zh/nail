# 令牌缓存层重构设计报告

- 对象：`back/src/repo/token.rs` 及其子模块（authenticate / session / deregister / download / email_update / challenge）
- 依据：moka 0.12.15 `sync` 模块源码逐行核实（`cache.rs` / `entry_selector.rs` / `builder.rs` / `base_cache.rs` / `policy.rs` / `ops.rs` / `value_initializer.rs` / `segment.rs` / `notification.rs`）；0.12.16 与 0.12.15 仅内部实现差异、公共 API 无变化，按 0.12.15 落地
- 日期：2026-08
- 状态：**已落地**（v2 设计按方案 B 实现：统一有序 `Vec<ReverseMember>`；`SegmentedCache` 分片留待实测 n 或吞吐瓶颈再启用）。**2026-08 邮件重构后续**：`created_at` 从主缓存条目与反向成员中彻底移除（见 §3.3/§6）；authenticate 冷却复用查询（`find_unconsumed_by_email_address_hash`）与 `auth_send_locks` per-邮箱锁删除，冷却/速率/防轰炸统一由 `EmailService` 按收件邮箱承担（独立于 token 缓存）。

---

## 0. 设计总纲（原则）

1. **正确性不变量一字不动**：主缓存 = token 有效性**唯一事实来源**；反向索引只是**候选提示**，所有候选强制回主缓存核验；缓存内统一 `hash::token(明文)`、永不存明文。
2. **能力边界内设计**：moka 不能做的（按 value 属性反查、时间 range、值内子成员过期）不硬造，在它给的四个原语（`and_compute_with` / `eviction_listener` / `Expiry` / `eviction_policy`）之上做最小、最优的承接。
3. **驱逐策略显式化**：一次性 token 存储不是"缓存"，admission 控制语义不适用——主/反向缓存一律显式 `eviction_policy(EvictionPolicy::lru())`，杜绝 TinyLFU 默认策略对满容量新插入条目的 admission 拒绝（见 §2.1・风险面）。
4. **消费原语升级**：consume 从"`remove` + `created_at`/TTL 复验"升级为 `and_compute_with` 单次原子调用——前置 `get` 已过滤过期/失效条目，天然 TTL 感知，不再依赖 `policy().time_to_live()`（见 §4.1）。
5. **反向索引 = 主缓存的投影**：主缓存每消失一条（任何 `RemovalCause`），`eviction_listener` 在源头同步摘除对应候选，不再依赖"写时猜、读时跳"的惰性清理。
6. **集合结构由实测 n 决定**，不预设小 n、也不全局一种：`session_by_user`（可能大 n）与 `authenticate/deregister`（冷却兜底、小 n）分开选型。

---

## 1. 现状（保留不动的不变量）

| 表 | key | value 条目 | TTL | 反向索引 |
|---|---|---|---|---|
| `authenticate` | token_hash | `{ email_address_hash, email_subject }` | token_ttl | `authenticate_by_email_hash` |
| `session` | token_hash | `{ user_id }` | session_ttl | `session_by_user` |
| `email_update` | **user_id** | `{ old_email_address_hash, new_email_address_hash, token_*_hash }` | token_ttl | 无 |
| `deregister` | token_hash | `{ user_id, email_address_hash }` | token_ttl | `deregister_by_user` |
| `download` | token_hash | `{ version_id, user_id }` | download_token_ttl | 无 |
| `challenge` | challenge_id | `()` | challenge_ttl | 无 |

现状无结构性错误；待改两点：**死候选滞留**（主缓存被 TTL/capacity 驱逐时反向集不清）与**最新/窗口查询扫全量**（`HashSet` 无序）。

---

## 2. moka 能力边界（读源码的硬事实 v2）

### 2.1 `sync` 模块提供的

- O(1) `get/insert/remove/invalidate/contains_key`。
- **单 key 原子 compute**：`entry().and_compute_with(f)` → `Op::Put/Remove/Nop`。同 key 经 key-level waiter + key lock 串行化（`value_initializer.rs`、`entry_selector.rs` 文档）。compute 的 `Op::Remove` 走 `invalidate_with_hash`，与 `Cache::remove` 完全同路径，**同步**触发 eviction listener。
- **compute 前置 `get` 是 TTL 感知的**：闭包收到 `Some(entry)` 必为未过期条目（`get` 过滤 per-entry TTL / 裸 TTL / TTI / invalidate 标记，`base_cache.rs` `do_get_with_hash`）——过期未驱逐条目在闭包中呈现为 `None`。**这与 `remove` 不同**：`remove` 不检查 TTL，过期条目仍会被返回（现状复验的存在根源）。
- **`eviction_listener(Arc<K>, V, RemovalCause)`**，`RemovalCause ∈ {Expired, Explicit, Replaced, Size}`。`remove/invalidate` 在**调用线程同步阻塞**通知（挂 listener 时带 key lock 串行化）；timer-wheel 到期驱逐报 `Expired`；`invalidate_all` 的惰性清理最终报 `Explicit`。
- **`Expiry` 逐条过期策略**：`expire_after_create/read/update(key, value, Instant)` → 返回精确到期时刻；与裸 `time_to_live` 并存时取**最早者**。⚠️ `policy().time_to_live()` **只反映裸 TTL 字段、不反映 Expiry 返回值**——Expiry-only 会令该查询返回 `None`。
- **`eviction_policy(EvictionPolicy)`**：TinyLFU（默认）/ LRU。**LRU 分支无条件 admit**（容量满仅走 LRU 驱逐）；TinyLFU 在容量满时对新条目做频次比较，**可能 `Rejected` → 将刚插入条目直接从哈希表移除（`RemovalCause::Size`）**——对一次性 token 即"铸造即失效"，是默认策略下的隐藏风险面。
- **`SegmentedCache`（0.12.15 新类型）**：`.segments(n)` 构建；内部 `next_power_of_two(n)` 个独立完整 `Cache`，key 按 hash 高位固定路由，总容量均分（每段 `ceil(cap/n)`），TTL/Expiry/listener/policy 全套配置复制到每段；写通道（64 槽）、deques、timer wheel、policy 锁全部按段分片。
- `iter()`（O(N) 全表扫、弱保证）、`invalidate_entries_if`（O(N) 谓词扫，需 `support_invalidation_closures` + `PredicateId`）、`run_pending_tasks`（显式触发维护任务，测试/确定性清理用）。

### 2.2 `sync` 模块没有的（硬边界）

- ❌ 无二级索引：key 是不透明标量，无法按 value 属性（user_id/email）反查。
- ❌ 无前缀 / 时间 range 扫描：`iter` 只能 O(N) 全扫。
- ❌ TTL/TTI/Expiry 均整条 entry 级：无法"值内单个成员过期"。

> 推论："属性 → 成员集合"维度 moka 给不出，只能存在于一个 moka 槽的 value 里手管集合语义。此边界对任何 KV 缓存成立（Redis 用 `SET`/`ZSET` 补，moka 没有内置同伴，故建 companion cache）。

---

## 3. 目标结构

### 3.1 主缓存（moka Cache<token_hash, Entry>，六表同构）

- **TTL：裸 `time_to_live` 与 `Expiry` 同值并存**。`expire_after`（`expire_after_create` 返回 `Some(ttl)`）保持每条到期显式化、留 per-entry TTL 扩展点；裸 TTL 保住 `policy().time_to_live()` 返回 `Some`（防御未来复用复验模式）并让 `notify_invalidate`/`notify_upsert` 的 cause 判定（查裸 TTL 字段）正确报 `Expired`。两者取最早 = 同值，行为等价。
- **挂 `eviction_listener`**：主缓存条目因任何 `RemovalCause` 消失 → 从对应反向索引 `remove` 该 token。listener 内只做快操作（反向索引删除），因 remove/invalidate 的 listener 是调用线程同步阻塞。
- **显式 `eviction_policy(EvictionPolicy::lru())`**：LRU 无条件 admit，新铸造 token 必进缓存；容量满走 LRU 驱逐（`RemovalCause::Size`，listener 照常清反向索引）。
- **`SegmentedCache`（可选，建议开启）**：`.segments(n)`（n = 4/8/16，取 power-of-two；token_hash 随机散列、天然均衡）。写通道/deques/timer wheel/policy 锁按段分片，缓解高并发铸造/消费下的写通道满阻塞（写通道满时写操作会阻塞调用线程）。容量语义不变：`max_capacity` 是总容量，conf `token_cache_capacity` 无需改。

> 效果：反向索引成为主缓存的**投影**。主缓存（唯一事实）每消失一条，listener 在源头同步摘除——**死候选在源头被清**。consume / delete 走 `remove`/`invalidate`/compute 都会触发 listener，与调用点显式 `reverse_remove` 重复但**幂等**（`reverse_remove` 对不存在成员是 Nop），安全。

### 3.2 反向索引（moka Cache<attribute, Collection<Member>>）

- 所有写走 `entry().and_compute_with`（原子、按属性键串行）。
- 集合按 n 选型（见 3.3）。
- 清理三保险：主缓存 `eviction_listener`（源头同步）＋值内惰性 `expires_at` prune ＋集合空则 `Op::Remove`。
- **反向缓存不设 TTL**（逐成员管到期，整条 TTL 会对冲）；用 `eviction_policy(lru())` + `max_capacity` 做属性数量的资源兜底——LRU 驱逐属性 key 只损失候选提示（无 listener，主缓存成员不受影响），读路径回主缓存核验保证正确性；空属性即 `Op::Remove`，属性 key 数被 live 用户/邮箱数自然约束。
- **`delete_*_by_*` 顺序：先整条 `invalidate` 反向 key（O(1) 使全部候选立即失效、立刻阻断读路径），再逐 key `invalidate` 主缓存**——listener 里的逐成员 `reverse_remove` 全部变为 Nop，省去逐成员锁与 compute 开销；正确性不变（幂等）。

### 3.3 成员集合选型

成员字段：`ReverseMember { token: token_hash, expires_at: Instant }`（`expires_at = 主缓存条目创建时刻 + 主缓存 TTL`）。**不存 `created_at`**（主缓存与反向成员都不存——cooldown/速率已迁至 `EmailService`，见 §6）：反向索引只当候选提示，有效性/新鲜度回主缓存核验，时间戳只从 `expires_at` 一处取。同一缓存内 TTL 恒定 ⇒ `expires_at` 与创建时刻排序等价，**单一时间序同时服务**取最新、新鲜度窗口、过期修剪。

| n 规模 | 结构 | 操作复杂度 | 适用 |
|---|---|---|---|
| 小 | `Vec<ReverseMember>`（`expires_at` 升序） | 取最新=倒序命中即返 O(k)；remove=retain O(n)；add 尾部 push 后 sort | authenticate / deregister（冷却兜底） |
| 大 | `{ by_token: HashMap<token, expires_at>; by_time: BTreeMap<expires_at, HashSet<token>> }` | 全 O(log n)；窗口 range、过期从头连弹 | session_by_user（每设备/每刷新一条会话，n 可能很大） |

> 枚举（批量作废 `delete_*_by_*`）在任何结构下都是 Ω(n)（铁律，无法避免）。

---

## 4. 消费与读写路径

### 4.1 consume：`and_compute_with` 单次原子、TTL 感知

替代现状"`remove` + `created_at`/TTL 复验"（`expired_after_remove` 删除）：

```rust
let result = cache.entry(key).and_compute_with(|e| match e {
    Some(_) => Op::Remove,   // 前置 get 已保证未过期 → CompResult::Removed(entry) 拿值
    None => Op::Nop,         // 不存在 / 已过期 / 已被并发消费 → StillNone ≡ 已耗尽
});
```

- 单赢家：key-level waiter + key lock 串行（与 `remove` 同强度）。
- 过期条目在闭包中不可见 → 无需 created_at 复验、不查询 `policy()`（**不再有 Expiry-only 时复验静默失效的坑**）。
- `Op::Remove` 同步通知 listener → 反向索引投影自动摘除，显式 `reverse_remove` 可删可留（幂等）。
- **`consume_email_update_token_if_matches` 同样迁移**：闭包内比较 token 对哈希，匹配 → `Op::Remove`（拿行），不匹配 → `Op::Nop`（当前行原样保留 = 并发 send 覆盖的新行，语义与"remove 后插回"等价且更简单）。
- **`consume_challenge` 迁移**：get→remove 两次操作合并为一次 compute，消除 TOCTOU 窗口。

### 4.2 读路径（现状）

- **批量作废**（三个 `delete_*_by_*`）：先整条 `invalidate` 反向 key，再遍历成员逐个 `invalidate` 主缓存，返回值 = 候选数（反向集合取到的成员数）。authenticate 反向索引仅此用途。
- **取最新未消费**（authenticate）：2026-08 邮件重构后续已删除——速率/防轰炸全部由 `EmailService` 按收件邮箱限速承担，验证域不再有冷却复用查询（`find_unconsumed_by_email_address_hash` 整个删除）。

---

## 5. 正确性论证

| 关注点 | 保证 |
|---|---|
| 有效性 | 主缓存唯一事实；反向仅提示；候选强制回核；consume 前置 get 过滤过期（TTL 感知） |
| 并发 | 写走 `and_compute_with` 按属性键串行；consume 单次原子 compute（单赢家） |
| 过期 | 主缓存裸 TTL + `Expiry` 同值（显式到期）；listener cause 判定两路径均正确 |
| 铸造必达 | `eviction_policy(lru())` 无条件 admit，杜绝 TinyLFU 对新 token 的 admission 拒绝 |
| 死候选 | `eviction_listener` 在源头投影删除，零后台任务、零定时器 |
| 一致性 | 反向为最终一致的投影，短暂不一致无害（文档已声明）；`delete_by` 先失效反向再清主，幂等 |
| 不存明文 | 主/反向统一 `hash::token(明文)` |

---

## 6. 变更面

- `repo/token.rs`：`ReverseMember`；`reverse_add`/`reverse_remove` 签名；三个反向字段类型；`build_cache` 加 `eviction_policy(lru())`、`time_to_live` + `Expiry`、`eviction_listener`；consume 基元 `consume_with`（`and_compute_with` 封装）替换 `expired_after_remove`。
- 主缓存可选切 `SegmentedCache`：`build_cache` 增 `segments(n)` 参数（n 由 conf 或按核数决定，默认 1 = 现状单 Cache）。
- `authenticate.rs` / `deregister.rs` / `session.rs`：create（带 `expires_at`）；find / has_recent / delete_by 的取值循环；consume 迁移。
- **2026-08 邮件重构后续**：主缓存条目 `created_at` 移除（过期驱逐交给 moka TTL）；`authenticate` 的 `find_unconsumed_by_email_address_hash`（冷却复用查询）与 `deregister` 的 `has_recent_token` / `delete_deregister_token` 删除；`email_update` 的冷却样板与 `auth_send_locks` per-邮箱锁（含 conf `auth_send_lock_ttl_seconds` / `auth_send_lock_capacity`）删除——冷却/速率/并发双发防护统一由 `EmailService` 按收件邮箱承担（`other/email`，零 moka，`SendEmailError::RateLimited` 映射 4xx）。
- `email_update.rs` / `challenge.rs`：consume 迁移到 `and_compute_with`（无反向索引，listener 不动）。
- `download.rs`：consume 迁移（无反向索引，零其他改动）。

---

## 7. 不做的事（明确排除）

- **Bloom filter**：无法枚举，失效于批量作废。
- **落 DB（agdb）表**：agdb 全局写锁会成为高变更 token 的热点；持久化破坏 TTL/单次使用语义。
- **`invalidate_entries_if` 全表扫**：O(N) 且需 feature flag，只适合低频兜底，不用作热路径。
- **`run_pending_tasks` 后台线程**：moka 本无专用线程，维护任务由用户线程惰性触发；不新增定时器。
- **为优雅而优雅**：`email_update`/`download`/`challenge` 无"按属性批量/时间查询"需求，保持无反向索引；`and_try_compute_with`/`and_upsert_with` 等 compute 家族其他成员无适用场景。
- **`Expiry::expire_after_read` 滑动过期（TTI）**：一次性 token read 即 remove，TTI 无意义；session 若将来要"活跃会话永续"，是行为变更，另行设计。

---

## 8. 扩容边界

本设计对单实例正确。若将来多实例共享 token 层：整体平移 Redis——主缓存 `SET`、反向 `ZSET`（score=expiry）、TTL、原子消费全部原生，跨进程一致。此前的内存投影/有序结构即"逼真预演"，迁移代价平滑。`SegmentedCache` 的 hash 路由与 Redis 分片思路同构，实例化时只需把段路由换成实例路由。
