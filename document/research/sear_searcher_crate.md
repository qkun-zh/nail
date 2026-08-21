# seekstorm Research Report for the `searcher` Crate Extraction

Status: research complete (source + probe double evidence). Counterpart of
`database-crate-research.md`. Probe suite: `/tmp/opencode/searcher-probe` (throwaway).
Evidence tags: **[S]** source (file:line), **[P]** probe.

## 1. Requirement and questions

R₀: extract the searcher (`code/server/src/repository/search/`) into an independent,
highly decoupled `searcher` crate, mirroring how `database` was extracted.

Unknowns: index lifecycle semantics · document model · write/commit visibility ·
delete/tombstone behavior · search/highlight APIs · docstore reads · official
alternatives for nail's workarounds · schema limits.

agdb re-check: no full-text/ranking capability in its public surface (lib.rs:37-178) [S]
— "search inside agdb" is not an alternative.

## 2. seekstorm 3.3.5 essentials

### 2.1 Lifecycle
- Dir holds meta.json/schema.json/synonyms.json/shards/. `create_index` panics on bad
  path (to_str().unwrap() :1932, create_dir_all().unwrap() :1938); `open_index` panics on
  corrupt JSON (:3837,:3841) and read_dir (:3854). Wrapper must pre-validate. [S]
- `open_index` hardcodes segment bits 11 (:3867) and **mute=false** (:3868) → stdout
  noise after restart regardless of create-time mute. [S]
- Shard count defaults to min(available_parallelism, physical cores) when None (:2055). [S]
- No official schema-version mechanism → nail's marker file (search.rs:23-24,96-105)
  is necessary. [S]

### 2.2 Document model
- `Document = IndexMap<String, Value>` (index.rs:499). [S]
- `SchemaField::new(field, store, index_lexical, index_vector, type, facet, longest,
  boost, dict_source, completion_source)` (index.rs:1179-1190); private
  `indexed_field_id`/`field_id` → struct literals impossible outside crate; serde form
  works (P8). [S][P]
- Non-stored fields stripped from docstore (doc_store.rs:230-242): nail's doc_type
  never comes back from get_document. Missing fields accepted without error. [P]

### 2.3 Write path
- docid assigned synchronously pre-spawn (index.rs:5284-5293) → id order == call order [P];
  then spawned onto global `INDEX_RUNTIME` LazyLock<Runtime> (lib.rs:482, index.rs:5295).
- Per-shard `Semaphore::new(1)` serializes indexing; `index_documents` returns before
  tasks land (P1: realtime search right after saw 0 docs). Commit acquires the same
  permit (commit.rs:120-123) → commit-after-index is race-free; commit = only
  happens-before edge. [S][P]
- Auto-commit at 64k block boundary per shard (index.rs:5515-5523). [S]

### 2.4 Commit/durability
- Hard commit: RAM level-0 → compressed roaring blocks, mmap flush, no fsync — crash
  window like agdb WAL finding. Expensive; official guidance: avoid manual commits
  unless needed (commit.rs docs :66-89). [S]

### 2.5 Deletes
- Tombstones (delete_hashset + delete.bin), immediately effective, independent of
  commit; compaction NOT implemented in 3.3.5; official advice prefers full reindex
  over deletes (index.rs:5080-5142). [S]
- `clear_index(&mut self)` (index.rs:4920-4945): removes dictionary/completions,
  clears shards, keeps schema; does NOT reset `docid_global`. [S][P]
- `delete_documents_by_query` = search+delete loop with length cap (index.rs:5149). [S]

### 2.6 Search
- 15-param trait method is the official library API (search.rs:1153-1170); no struct
  params API outside REST server. Param order verified. [S]
- Empty query + enable_empty_query + empty facet_filter → match-all iterator path
  (search.rs:1413-1422) which IGNORES include_uncommitted (`_include_uncommitted`,
  iterator.rs:360-411). Term queries respect the committed boundary. With non-empty
  facet_filter the normal path runs (find_document_ids_by_article unaffected). [S][P]
- ResultType::TopkCount fills result_count_total; Topk does not. FacetFilter enum:
  String16/StringSet16/String32/StringSet32{field,filter}, Timestamp{field,range},
  … AND across fields, OR within values. result_sort exists, unused by nail. [S]

### 2.7 Highlighter
- Aho-Corasick over query terms; results written under Highlight.name (or overwrite
  field if name empty) during get_document (highlighter.rs:71-103, doc_store.rs). [S]
- fragment_number=0 → whole field as one fragment; truncation window (highlighter.rs:
  149-179): first-match-end > size → keep tail; else if len > size → keep
  `[0 .. first whitespace ≥ size]`; no whitespace → NO truncation. Byte-based,
  char-boundary adjusted. markup skipped for single 1-char term (:225-226). [S][P]

### 2.8 Docstore reads
- `get_document(doc_id, include_uncommitted, highlighter, fields, distance_fields)`
  inherent method; Err("not found") for deleted/out-of-range/**uncommitted while
  flag=false** (doc_store.rs:38-54). Decompress paths unwrap → panic on corruption. [S]

### 2.9 String16/StringSet16 facet limit — silent corruption
- Facet write arms guarded `facet.values.len() < u16::MAX` (index.rs:5751,5765);
  overflow falls to `_ => {}` (index.rs:5825) → cell never written → stays 0 →
  aliases to FIRST inserted value. String32/Set32 use u32::MAX (5778,5792). [S]
- P3 demonstrated: 66k distinct titles → filter title=="title-000000" returned 466
  (= 65_535-cap skip 465 + genuine 1, exact); last-title filter returned 0; lexical
  query still found it. [P]

## 3. Findings vs nail's current implementation

| # | Current | Verdict | Evidence |
|---|---|---|---|
| F1 | read(): search realtime=true (search.rs:254), fetch false (:292), error aborts via ? (:298) | Bug: uncommitted hit fails whole search during sync windows | S+P2 |
| F2 | sync_all: match-all(Topk, length=live) → delete ids (search.rs:161-191) | Works; official clear_index simpler; match-all ignores realtime=false (benign today: full reindex follows) | S+P1/P4 |
| F3 | sync()/sync_user(): delete+index+commit per article (search.rs:137-159) | Correct but ~27× slower than batched; tombstones grow monotonically; latency scales with total-ever-indexed (8.2ms @ 4k indexed/180 live) | P5 |
| F4 | Hand-rolled schema marker + rebuild | Necessary (no official mechanism); move into wrapper | S |
| F5 | SchemaField::new 10 positional args | Replace with serde-from-json literals | P8 |
| F6 | Untyped Document building | No official typed layer; typed builder belongs in searcher crate | S |
| F7 | title/author_name String16+facet (unbounded distinct values) | **Silent corruption past 65_535**; fix to Text or String32; needs version bump to "6" + rebuild | S+P3 |
| F8 | segment_number_bits plumbed through open_or_create_with_segments | Dead param on reopen (hardcoded 11) | S |
| F9 | mute=true at create | open_index forces mute=false → stdout noise anyway | S |
| F10 | sync_all relies on match-all snapshot semantics | Flag ignored in that path; benign now, must be preserved knowingly | S+P1 |
| F11 | read() fetchability assumption | Fix: fetch with same realtime flag or fetch only post-commit | P2 |
| F12 | clear_index as wipe primitive | Safe; docids keep counting (never treat as stable/restarting) | P4 |
| F13 | content highlight fragment_size=4096 | Window semantics, not hard cap; space-less text untruncated; tail loss possible | S+P7 |
| F14 | Readable schema construction | serde-from-json only | P8 |
| F15 | Version/comment discriminator contains_key(comment_id) | Correct: version docs omit the key (document.rs:187-209 vs :236) | S |

## 4. Decoupling design (draft for exec doc)

Hexagonal cut, zero workspace deps (mirrors database):
- searcher owns: schema + version marker + rebuild-on-mismatch, typed IndexDoc
  contract, lifecycle (open/create/clear/close), replace/replace_all, query execution +
  highlighting, outcome types, wrapper invariants.
- server keeps: graph→IndexDoc adapter (rows, GraphRead, has_soft_deleted_flag),
  article-id resolution, db enrichment, orchestration, SearchRange↔SearchField mapping.
- Wrapper invariants (probe-backed): every write ends with commit(); get_document uses
  same realtime flag as search; no match-all for committed-only snapshots; serde-built
  schema; directory pre-validation before open_index.

```rust
pub enum IndexDoc { Version(VersionDoc), Comment(CommentDoc) }
pub struct SearchRequest { query: Option<String>, fields: Vec<SearchField>,
                           from_seconds: Option<u64>, to_seconds: Option<u64>,
                           offset: usize, limit: usize }
impl SearchIndex {
    pub async fn open_or_create(path: &str) -> anyhow::Result<Self>;
    pub fn was_recreated(&self) -> bool;
    pub async fn close(&self);
    pub async fn replace(&self, key: &str, docs: Vec<IndexDoc>) -> anyhow::Result<()>;
    pub async fn replace_all(&self, docs: Vec<IndexDoc>) -> anyhow::Result<u64>;
    pub async fn read(&self, request: SearchRequest) -> anyhow::Result<SearchOutcome>;
}
```

## 5. Probe evidence summary

| Probe | Question | Result |
|---|---|---|
| P1 | delete/commit visibility, reopen | PASS + fire-and-forget race + match-all ignores flag |
| P2 | uncommitted fetch trap | PASS — reproduces F1 exactly |
| P3 | String16 facet overflow | PASS — 466≠1 and 0 results; mechanism confirmed in source |
| P4 | clear_index | PASS — content reset, schema kept, reopen ok, docids continue |
| P5 | sync cost model | PASS — 27× per-article commit overhead; 135ms/article-sync at toy scale; tombstone-driven latency |
| P6 | value round-trip | PASS — arrays/i64/unicode preserved; store=false never returns; missing fields ok |
| P7 | highlight truncation | PASS — tail lost; window mechanics explained |
| P8 | SchemaField serde | PASS — serde round-trip exact; struct literal impossible |

## 6. Impact on R₀

No contradiction of R₀ — extraction is feasible and clean. R₁ additions:
1. title/author_name field-type fix (F7) must ride along (schema v6 rebuild).
2. read() fetch-flag fix (F1/F11) rides along.
3. Batched commit option for sync_user (F3) — optional slice, ask user.

## 7. Open items (user input)

1. Adopt field-type change (title/author_name → Text) given it forces one-time full rebuild?
2. Include tombstone-rebuild policy (periodic clear_index when deleted ratio > threshold)?
3. Stdout noise from open_index mute=false: capture/ignore?

## 8. Open technical items

- Exact trim/ellipsis accounting in highlight output (±3 bytes observed) — cosmetic.
- Whether non-facet StringSet16 (tags) shares any lexical-path limit — P3 proved
  lexical path survives for String16; tags likely fine.
