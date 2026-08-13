# SurrealDB 3.2.4 搜索功能源码调研 + 行为实测报告

- 调研对象：`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/surrealdb-core-3.2.4/`
- 项目使用版本：surrealdb 3.2.4（`code/back/Cargo.lock`）
- 实测方式：全新 `Mem` 库裸 SQL + 10 个探针（`test/unit/back/repository/fulltext_probe.rs`，已登记进 `harness.rs`，全部通过）
- 日期：2026-08

---

## 0. 结论速览

1. SurrealDB 的"搜索"是 **FULLTEXT 倒排索引 + `@@`/`@AND@`/`@OR@` MATCHES 运算**，不是传统 DB 的 `SEARCH` 子句。
2. 匹配粒度是**整词项**（token），大小写不敏感；不做子串/拼音/模糊匹配。
3. `@AND@` 走倒排索引（`FullTextScan`），命中即 bitmap 查找，**非全表扫**。
4. `search::highlight` / `search::offsets` 依赖索引声明 `HIGHLIGHTS`；不声明则返回原串/`NONE`。项目三张 FTS 索引均未声明。
5. `~`（Like）在 3.2.4 顶层**不是**可用的二元运算符（解析报错）。
6. FULLTEXT 默认开启 BM25 打分 `BM25(1.2,0.75)`。

---

## 1. 源码架构

### 1.1 组件构成

| 层次 | 文件 | 职责 |
|---|---|---|
| 倒排索引 | `src/idx/ft/fulltext.rs` | 增量(segmented-log)倒排、BM25 打分、compaction、高亮/offset 存储 |
| 分析器 | `src/idx/ft/analyzer/tokenizer.rs` | 分词器：blank / class / camel / punct |
| 分析器 | `src/idx/ft/analyzer/filter.rs` | 过滤器：ascii / lowercase / uppercase / ngram / edgengram / snowball(stemmer) / mapper |
| 分析器 | `src/idx/ft/analyzer/mapper.rs` | 词项映射（文件） |
| 分析器出口 | `src/idx/ft/analyzer/mod.rs` | `generate_tokens` / `analyze_content` / 频率与 offset 提取 |
| 高亮 | `src/idx/ft/highlighter.rs` | `search::highlight` 渲染、offsets 收集 |
| 偏移 | `src/idx/ft/offset.rs` | `Offset{index,start,gen_start,end}` |
| 索引定义 | `src/sql/index.rs`（ToSql）、`src/expr/statements/define/index.rs` | `FULLTEXT ANALYZER x HIGHLIGHTS` |
| 求值 | `src/exec/physical_expr/matches.rs` | `@@`/`@N@` 判定（KV 查 doc_id + bitmap 查） |
| 扫描算子 | `src/exec/operators/scan/fulltext.rs` | `FullTextScan` 算子 |
| 函数 | `src/fnc/search.rs` | 分发 |
| 函数 | `src/exec/function/builtin/search.rs` | `search::analyze/score/highlight/offsets/rrf/linear` 实现 |
| 语法 | `src/syn/parser/expression.rs:537` | `parse_matches` 解析 `@@`/`@AND@`/`@OR@`/`@N@` |

### 1.2 核心结构：安全全文索引（全注释签语义）

`FullTextIndex`（`fulltext.rs:151`）：
- `ikb`：IndexKeyBase（key 前缀）
- `analyzer`：分析器
- `bm25`：`Option<Bm25Params>`——只有在 `DEFINE INDEX ... FULLTEXT ...` 时默认启用（见下）
- `highlighting: bool`：**决定是否落词项 offsets**。`index_content` 分支：`highlighting` 为 true → `index_with_offsets`；false → `index_without_offsets`（只存 term frequency，不存 positions）。

BM25 打分（`fulltext.rs:1173` `compute_bm25_score`）：
```
idf = ln((N − n + 0.5)/(n + 0.5))，clamp ≥ 0
tf′  = 1 + ln(tf)
score = idf·(k1+1)·tf′ / (tf′ + k1·(1−b + b·doc_len/avg_len))
```
参数 `k1=1.2, b=0.75`（`INFO FOR TABLE` 实证）。

### 1.3 分析器（nail_fts，项目所用）

`code/back/src/repo/schema.rs:199` 定义：
```sql
DEFINE ANALYZER nail_fts TOKENIZERS blank, class, camel, punct FILTERS ascii, lowercase
```

- **blank**（`tokenizer.rs:358`）：遇空白切词。
- **class**（`tokenizer.rs:366`）：按字符类（字母/数字/标点）切分。
- **punct**（`tokenizer.rs:382`）：标点作为**独立词项**（isolated token）。
- **camel**（`tokenizer.rs:392`）：大写字母起始新词（`CamelCaseWord → camel case word`）。
- **ascii**（`filter.rs:134`）：`deunicode` 归一，`Café → cafe`。
- **lowercase**（`filter.rs:125`）：小写。

多个 tokenizer 组合时，任一返回"起始新词/独立词/不可切"即生效（`tokenizer.rs:209`）。

### 1.4 MATCHES 求值（matches.rs 关键点）

`MatchesOp`：
- 纯索引驱动：`fti.get_doc_id(rid)` → `qt.contains_doc(doc_id)`（bitmap 查），**无 tokenize 回退**（`matches.rs:2-7`、`:218-222`）。
- 某字段无 FULLTEXT 索引 → 恒 `false`（`matches.rs:143-145`）。
- 空查询词项 → `false`（`matches.rs:207`）。
- 索引懒加载并按表+idiom 缓存（`ft_cache: OnceCell`）。

查询词项提取 `extract_querying_terms`（`fulltext.rs:442`）：跑分析器 `FilteringStage::Querying`（ngram/edgengram 在 Querying 阶段被跳过——`filter.rs:78-84`），再合并 delta。

### 1.5 求交集/并集（`fulltext.rs:703/744`）

- `@@`（无操作符）/`@AND@`：**交集**（须全数词项命中）；任一词无文档 → 空。
- `@OR@`：**并集**（任一命中）。

---

## 2. 实测行为（10 探针实证结果）

### 2.1 分词粒度
输入 `'Hello World CamelCaseWord testing Café'` → 输出：
```
hello  world  camel  case  word  testing  cafe
```
- ascii 归一：`Café → cafe`。
- camel 拆分：`CamelCaseWord → camel case word`。
- 整词粒度证明：搜索词项 `ell` 不命中含 `Hello` 的文档。

### 2.2 `@@` / `@AND@` / `@OR@`
- `@@ 'hello world'` ≡ `@AND@ 'hello world'` → 同文档集（空格分隔词项默认 AND）。
- `@OR@ 'hello machine'` → 命中既含 hello 又含 machine 的文档并集。
- 大小写不敏感：`HELLO` 命中 `Hello`。

### 2.3 走索引，非全表扫
```sql
EXPLAIN SELECT id FROM article WHERE title @AND@ 'hello'
```
```
SelectProject [ctx: Db] [projections: id]
    FullTextScan [ctx: Db] [index: article_title_fts_idx, query: hello]
```

### 2.4 无 FTS 索引字段
`WHERE id @AND@ 'hello'`（id 无 FULLTEXT 索引）→ 空结果集（恒 false）。

### 2.5 打分
```sql
SELECT id, search::score(0) AS s FROM article WHERE title @AND@ 'machine learning' ORDER BY s DESC
```
两篇含 machine+learning 的文档分数不同且降序（1.545 > 1.355），证明 BM25 词频/长度归一化生效。

### 2.6 高亮与偏移（关键限制）
- 索引**不**声明 `HIGHLIGHTS`（项目现状）：
  - `search::highlight('<b>','</b>',0)` → 返回**原串，不包裹**（`Hello World`）。
  - `search::offsets(0)` → **`NONE`**。
  - 根因：`FullTextIndex.highlighting=false` → `index_without_offsets`，不落 positions。
- 索引声明 `HIGHLIGHTS` 后：
  - `Hello <b>World</b>`
  - `search::offsets(0)` → `{ "0": [{s,e}, ...] }`
- **注意**：`HIGHLIGHTER(true)` 是错误语法（解析报错），正确关键字是 `HIGHLIGHTS`（`src/sql/index.rs:303`）。

### 2.7 fuzzy / LIKE
- 顶层 `~`（Like）在 SurrealDB 3.2.4 **不是**可用的二元运算符（`SELECT ... WHERE title ~ 'hell'` 报 "Unexpected token `~`"）。
- 项目 fuzzy 用 `string::lowercase(title) CONTAINS ...`（子串 + 大小写不敏感），实测命中；这与源码「MATCHES 纯索引、无回退」一致——fuzzy 本质是另一条 CONTAINS 扫描路径。

### 2.8 项目索引现状（`INFO FOR TABLE article`）
```sql
article_summary_fts_idx → FULLTEXT ANALYZER nail_fts BM25(1.2,0.75)
article_title_fts_idx   → FULLTEXT ANALYZER nail_fts BM25(1.2,0.75)
```
（另 `user_name_fts_idx` 同。三张均未声明 `HIGHLIGHTS`。）

---

## 3. 对项目的启示 / 风险

1. 三张 FTS 索引未声明 `HIGHLIGHTS`，故 `search::highlight` / `search::offsets` 在项目里**不可用**（返回原串 / NONE）。项目当前未用到，无损；若未来要做"命中文档里高亮关键词"，需给索引加 `HIGHLIGHTS`（会额外落词项 positions，索引体积增大）。
2. 全文检索语义严格为**整词项 AND/OR**，无子串/模糊。fuzzy 模式显式退化为 `CONTAINS` 全表扫——识别到性能换匹配粒度的开关，正确。
3. `@AND@` 走倒排索引（`FullTextScan`），符合 `logic/article_search.rs` 注释里"倒排命中、免全表扫"的判断。
4. 中文内容：token 粒度不适合 CJK，确认"站点不面向中文"（LOW L72）成立。

---

## 4. 交付物

- 新增探针：`test/unit/back/repository/fulltext_probe.rs`（10 探针，全绿）。
- 登记：`test/unit/back/harness.rs` → `repository_fulltext_probe`。
- 知识库更新：`ai_must_read_and_follow/knowledge_base.md` → C1 节（分词细节、@@/@AND@ 语义、HIGHLIGHTS 依赖、~ 不可用、BM25 默认参数）。