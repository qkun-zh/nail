# 搜索功能最终设计报告

- 关联调研：`surrealdb_search_report.md`（SurrealDB 3.2.4 搜索功能源码 + 实测）
- 可行性探针：`test/unit/back/repository/search_redesign_probe.rs`（9 探针全绿）
- 日期：2026-08
- 状态：**已确认稿**（范围 6 项：标题/摘要/作者/评论/版本说明/标签；结果=文章+命中片段；相关度不可选、恒挂末尾；整套替换现有 `/article/search`）

---

## 0. 设计总纲（原则）

1. **输入框只装纯文本**：主搜索框内零保留字、零转义、零内联语法——`输入 → 恒等`，歧义在结构上不存在。
2. **复杂度 = 用户需学会的"语义决定"数，不是控件数**：输入框（自明）≠ 负担；模式开关（需解码）＝ 负担。故只保留极少数自明语义。
3. **聪明交给 SurrealDB**：相关度 = 6 个 FTS 来源各自 BM25（`search::score`）→ Rust 层 RRF 跨来源融合；展示 = `search::highlight`（需索引声明 `HIGHLIGHTS`）。
4. 界面收敛为：**搜索框 + 范围圈 + from/to + 排序池/当前顺序 + 分页结果**。

---

## 1. 界面结构

```
[🔍 任意文本…   ]                          ← 主搜索框

范围（复选，默认 6 项全选）:
 (✓) 标题   (✓) 摘要   (✓) 作者   (✓) 评论
 (✓) 版本说明 (✓) 标签

筛选: 时间  from [  ]  to [  ]            ← from ≤ to

排序:
  池:       [时间] [标题字母序] [作者名字母序]
  当前顺序:  [时间↓ ×] [标题↑ ×] [作者↑ ×]

── 结果（分页）──
────────────────────────────────────────────────
 深入 Rust 的内存安全 · 作者:李雷 · 2026-08    ← 第一行:标题·作者·时间,不高亮,整行可点跳详情
 [评论] 编译期内存安全保证…                   ← 命中字段标签,一个一行
 [版本说明] 修复内存泄漏                       ← snippet 整段显示
────────────────────────────────────────────────
```

---

## 2. 搜索框

- 自由文本；**多词 = 隐含 AND**（不打即算，输入更多 = 更精确）。
- 默认搜**全部 6 项**；无保留字、无 `#tag` 特例语法、无转义前缀。
- 文本框内容对 6 个范围统一使用；在 FTS 来源上做词项 AND。

---

## 3. 范围（6 个复选圈）

- 默认全选；点圈取消 = 该字段不参与搜索/融合。
- **跨范围合并语义 = OR**：命中**任一**勾选字段的文章即上榜；卡片里只列有命中的字段。
- 字段（共 6 项，平级）：
  **标题、摘要、作者、评论、版本说明、标签**。
- 各字段来源与匹配方式：

| 范围 | 字段 | 表 | 匹配方式 | 索引 |
|---|---|---|---|---|
| 标题 | `title` | article | FULLTEXT 词项 AND | article_title_fts_idx ✓ |
| 摘要 | `summary` | article | FULLTEXT 词项 AND | article_summary_fts_idx ✓ |
| 作者 | `name` | user | FULLTEXT 词项 AND | user_name_fts_idx ✓ |
| 评论 | `content` | comment | FULLTEXT 词项 AND | comment_content_fts_idx（新增） |
| 版本说明 | `note` | version | FULLTEXT 词项 AND | version_note_fts_idx（新增） |
| 标签 | `name` | tag | FULLTEXT 词项 AND | tag_name_fts_idx（新增） |

- 圈选决定参与搜索的来源子集；6 个来源均为 FTS，全部参与 RRF 融合（产生相关度分）。
- **版本说明只取每篇最新版本**参与命中（§9-4）。

---

## 4. 时间（from / to）

- 窗口过滤；**from ≤ to**（方向无关，只取区间）；空即不限。
- 格式：ISO 日期，粒度 年 → 月 → 日 → 时:分:秒；缺省时区 = **+08:00（东八区）**。
- **基准 = 最新版本创建时间**（§9-2 已定；不再提供切换选项）。
- 后端映射：from → `uuidv7_min`，to → `uuidv7_max`，闭合区间。

---

## 5. 排序（有序多键，每键独立方向）

- **池**（可点字段，相关度不在池内）：**时间（最新）/ 标题字母序 / 作者名字母序**。
- **相关度不可选、恒挂当前顺序末尾**作最低优先级 tie-break（用户不可见、不可改）。
- **当前顺序**：按点击顺序构成优先级（先点 = 主排序）。
  - 每个 item 左端：切换 `↓`（倒序）/ `↑`（正序），各键独立。
  - 每个 item 右端：`×` 移除该键。
- **空当前顺序** = 用户未选键 → 实际顺序仅 `[相关度]`，即默认按相关度排序。
- 方向默认：时间默认 `↓`；标题、作者默认 `↑`（A→Z）。
- **执行方式（一刀切）**：
  - 当前顺序**不含相关度**（即用户选了键）→ 整条 DB `ORDER BY <键1> <方向1>, <键2> <方向2>, …` + 后半段反向扫描优化；相关度仅在"主排序值相同"的同值组内作 Rust tie-break。
  - 当前顺序**只含相关度**（用户没选键）→ Rust 全量取回命中文章并按相关度排序。

| 用户操作 | 实际顺序 | 执行方式 |
|---|---|---|
| 没选键 | `[相关度]` | Rust 全量排序（默认主排序=相关度） |
| 选 `[时间↓]` | `[时间↓, 相关度]` | DB `ORDER BY time DESC` + 相关度 tie-break |
| 选 `[标题↑]` | `[标题↑, 相关度]` | DB `ORDER BY title ASC` + 相关度 tie-break |
| 选 `[作者↑]` | `[作者↑, 相关度]` | DB `ORDER BY author ASC` + 相关度 tie-break |

---

## 6. 结果展示（卡片）

1. **结果行 = 文章**。**第一行**：`标题 · 作者 · 时间`，**不高亮**（纯元信息），**点击整行**跳转文章详情页（`/public/article/{id}`）。
2. **命中信息**：按范围圈选中的字段，**只列出有命中的字段**（无命中省略），逐行 snippet，行首带字段标签 `[标题]/[摘要]/[作者]/[评论]/[版本说明]/[标签]`。
3. **字段标签一个一行**（每个有命中的字段单独一行，行首标签 + 该字段 snippet）。
4. **snippet 整段显示、不限长**；行内所有命中词 `<mark>` 高亮（不只亮一个）。
5. 命中字段全部展开（总量 ≤ 6，可控）。
6. 命中来源（跨表字段如评论/版本/标签/作者）在结果里以片段形式**进结果集展示**——用户能看到"这篇文章命中了哪些地方"，但命中来源行不是独立搜索结果，只挂在所属文章下。

**字段长度上限**（snippet 展示的字段内容上限）：

| 字段 | 上限 |
|---|---|
| 标题 | 128 |
| 摘要 | 1024 |
| 评论 | 512 |
| 版本说明 | 256 |

数据来源：逐字段 `search::highlight(open, close, idx)` 渲染；字段标签对应 FTS 索引。跨表来源经各表 FTS 命中后映射回所属文章，再取该字段命中片段。

---

## 7. 分页

- 沿用现有实现：offset 分页 + 页数封顶。
- 每页 `search_page_size`（默认 8）；响应 `article_list/total/total_pages/page/has_more/has_prev/truncated`。
- 翻页**保持**当前过滤与排序（URL query 同步）。
- **总命中数 = SQL count**：`SELECT count() FROM ... WHERE <勾选字段 OR 过滤>`，与排序方式无关（RRF 只管顺序，不管 count）。P0/P1 均走现有 `count_articles`。
- 排序方式：
  - 非相关度排序：DB `ORDER BY` + 后半段反向扫描优化。
  - 相关度排序（用户未选键）：Rust 全量排序后按分页窗口切片（天然放弃反向扫描优化）。

---

## 8. 后端（SurrealDB 支撑）

| 层 | 用 SurrealDB |
|---|---|
| 相关度 | 6 个 FTS 来源各自 `BM25(1.2,0.75)`（`search::score`）；跨来源用 RRF 在 Rust 层融合 |
| 高亮 | `search::highlight(open,close,idx)`；**前置 = FTS 索引声明 `HIGHLIGHTS`** |
| 排序键 | 时间（id=uuidv7 时间序）/ 标题（COLLATE NUMERIC）/ 作者（COLLATE）走 DB；相关度（RRF 综合分）走 Rust |
| 命中字段 | 按参与的各 FTS 来源分别判命中，跨表来源映射回所属文章 |
| 时间 | `uuidv7_min/max`，基准 = 最新版本时间 |
| 总命中数 | SQL `count()` + 勾选字段 OR 过滤 |

**相关度链路：分数 → 排名 → RRF**

1. **DB 层每来源算 BM25 分数**：每个勾选来源在 DB `@AND@` 命中后取 `search::score(0)`，**来源内部按分降序**排，取 top-k。
2. **Rust 层 RRF 融合（只看排名，不看分数绝对值）**：把每来源 top-k 映射回文章，某篇文章综合分 = 它在各来源里的名次位次加权和：
   `rrf_score(article) = Σ_sources 1 / (k + rank_in_source)`（`rank` 为该文章在某来源的名次，`k` 为 RRF 常数，通常 60）。
3. **BM25 分数仅用于生成来源内排名**；跨来源融合只用名次。原因：BM25 的 IDF 在"命中词出现于全部文档"（n=N）时被钳到 0 → 该来源全 0 分；RRF 只看名次，来源内全 0 分仍有 1/2/3… 名次可参与融合（探针已实证此坑）。

> ⚠️ 正确关键字是 `HIGHLIGHTS`，不是 `HIGHLIGHTER`（后者解析报错）。声明后额外落词项 positions、索引体积增大——项目未上线、数据可随时删，接受该开销，无需迁移/回填。

---

## 8.2 可编码契约（API）

搜索接口为 **`GET /article/search`**（恢复 GET，整套替换现有实现），请求经 query 参数，响应经 JSON 体。`code/common/src/request.rs` / `response.rs` 承载结构体。

### 请求参数

| 参数 | 类型 | 说明 |
|---|---|---|
| `q` | string | 搜索文本，**空串 = 搜全部文章**（无文本过滤，仅剩范围/时间/排序）。多词 = AND。超 `max_search_query_chars` 400 |
| `ranges` | string | 参与搜索的范围，逗号分隔，值 ∈ `title,summary,author,comment,note,tag`；**缺省 = 全部 6 项**。空串 = 全部 |
| `sort` | string | 排序池多键，逗号分隔，每个 `键:方向`，键 ∈ `time,title,author`，方向 ∈ `asc,desc`；按出现序 = 优先级。**缺省 = 空（仅相关度）**。相关度恒挂末尾，不可在此指定 |
| `from` | ?date | 时间窗起点（ISO，粒度秒，含边界），基准 = 最新版本创建时间；空 = 不限 |
| `to` | ?date | 时间窗终点，闭合；`from>to` → 400 |
| `limit` | int | 每页大小，缺省 `search_page_size`(8)，钳制 [1, `max_search_page_size`] |
| `page` | int | 页码，1-based，缺省 1 |

示例：`/article/search?q=memory&ranges=title,comment,note&sort=time:desc,title:asc&from=2026-01-01&limit=8&page=1`

### 响应（200 `SearchArticleResponse`）

```json
{
  "ok": true,
  "page": 1,
  "total": 3,
  "total_pages": 1,
  "has_more": false,
  "has_prev": false,
  "truncated": false,
  "article_list": [
    {
      "id": "article-uuid",
      "title": "深入 Rust 的内存安全",
      "author": "李雷",
      "time": "2026-08-01T12:00:00+08:00",
      "hits": [
        { "field": "comment", "label": "评论", "snippet": "编译期内存安全保证" },
        { "field": "note", "label": "版本说明", "snippet": "修复内存泄漏" }
      ]
    }
  ]
}
```

- `article_list[]`：结果行 = 文章；`hits` 只含勾选范围内有命中的字段（≤6），每个 = 一个标签一行；`snippet` 整段显示不限长，前端渲染 `<mark>` 高亮命中词。
- 排序语义：`sort` 缺省（仅相关度）→ 按 RRF 综合分；否则按 `sort` 键序 + 相关度末尾 tie-break。
- 时间排序（`sort=time`）基准与筛选一致 = **最新版本创建时间**。

---

## 9. 已定决策

1. **跨表范围 → 命中判定归一为文章**：tag / 作者 / 评论 / 版本（经版本说明）跨表（article / user / comment / version）。
   - **结果一律是文章**；非文章表只作命中判定来源，不产生独立结果行。
   - 各表按 FTS 命中，经边映射回所属文章（user→`user_to_article`；comment→`comment_to_version`→`article_to_version`；version→`article_to_version`；tag→`article_to_tag`）。
   - 每篇文章的命中片段（哪些字段命中 + snippet）**进结果集展示**，见 §6-6。
   - 相关度：6 个 FTS 来源内部按 `search::score` 排 top-k 排行，Rust 层映射回文章后 RRF 融合成综合分。
2. **时间基准 → 最新版本时间**：搜索时间筛选用**最新版本创建时间**，而非文章创建时间。**时间排序（`sort=time`）基准与此一致**（同为最新版本时间）。
   - 每篇取其最新版本 id（uuidv7 = 时间序）再套 from/to 窗口；与现有 `TimeBasis::Version` 逻辑一致。
3. **版本只经「版本说明」参与搜索**：`version.note`（FTS）为一个范围圈；**版本号不参与搜索**（surrealdb 3.2.4 分词会切碎 semver、无子串/前缀语义，取舍后不搜版本号）。
4. **版本说明只取最新版本**命中：跨表映射回文章时，仅该篇最新版本的 `note` 参与 FTS 判定。
5. **跨范围合并 = OR**：命中任一勾选字段即上榜；卡片只列有命中的字段。
6. **多词 = AND**：输入更多关键词 = 更精确。
7. **排序池 = `{时间（最新）, 标题字母序, 作者名字母序}`**：作者排序需投影 `_author`（COLLATE）；相关度不进池、固定挂末尾，不作为用户可选排序键。
8. **相关度恒挂当前顺序末尾**作自动 tie-break，不可选、不可见。
9. **snippet 整段显示、不限长**；命中字段标签一个一行。
10. **字段长度上限**：标题 128 / 摘要 1024 / 评论 512 / 版本说明 256。
11. **整套替换现有 `/article/search`**：新契约不保留旧过滤参数模型，前端 search 页同步重建。
12. **P0 建索引一步到位带 `HIGHLIGHTS`**：新增 3 张 FTS 索引直接声明 `HIGHLIGHTS`，避免 P2 时重索引/回填；P2 只加展示逻辑。

---

## 10. 实施阶段

- **P0（前端 + 后端基础）**：搜索框 + 6 范围圈 + from/to + 排序池 `{时间, 标题字母序, 作者名字母序}` + 分页 + 卡片（第一行跳转 + 命中字段 snippet 整段 + 标签一行）+ **§8.2 契约落地**。**后端同步改动**：新增 3 张 FTS 索引（comment.content / version.note / tag.name，**一步到位带 HIGHLIGHTS**）、6 来源命中判定与跨表映射、OR 合并、总命中 count、整套替换 `/article/search` API。
- **P1（相关性）**：跨来源 RRF 综合相关度排序（用户未选键时）；同表跨字段搜索。
- **P2（高亮）**：FTS 索引加 `HIGHLIGHTS`，`search::highlight` 出 `<mark>`；跨表字段命中片段按 §9-1 策略落地。
