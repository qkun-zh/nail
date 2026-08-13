# SeekStorm 替代 SurrealDB 全文检索 可行性调研 + 迁移知识库

- 调研对象：`seekstorm` 3.3.4（crates.io，2026-08-08 发布，Apache-2.0）+ 源码 `probe/seekstorm/`（浅克隆 SeekStorm/SeekStorm，2026-08-12 拉取 main）
- 实测方式：`cargo add seekstorm` 独立探针 crate `probe/seekstorm_probe/`（纯 Rust，`default-features = false` 免 pdfium/zh/vb；从零编译 ~1m20s，target 1.7GiB）
- 探针覆盖：nail 搜索层 12 项核心语义，12/12 全绿（`cargo run --manifest-path probe/seekstorm_probe/Cargo.toml`）
- 日期：2026-08

---

## 0. 结论速览

1. **seekstorm 是嵌入式纯 Rust 全文检索引擎（词法 BM25F + 向量双核），是 nail 搜索层的匹配替代**，且形态比原计划 Meilisearch 更贴合（Meilisearch 是外置服务；seekstorm 是 in-process 库，与 agdb 同为嵌入式）。
2. **搜索设计 §8 的 SurrealDB FTS 支撑整体坍缩为单索引**：6 来源 BM25 top-k + Rust 层 RRF + `search::highlight` + `HIGHLIGHTS` 声明，全部被 seekstorm 的「单索引多字段 BM25F + field_filter 圈选 + highlighter + ResultSort + facet 过滤」替代，相关度/高亮/排序/分页/时间窗/计数原生内建。
3. 代码量/依赖量级与 SurrealDB 差一个数量级：surrealdb-core 3.2.4 源码 15MB / 1163 个 `.rs`；seekstorm 3.3.4 crate 源码 2.1MB / 25 个 `.rs`（本地 wc 全部 src ~35K 行），无原生依赖（default-features=false 时连 pdfium 都不编）。
4. **三个实证发现（迁移必读）**：① 空查询路径 `result_count_total` 含已删除墓碑（带词查询路径正确）→ 空查询的 total 走 agdb 计数；② `close()` 隐式 `commit()`（未 commit 且进程崩溃的数据才会丢）；③ 分词无 camelCase 拆分、连字符进 token（见 §2.6）。
5. 无事务/无 schema：搜索索引是 agdb 的**派生数据**，一致性由写路径（agdb 事务提交成功后同步索引）保证 + 启动全量重建兜底（数据量小，重建毫秒级）。

---

## 1. 候选对比（为什么是 seekstorm）

| 候选 | 否决/通过理由 |
|---|---|
| Meilisearch | 外置服务：独立进程/端口/API key/配置面，与 nail 单进程嵌入式形态冲突；原计划（agdb 报告 §0-4）仅因 SurrealDB FTS 不可用时需要外部检索才选它 |
| Tantivy | 同为纯 Rust 库、功能接近，但相关能力分散在多个 trait/crate（tantivy + tantivy-query-parser + 高亮需自拼 snippet）；无内建空查询全量+facet 过滤+多键排序一体 API |
| SurrealDB FULLTEXT | 现搜索实现：`@@`/`@AND@` 词项 AND、BM25(1.2,0.75)、`HIGHLIGHTS` 声明依赖、`search::score/highlight` 局限（见 `surrealdb_search_report.md`）；6 张 FTS 索引 + RRF 拼装 + 跨表边映射，且需为引擎怪癖打补丁 |
| **seekstorm** | crates.io 发布（3.3.4 四天前仍发版）、Apache-2.0、2015 起生产使用（seekstorm.com）、2020 上线、Rust 移植 2023、纯 Rust 无原生依赖、一次 `search()` 调用同时完成 词法检索+字段圈选+facet 过滤+多键排序+TopKCount。单作者项目（wolfgarbe，SymSpell 作者）但发布稳定活跃 |

---

## 2. 核心 API 知识（迁移必读）

### 2.1 库形态

```rust
use seekstorm::index::{create_index, open_index, IndexMetaObject, ...};

// 建索引（Mmap 文件库；RAM 索引传内存路径亦可）
let index_arc = create_index(path, meta, &schema, &Vec::new(), 11, true, None).await?;
// 重开
let index_arc = open_index(path).await?;
// IndexArc = Arc<RwLock<Index>>：search 并发安全（&self）；写操作内部排队
```

- `index_document`/`index_documents`：索引（实时可搜，commit 前需 `include_uncommitted=true` 才可查）。
- `commit()`：把未提交文档落盘（Mmap flush）。**`close()` 隐式 commit**（源码 index.rs:5017）。
- `indexed_doc_count`（含墓碑）vs `current_doc_count`（净数）vs `uncommitted_doc_count`。

### 2.2 数据模型

- `Document = IndexMap<String, Value>`（字段名 → JSON 值）；`doc_id`（usize）**自动分配**（index.rs:5290 `docid_global += 1`），不接受自定义。
- **业务 id 必须存为字段**（如 `id: "article:018f..."`）+ facet 声明，更新时用 `FacetFilter::String16` 精确反查 doc_id（探针 S8 实证）。
- 字段类型：Text（长文本，词法索引+最长字段归一化）、String16/32（≤65535/4G 字节，可同时词法+facet）、StringSet16/32（多值精确过滤）、**Json（字符串数组递归提取 → 词法索引 + 高亮，多值标签/评论用这个）**、Timestamp（Unix 秒，facet 范围过滤+排序）、U8..U64/I8..I64/F32/F64/Point/Bool/Binary。

### 2.3 写入

```rust
// 索引新文档（doc_id 自动分配）
index_arc.index_document(doc, FileType::None).await;
// 更新 = delete + insert（墓碑），需旧 doc_id
index_arc.update_document((doc_id, new_doc)).await;
// 删除（墓碑；空间不回收，直至上游实现 compaction）
index_arc.delete_document(doc_id).await;
index_arc.commit().await; // 落盘
```

> ⚠️ **墓碑**：删除/更新的旧文档不回收空间（源码 DeleteDocuments 注释明说），影响 Count 路径性能与索引体积；BM25 分数不更新。nail 规模可接受，需记录。
> ⚠️ **update 后 doc_id 变化**：update 是新 doc_id（旧 doc 墓碑化），后续再删/改需重新按业务 id 反查（探针 S7 实测踩坑后修正）。

### 2.4 读取

```rust
let rlo = index_arc.search(
    query,               // 词查询；空串 + enable_empty_query=true = 全量
    None,                // 向量查询（不用）
    QueryType::Intersection, // AND（nail 多词=AND）；另有 Union/Phrase/Not
    SearchMode::Lexical,
    enable_empty_query,  // true：空查询返回全部（q='' 即搜全部）
    offset, length,      // 分页
    ResultType::TopkCount, // 一次拿 top-k + result_count_total
    false,               // include_uncommitted
    field_filter,        // 圈选参与搜索的字段（范围复选框语义）
    vec![],              // query_facets
    facet_filter,        // FacetFilter：String16 精确 / Timestamp 范围 / U64 范围等
    result_sort,         // ResultSort{field, Asc/Desc}，多键按序 tie-break
    QueryRewriting::SearchOnly, // 关掉拼写纠正/补全（避免额外延迟）
).await;
// rlo.results: Vec<Result{doc_id, score(BM25)}>；rlo.result_count_total: 总命中
// 取回文档 + KWIC 高亮片段：
let hl = highlighter(&index_arc, vec![Highlight { field, name, fragment_number,
    fragment_size, highlight_markup: true, pre_tags: "<mark>", post_tags: "</mark>" }],
    rlo.query_terms).await;
let doc = index_arc.read().await.get_document(doc_id, false, &Some(hl), &HashSet::new(), &vec![]).await?;
```

### 2.5 过滤与排序语义（实测重点）

| 需求 | API | 语义 |
|---|---|---|
| 范围圈选（title/summary/author/…） | `field_filter: Vec<String>` | 只在这些字段里检索（空 = 全部字段） |
| 时间窗 from/to | `FacetFilter::Timestamp{field, filter: Range<i64>}` | Unix 秒，**半开区间**（`from..to+1` 表闭合） |
| 标签/业务 id 精确过滤 | `FacetFilter::String16{field, filter: Vec<String>}` | facet 值相等匹配，不存在 → 空结果非报错 |
| 排序（时间↓/标题↑/作者↑ 多键） | `ResultSort{field, order}` | 按 facet 字段排序，向量按序 tie-break；`base: FacetValue::None` 非 geo 忽略 |
| 总命中数 | `ResultType::TopkCount` → `result_count_total` | 带词查询正确；**空查询含墓碑（见 ⚠️）** |

> ⚠️ **空查询计数含墓碑（实证，探针 S12）**：q='' + enable_empty_query 时 `result_count_total` 把已删除文档计入（带词查询正确），且 **Count / TopkCount / Topk 三种 ResultType 全部受影响**——results 正确排除墓碑却报总数含墓碑（4 条结果报 6），内部自相矛盾。**根因定位（3.3.4 源码）**：`search()` 对「空查询 + 无 facet 过滤 + 无排序」走性能捷径 `search_iterator_index`（search.rs:1413 early-return），其中 `result_count_total = indexed_doc_count`（iterator.rs:410，**含墓碑的历史总数**），而 results 走 `get_iterator`（正确跳过 delete_hashset，iterator.rs:182）；作者有 `current_doc_count()`（活文档数）却没用。带词查询走倒排路径 `add_result_singleterm_multifield`（delete_hashset 检查在计数前，add_result.rs:127），故正确。**workaround**：q='' 场景的 total 改走 agdb 计数（文章数本来就是 agdb 的事实数据）。

### 2.6 分词（UnicodeAlphanumericFolded，与 nail_fts 对比）

- `to_lowercase()`（Unicode）+ 变音/重音折叠（`Café → cafe`，对应现 ascii 过滤器）+ 词字符连续成 token（Unicode 词字符含 CJK，中文连续文本整段一个 token，与现 class 分词行为一致）。
- **差异**：无 camelCase 拆分（`CamelCaseWord` 是单 token，搜 `camel` 不命中——探针 S11 实证）；无标点独立词元；`-`/`+`/`#`/`"` 保留在 token 内（`foo-bar` 单 token，搜 `foo` 不命中）。
- 备选：`UnicodeAlphanumeric`（不折叠变音）、`WhitespaceLowercase`（按空白）、`UnicodeAlphanumericZH`（中文分词，feature=zh，nail 不需要）。

### 2.7 无 schema / 无事务

- 无 `DEFINE FIELD/EVENT`：字段可缺省（缺字段的文档不参与该字段检索，源码 index.rs:1108）；字段校验在应用层（沿用 `from_value`）。
- 无 MVCC 事务：写操作是索引内部串行化 + 原子落盘，无跨语句回滚。**搜索索引是派生数据**，业务一致性由 agdb 事务 + 写后同步保证。

---

## 3. 实测验证（探针 12/12 全绿）

| 探针 | 验证内容 | 结果 |
|---|---|---|
| S1 | Intersection AND（多词全命中 / 缺一词 0 命中）+ result_count_total | PASS |
| S2 | field_filter 字段圈选（title only / summary only / 并集） | PASS |
| S3 | 空查询 + enable_empty_query → 全量 + 分页 | PASS |
| S4 | FacetFilter::Timestamp 时间窗（半开区间表闭合） | PASS |
| S5 | ResultSort 多键（ts desc + title asc）tie-break | PASS |
| S6 | KWIC 高亮：自定义 `<mark>` 标签 + fragment_size 截断 | PASS |
| S7 | update_document 内容替换 + delete_document 消失（含 update 后 doc_id 变化重定位） | PASS |
| S8 | 业务 id ↔ doc_id 反查（String16 facet 精确过滤；不存在 id 空结果非报错） | PASS |
| S9 | Json 数组字段多值 lexical 索引 + 高亮（标签/评论场景） | PASS |
| S10 | Mmap 持久化：commit 后 close → open_index 保留；close 隐式 commit | PASS |
| S11 | 分词行为：大小写/变音折叠、camelCase 不拆、连字符单 token | PASS |
| S12 | 墓碑：带词查询 total 正确排除；**空查询 total 含墓碑（Count/TopkCount/Topk 全部，实证计数缺陷）**；current_doc_count 净数 | PASS* |

\* S12 把空查询计数差异作为**记录在案的行为**（不判失败），workaround 见 §2.5。

---

## 4. 迁移映射表（nail 搜索层 → seekstorm）

| nail 现状（SurrealDB） | seekstorm 实现 | 备注 |
|---|---|---|
| 6 张 FULLTEXT 索引（title/summary/name/content/note/tag_name） | 单索引 6 字段（或按文章反规范化） | `schema.rs` FTS 定义全删 |
| `@@`/`@AND@` 词项 AND | `QueryType::Intersection` | `@AND@` OR 退化 workaround 删除 |
| `search::score` BM25 → Rust RRF 融合 | `LexicalSimilarity::Bm25f` 原生多字段打分（字段 boost） | `article_search_hits.rs` RRF 整块删除 |
| `search::highlight` + `HIGHLIGHTS` 声明 | `highlighter`：`fragment_number=0` 整段 or 按句切 + `<mark>` | 无需索引声明，P2 高亮一步到位 |
| `count_articles`（COUNT 索引 / GROUP ALL） | 带词查询 `result_count_total`；**空查询走 agdb 计数** | 空查询墓碑坑规避 |
| `uuidv7_min/max` 时间窗 SQL | `FacetFilter::Timestamp`（Unix 秒；契约粒度秒级天然匹配） | `latest_version_time_clause` 删除 |
| `ORDER BY ... COLLATE` + 后半段反向扫描 | `ResultSort` 多键 asc/desc | 反向扫描优化不再需要 |
| 跨表边映射（user_to_article 等）回文章 | 写路径反规范化进文章文档（author/tags/comments/latest_note 字段） | `expand_via_edges`/`map_comment_to_articles` 删除 |
| 版本说明只取最新版本命中 | 只索引最新版本 note 字段 | 设计 §9-4 天然满足 |
| `meta::id(id) IN $ids` 候选集 + OR 并集 | 无需候选集；`field_filter` 圈选 + 命中即上榜 | 设计 §9-5 OR 语义由多字段命中覆盖 |
| SurrealValue 中转 / `BindValue` | 无 SQL，直接 Rust 结构 | `repo/search.rs` 整文件删除 |

**可删除补丁清单**（约 800–1000 行）：`repo/search.rs` SQL 拼装层（105 行）；`logic/article_search_hits.rs` 的 source_hits/RRF/边映射（408 行）；`schema.rs` 6 张 FTS 索引 + ANALYZER（约 50 行）；`logic/article_search.rs` 的 `@AND@` OR 退化规避、`uuidv7_min/max` 时间窗、`build_search_where`（约 150 行）；surrealdb 依赖重树。

**不可删（业务本身）**：`GET /article/search` API 契约（§8.2，前端零改动）；范围/排序/时间窗参数解析；结果卡片组装（字段标签 + snippet 展示）。

---

## 5. 风险与待办

1. **空查询计数含墓碑**（实证，3.3.4 Count/TopkCount/Topk 全中）：q='' 的 total 必须走 agdb 计数或迭代器（iterator 正确跳过墓碑，iterator.rs:182）。已定 workaround，落实现加测试锁死；值得提 GitHub issue（4 条结果报 6 总数属自相矛盾）。
2. **墓碑不回收**：更新/删除越多索引越大（compaction 上游未实现）；nail 个人站规模无压力，但长跑后需重建索引（删除目录重建即可，数据可由 agdb 全量重建）。
3. **分词差异**：camelCase 不拆 / 连字符进 token——`ReactJS` 搜 `react` 不命中（现 nail_fts 可命中）。迁移时用站点语料对比验收，必要时接受或文档化。
4. **派生索引一致性**：agdb 事务成功后同步 seekstorm（update/delete/index + commit）；失败 → 重试，仍失败 → 启动时全量重建兜底（数据量小，毫秒级）。
5. **doc_id 自动分配**：每次更新/删除先按业务 id facet 反查 doc_id（S8 实证通过，代价一次查询）；更新后 doc_id 变化需重查（S7 实证）。
6. **单作者项目**：seekstorm（wolfgarbe）与 agdb（agnesoft 同作者）都是小社区但活跃稳定；nail 用到的只是库的一小角，出问题可 fork 修（Apache-2.0）。
7. **迁移验证**：搜索层重写后跑 `test/unit/back`（含 `article_search` 探针）+ `end_to_end`；探针 `probe/seekstorm_probe/` 保留为语义基线。
8. **上游跟进**：已提 GitHub issue [SeekStorm/SeekStorm#66](https://github.com/SeekStorm/SeekStorm/issues/66)（空查询计数含墓碑，含最小复现与根因定位）；后续升级 seekstorm 时回归探针 S12。

---

## 6. 与 agdb 报告的衔接

- agdb 报告 §0-4 原计划「FTS 交 Meilisearch」→ 本报告替换为 **SeekStorm（嵌入式库，无需外置服务）**；agdb 报告其余结论不变。
- 组合架构：**agdb = 事实数据（节点/边/事务/唯一性）**，**seekstorm = 搜索倒排索引（派生数据）**；写路径 = agdb `transaction_mut` 提交成功后同步索引；搜索 = 单索引一次 `search()`。
