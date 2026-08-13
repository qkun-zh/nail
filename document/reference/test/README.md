# 测试目录（test/）

> 全项目测试代码的唯一家园。测试按**所测 crate** 分族，各自通过 `#[path]`
> 挂载到对应 crate：
>
> ```
> test/
> ├── unit/          普通测试：cargo test 必跑，零外部依赖
> │   ├── back/      nail_back crate 的单元测试（挂 code/back/src/main.rs）
> │   └── common/    common crate 的单元测试（挂 code/common/src/*.rs 各模块）
> └── end_to_end/    端到端：cargo test --features end_to_end（真实 TCP/HTTP + SMTP sink + 浏览器）
> ```
>
> `test/unit/back` 与 `test/unit/common` 是两个独立 crate 的测试树（common 是
> 独立 crate、被 back 依赖），不能合并：各自只能访问所挂 crate 的内部，且
> common 的 50 用例不编译 back 的重依赖、0.02s 独立跑完。

生产源码里不再出现任何 `#[cfg(test)]` 实体，只留 `code/back/src/main.rs` 里
`#[path]` 挂回声明（2 个挂载点：`unit_tests` / `end_to_end_tests`（feature
门禁，内含 http 族与 browser 族）。stress 族已于 2026-08-13 删除（其场景
已被迁移后的单元并发用例 + e2e http 覆盖）。

---

## 普通测试（unit/）

**跑法**：

```bash
cargo test --manifest-path code/back/Cargo.toml           # back 全部
cargo test --manifest-path code/back/Cargo.toml http_     # 只跑 HTTP 层
cargo test --manifest-path code/common/Cargo.toml         # common
```

**架构**：
- `unit/back/harness.rs` = 模块注册表（唯一挂载点，`#[path]` 显式相对路径）；
- `unit/back/context.rs` = 共享测试支撑 `TestCtx`（内存态 + SMTP
  指向关闭端口 + 独立 PDF 临时目录 + wire/PoW/种子/断言助手；限流已迁
  pingap 反代，后端不再放开限流）；
- 分层：`http_*`（打真实 Router 的 wire 层）/ `logic_*`（流程函数层）/
  `repository_*`（DB/cache 数据层）/ `configuration_`、`pdf_`、`log_`（纯函数层）；
- 探针（2026-08 agdb+seekstorm 迁移后）：`repository/authorization.rs`（Cedar
  授权判定 9 用例）+ `repository/search.rs`（seekstorm 搜索语义 8 用例）作为
  迁移语义基线；SurrealDB 探针（surreal_reference/fulltext_probe/
  search_redesign_probe/unique_concurrency_probe/atomic/migrate）随引擎删除。

**新增测试**：≤512 行/文件；在 `harness.rs` 登记一行；命名遵守 §命名规范。

**用例数**：back = 374（含 repository_authorization 9 + repository_search 8 等
迁移语义基线）+ common 57。2026-08-12 实测全量默认并行可跑（seekstorm Arc 环
泄漏已修，见知识库 A1；此前每用例泄漏 ~55MB 触发 OOM-kill）。

---

## 端到端测试（feature = "end_to_end"）

浏览器场景走**真实链路**（用户拍板，禁止进程内 ServeDir 模拟）：
真实 `nail_back` 进程（CONF_DIR 指测试临时配置）+ 真实 pingap 代理（8080
反代 /api→3000、/ →dist）+ 真实 `trunk build --release` 产物，chromiumoxide
驱动 Linux Chromium 打 `http://localhost:8080`。SMTP 收件端是**唯一**模拟点
（进程内 sink，不可能真发 qq 邮箱）；其余全真实。

**前置**：
1. 前端构建产物：`cd code/front && trunk build --release`（缺 `index.html`
   浏览器用例直接 panic）；
2. 后端二进制：`cd code/back && cargo build`（浏览器用例 spawn 它）；
3. Linux Chromium：`/usr/bin/chromium`（Debian 官方包），可用
   `NAIL_E2E_CHROME` 覆盖路径。**不要用 Windows Edge**：WSL2 里 Windows
   进程的 CDP 端口从 Linux 侧不可达（网络隔离），浏览器测试必须用 Linux
   chromium（Chromium 132+ 移除旧 headless，驱动已用 `--headless=new`；
   WSL/容器 root 下必须关沙箱，驱动已带 `--no-sandbox`）。

**跑法**（浏览器用例共端口 3000/8080，**必须 `--test-threads=1`**）：

```bash
cargo test --features end_to_end --manifest-path code/back/Cargo.toml -- --test-threads=1
cargo test --features end_to_end --manifest-path code/back/Cargo.toml end_to_end_tests::browser -- --test-threads=1   # 只跑浏览器族
```

- `end_to_end::http`：**自包含**——真实 TCP + 真实 HTTP 栈 + 进程内本地 SMTP
  sink（替代真实邮箱/IMAP），无需任何外部凭据；
- `end_to_end::browser`：chromiumoxide 驱动真实 WASM 前端，断言 DOM/路由/
  localStorage（登录链走真实邮件 → token 兑换，文章 seed 走真实 API）。

---

## 命名规范（铁律：零缩写、自解释）★★★

所有测试命名（目录、模块、文件、用例、测试支撑类型/函数）都必须遵守。

### 通用原则
1. 每个词必须是**完整英文单词**。
2. **允许保留**的技术缩写（白名单，单独出现即自解释）：`http` `url` `uuid`
   `pdf` `smtp` `imap` `json` `html` `mime` `tls` `ip` `ascii` `utf` `xml`
   `ast`。`cas` 展开为 `compare_and_swap`；`id` 展开为 `identifier`。数字
   （`400`/`401`/`429`/`v7` 等）可保留，用下划线分隔：`_with_401`、`uuid_v7`。
3. **禁止**一切项目内部缩写/简写（黑名单见下），出现即展开。
4. 用例名公式：**`<前置条件>_<动作>_<期望结果>`**。
5. 状态码断言统一 `_with_<码>` 后缀（HTTP 层）；语义断言用完整词。

### 黑名单展开表
| 禁用（缩写/简写） | 必须展开为 |
| :--- | :--- |
| `e2e` | `end_to_end` |
| `api`（测试层前缀） | `http` |
| `repo` | `repository` |
| `conf` | `configuration` |
| `auth`（简写） | `authentication`（`authenticate` 是完整单词，保留） |
| `pow` | `proof_of_work` |
| `ttl` | `time_to_live` |
| `ctx` | `context` |
| `util`（模块/文件名） | `context`（测试支撑）或 `utilities`（其它） |
| `tmp` | `temporary` |
| `db` | `database` |
| `meta` | `metadata` |
| `req` | `request` |
| `resp` | `response` |
| `unauth` | `unauthorized` |
| `cas` | `compare_and_swap` |
| `fts` | `full_text_search` |
| `ids` | `identifiers` |
| `ok` 之外短助词（`do`/`go`） | 完整语义词 |
| `new_tests` | `unit_tests` |

### 三大族模块前缀
| 族 | 目录 | 模块前缀 | 示例 |
| :--- | :--- | :--- | :--- |
| 普通 | `test/unit/back/` | `http_`/`logic_`/`repository_`/`configuration_`/`pdf_`/`log_` | `http_authenticate`、`repository_atomic` |
| 端到端 | `test/end_to_end/{http,browser}/` | 顶层 `end_to_end_http`/`end_to_end_browser`，文件内子模块无前缀 | `authentication_mail_chain`、`login_workflow` |

---

## 环境变量表
| 变量 | 作用 |
| :--- | :--- |
| `NAIL_E2E_CHROME` | 覆盖浏览器可执行路径（默认 `/usr/bin/chromium`；勿用 Windows Edge，WSL2 下 CDP 不可达） |
| `E2E_REAL_MAIL` | 真实邮箱冒烟（可选，默认不做） |
| `CONF_DIR` | 覆盖 conf 目录（config 加载用） |

## 常见故障
- **端口占用**：e2e 每个用例绑定 `127.0.0.1:0`（随机端口），通常不冲突；
  若仍报占用，杀残留后端进程。
- **找不到 Chrome**：浏览器场景前置缺浏览器二进制；设 `NAIL_E2E_CHROME` 指向
  可用 Chromium（Linux 侧路径）。
- **dist 未构建**：浏览器场景要求 `code/front/dist` 存在，先 `trunk build --release`。
- **浏览器用例卡在等待**：先 `curl http://localhost:8080/api/meta/limits` 确认
  代理链路活着（后端/进程残留会占 3000/8080）；残留进程 `pkill -x nail_back;
  pkill -f pingap-linux-gnu-x86-full` 后重跑。
- **WSL 需 `--no-sandbox`**：浏览器启动参数已带；root 环境下不要去掉。
