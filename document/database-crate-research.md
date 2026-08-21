# agdb Research Report for the `database` Crate Extraction

Status: research phase, pre-implementation. This report documents everything we learned about
agdb 0.13.2 (source-verified + probe-verified) before extracting the `database` crate out of
`code/back`. It feeds the slice plan in `REFACTOR_PLAN.md`.

Probe suite: `/tmp/opencode/agdb-probe` (throwaway crate, not part of the workspace).
Run with `cargo test --release -- --nocapture --test-threads=1`.
All numbers below were measured on this machine, release profile, in-process.
Suite status: **34/34 passing** (C1–C15 correctness, B1–B10 performance, P-A…P-E official-API
evaluation, T0–T4 trap verification).

## 1. Scope and method

Three sources of evidence, in order of authority:

1. **Source reading** — agdb 0.13.2 at
   `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/agdb-0.13.2/`
   (storage, transactions, query pipeline, builder, derive macros, collections).
2. **Correctness probes C1–C15** — one probe per semantic question that graph.rs depends on
   (upsert semantics, missing-key behavior, rollback, cascade, pagination, typed selects…).
3. **Performance probes B1–B10** — micro-benchmarks at 10k and 64k element scale covering every
   read/write strategy under consideration.

## 2. agdb architecture essentials

### 2.1 Storage backends — the truth about "mapped"

`DbAny` exposes three constructors (`db.rs:1294-1308`):

| Constructor | Backing struct | Reads | Writes |
|---|---|---|---|
| `new_memory` | `MemoryStorage` | RAM | RAM |
| `new_mapped` | `FileStorageMemoryMapped` | RAM | RAM **and** file |
| `new_file` | `FileStorage` | file | file |

Critical finding: **`FileStorageMemoryMapped` is not an mmap** (`file_storage_memory_mapped.rs`).
It holds a `FileStorage` plus a full `MemoryStorage` mirror:

- `new()` reads the *entire* file into RAM at open time;
- `read()` is served from RAM only;
- `write(pos, bytes)` writes to RAM **and** through the file path;
- `resize()` resizes both.

Consequences: startup cost and RAM footprint grow with total DB size; reads are always
memory-speed; every write pays double.

### 2.2 Write path and WAL

`FileStorage::write` (`file_storage.rs:150-160`) per call:

1. seek + read the old bytes at `pos` (read-modify pattern),
2. append a WAL record (`pos`, `len`, `value`) — up to 4 unbuffered syscalls,
3. seek + write the new bytes.

`std::fs::File` is unbuffered, so every step is a real syscall. The WAL performs **no fsync**:
`flush()` merely truncates the WAL (`set_len(0)`), and the source comment states explicitly that
`sync_data`/`sync_all` are avoided because they "result in extreme slowdown"
(`file_storage.rs:24-27`). Durability therefore relies on the OS page cache, not on fsync.
On clean `Drop`, pending WAL records are applied and the WAL cleared.

### 2.3 Transactions

`transaction()` / `transaction_mut()` take closures (`db.rs`, `transaction_mut.rs`):

- `Ok(_)` → commit (storage flush),
- `Err(_)` → rollback (storage restored from its own snapshot/WAL).

Any bare `exec_mut` outside a closure runs as its own implicit transaction. There is no
performance penalty for many small transactions versus one big one on memory storage (B3), and
on mapped storage the cost is dominated by per-*write* work, not per-commit work (B9).

### 2.4 Index internals — why index lookups are slow

Alias lookup uses `DbIndexedMap` (hash map → fast). Value indexes use `DbMultiMap`
(`collections/multi_map.rs`): open-addressing with **linear probing starting at
`stable_hash(key) % capacity`**, stopping at the first empty slot. Each probed slot is a storage
read. With a poorly-filled table this degrades badly, which matches measurement: an indexed
equality lookup costs ~98 µs/query at 10k elements while an alias lookup costs ~1.5 µs (B2).
A full scan is ~1.8 ms at 10k — the index wins over a scan only below roughly 20k elements,
and never approaches alias speed.

### 2.5 Graph model and traversal semantics

- Edge ids are **negative**; node ids positive. Removing a node cascades to its edges (C12).
- BFS distance counts **elements on the path**: `user -edge-> article` puts the edge at
  distance 1 and the article at distance 2. This is why nail's outgoing-edge helpers use
  `distance(1)` + `.edge()` and article fetches use `distance(2)` + `.node()` (C3, C10).
- `offset`/`limit` are applied by `LimitOffsetHandler` (`db_search_handlers.rs:134-151`) over
  **matching elements only**: the first `offset` matches are suppressed, traversal finishes
  after `limit + offset` matches. Pagination is therefore cheap and stable for a fixed graph
  (C11), but there is **no ordering guarantee** without `order_by`.
- **`order_by`** (`search_query.rs:176-216`): when set, the traversal runs to completion
  *without* limit/offset, all matches are sorted in memory, and only then sliced. The
  comparator calls `values_by_keys` per comparison — O(n log n) value fetches — and sorts
  missing keys last. This gives deterministic pagination at the cost of a full
  traversal + sort (P-C).
- Search algorithms: breadth-first (default), depth-first, index-backed equality, and A*
  path search. Index algorithm bypasses graph traversal entirely.
- Two footguns verified by probe **and** traced to source:
  - A rooted search `search().from(single_id)` reaches only the origin's **connected
    component**: `BreadthFirstSearch::expand` (`graph_search/breadth_first_search.rs:24-62`)
    follows `first_edge_from`/`edge_to` exclusively, so a graph-isolated element yields just
    itself (T1). The official algorithm docs state the contrast explicitly — `BreadthFirst`:
    "Examines each distance level from the search origin…", `Elements`: "Examines all elements
    in the database disregarding the graph structure". Full-graph scans must use the
    destination form `search().elements()`.
  - A bare `search()` query returns elements with **empty values** — `SearchQuery::process`
    (`search_query.rs:83-88`) constructs `DbElement { …, values: vec![] }` verbatim; the
    struct is documented as "Query to search for **ids**". Values require wrapping in
    `select().…ids(search…)` (T2).

### 2.6 Derive macros

- `#[derive(DbType)]` — maps a struct to a fixed key set; enables `try_into::<Vec<T>>()` from
  `QueryResult` and `into_query()`-style value building. Tolerates **missing optional keys**
  and **extra unknown keys** (C15) — ideal for schema evolution.
- `#[derive(DbElement)]` — the official entity-typing mechanism: persists
  `("db_element_id", "TypeName")` on the element and enables typed queries
  `select().elements::<T>()` and `where_().element::<T>()`. Cannot be combined with
  `DbType` on the same struct (conflicting impls); `DbElement` implies the value-mapping
  functionality.
- Subtlety verified in the derive source (`agdb_derive/src/db_type.rs:38-55`) **and corrected
  on second pass**: the `has_option` check skips the `db_id` field, so emptiness of
  `db_keys()` depends on Option fields *other than* `db_id`:
  - A row with no such fields (e.g. nail's `UserRow`) keeps its explicit key list —
    `select().element::<T>().ids([...])` then enforces key presence and **errors** on a
    missing key (T0).
  - A row with any optional field (e.g. nail's `ArticleRow.latest_version_id`) collapses
    `db_keys()` to an empty vec, which `SelectValuesQuery` treats as "fetch all values" —
    behaviorally identical to `select().ids()`, no presence enforcement.
  - This is documented behavior, not a bug: the official derive docs state "When reading
    elements if a type contains an `Option` all keys are always retrieved" — because a `None`
    optional is simply not stored.
- The typed *filter* (`db_element_id` condition) is orthogonal: it is only injected by
  `.search()` and only when `db_element_id()` returns `Some`, i.e. with the `DbElement`
  derive (P-B).

### 2.7 Housekeeping APIs

- `optimize_storage()` (runs automatically on `Drop` of `DbImpl`, `db.rs:1288-1292`) performs
  a **full compaction**: every record rewritten contiguously, file truncated, free lists
  cleared (`storage.rs:303-314`). Probe T4 measured a fragmented 2.98 MB file shrinking to
  159 KB (**18.7×**) on drop. The official docs for `backup`/`copy` reference this ("Consider
  calling optimize_storage() prior… to reduce the size"). Shutdown cost grows with DB size —
  negligible at nail scale, worth knowing.
- `backup(filename)`: for a memory DB this dumps the internal buffer to a file restorable via
  `DbMemory::new(filename)`; for file-backed DBs it copies the file. This enables a
  "work in memory, checkpoint to disk" strategy (see §4.2 B9/B10 discussion).
- `rename(filename)` moves both data and WAL files.

### 2.8 Bulk primitives (unused by nail today)

All source-verified and probe-verified (P-A, P-E):

- `insert().nodes().aliases([a1…aN]).values([[row1]…[rowN]])` — N nodes + N per-row aliases +
  N rows in **one query** (aliases zip by index; count derived from values).
- `insert().nodes().values_uniform(row).count(n)` — same row for n nodes.
- `insert().edges().from(many).to(many)` — paired edges when lengths are equal; `.each()`
  turns it into the cartesian product.
- `remove().search().elements().where_(…)` — conditional bulk removal; `QueryResult::result`
  reports the number of removed elements (P-D).
- `insert().values(…).search(…)` — conditional bulk update by the same pattern.

## 3. Inventory: nail's current usage vs official API

All 19 query shapes used by `code/back/src/repository/graph.rs` + `schema.rs`, with verdicts:

| # | Shape (as used today) | Verdict |
|---|---|---|
| 1 | `insert().nodes().aliases([a]).values(row)` ×9 sites | Official upsert-by-alias; keep (C1) |
| 2 | `insert().nodes().ids([id]).values(partial)` | Keep; beware stale-key trap (C1/C2) |
| 3 | `insert().edges().from(&to).values(...)` | Keep |
| 4 | `insert().values(...).ids([...])` (update by id) | Keep |
| 5 | `insert().index("key")` | Keep, but see §2.4 cost warning |
| 6 | `select().ids(alias)` resolve | Keep (~1.5 µs) |
| 7 | `select().values().ids(search().where_().ids())` — `read_rows` | **Replace**: 37.9× slower than direct `select().ids([...])` (B1); direct form also has better error semantics when paired with explicit existence checks (C7) |
| 8 | `select().values().ids(single)` | Replace with typed `element::<T>().ids()` (C15) |
| 9 | `select().values().search().index().value()` | Works; keep only where truly needed; prefer alias |
| 10 | `select().indexes()` | Absorb into `Database::open` (ensure-indexes) |
| 11–15 | `search().elements().where_()` variants: ids wrapper, key/value scan, `edge_count_to`, `not().keys()`, `GreaterThan` | All verified correct (C9); keep |
| 16 | one-hop: `from/to + distance(1) + .edge() + key filter` | Verified (C3); ~0.7 µs |
| 17 | two-hop: `distance(2) + .node() + not().keys() + offset/limit` | Verified (C10/C11); add `order_by` for deterministic pages (P-C) |
| 18 | `remove().ids(...)` ×14 sites | Keep; cascade confirmed (C12); bulk variant `remove().search()` available (P-D) |
| 19 | `remove().values(keys).ids(...)` | Keep |

Non-builder usage: `DbAny::{new_memory,new_mapped}`, `exec`/`exec_mut`, `transaction_mut`,
`RwLock` guards around the handle, matching on `DbErrorType::NotFound`,
`QueryResult::try_into`, `#[derive(DbType)]` on row structs. All stay, wrapped behind the new
crate boundary.

## 4. Probe evidence

### 4.1 Correctness (C1–C15, all passing)

| Probe | Question | Result |
|---|---|---|
| C1 | Alias insert on existing alias | Reuses node (official upsert). **Trap:** keys absent from the new values survive — an `Option` field going `Some→None` leaves the old value in place |
| C2 | Partial update by id | Only listed keys touched |
| C3 | One-hop typed edge query | Correct; edges live at distance 1 |
| C4 | Update values by ids | Correct |
| C5 | Duplicate unique index insert | Errors as expected |
| C6 | Alias resolve hit/miss | Miss returns `DbErrorType::NotFound` |
| C7 | Missing-key / dangling-id reads | Direct ids-select **errors**; search-wrapped silently skips. New API must make this explicit |
| C8 | Index tracks value replacement | Yes |
| C9 | Scan condition family | key/value, `not().keys()`, `edge_count_to`, `GreaterThan` all correct |
| C10 | Two-hop navigation | Articles at distance 2 from author |
| C11 | Pagination | Offset/limit over matches only; stable for fixed graph |
| C12 | Remove cascade | Node removal drops its edges; value-only removal keeps element |
| C13 | Rollback mid-chain | Closure error rolls back all writes in the closure |
| C14 | Alias rebinding | `insert().aliases().ids()` moves the alias |
| C15 | Typed select `element::<T>().ids()` | Full-values select + `try_into` tolerates missing optional keys and extra keys |

### 4.2 Performance (release, in-process)

| Probe | Scale | Measurement | Result |
|---|---|---|---|
| B1 | 10k | search-wrapped `read_rows` vs direct `select().ids()` | **37.9× slower** (wrapped) |
| B2 | 10k / 64k | alias vs index vs scan equality lookup | ~1.5 µs / ~98 µs / ~1.8 ms per query @10k; 4.7 µs / 34 µs / ~26 ms @64k |
| B3 | 10k | per-row transactions vs one big transaction (memory) | Nearly identical |
| B4 | 10k | one-hop typed edge query | ~0.7 µs each |
| B8 | 64k | mapped write (per-row txns) / mapped read 1024q / memory write | 60.8 s / 4.4 ms (~4.3 µs/q) / 2.75 s |
| B9 | 64k | mapped write in ONE transaction | 61.8 s ⇒ cost is per-write, not per-commit |
| B10 | 64k | plain-file write / read 1024q | 120.9 s (2× worse) / 23.9 ms (5.4× worse) |

Storage verdict: **`new_mapped` is the right default** (fastest reads, cheapest persistent
writes). `new_memory` is ~22× faster for bulk writes — relevant if we ever need a seeding or
rebuild job (write to memory, then `backup()` to disk). Plain `new_file` is dominated in every
dimension.

### 4.3 Official-API evaluation (P-A…P-E, all passing)

| Probe | Candidate API | Verdict |
|---|---|---|
| P-A | `insert().nodes().aliases(N).values(N rows)` single-query bulk seed | Correct; **1.95 s @64k vs ~2.2–2.7 s looped** on memory — modest speed gain, but one failure domain and one round of index/alias maintenance. Adopt for seeding paths |
| P-B | `#[derive(DbElement)]` + `select().elements::<T>().search()` | Auto-injects `db_element_id == "T"` filter; only `T` rows returned; `DbType`-only rows carry no marker. The official path to typed queries — requires data migration, defer behind new API (see §7) |
| P-C | `search().from(x).order_by([Asc(k)]).offset(n).limit(m)` | Deterministic pagination verified (reversed insertion, page exactly n+1…n+m). Full traversal + in-memory sort; fine at nail scale. Adopt for all paged listings |
| P-D | `remove().search().elements().where_(…)` | Conditional bulk removal; `result` = removed count (10/10). Adopt where nail currently does search-then-remove-in-loop |
| P-E | `insert().edges().from(many).to(many)[.each()]` | Paired when equal length, cartesian with `.each()` (2 vs 4 edges). Adopt for bulk relation creation |

Additional source-verified notes: `db_keys()` is empty only for structs with Option fields
*besides* `db_id` (§2.6, corrected on second pass) — so typed select-by-ids keeps presence
enforcement for `UserRow`-shaped rows but not for `ArticleRow`-shaped ones; the type filter
only exists in search mode with `DbElement`.

### 4.4 Second-pass verification of reported traps (T0–T4, all passing)

Every trap was re-checked against official comments and source lines, with a dedicated probe:

| Trap | Source evidence | Probe result |
|---|---|---|
| Rooted search sees only its connected component | `breadth_first_search.rs:24-62` (`expand` follows edges only); algorithm docs "from the search origin" vs Elements "disregarding the graph structure" | T1: isolated origin → 1 result; after wiring edges → 3 |
| Bare search returns empty values | `search_query.rs:87` constructs `values: vec![]`; doc "search for ids" | T2: bare → 0 values; select-wrapped → values present |
| `db_keys()` collapse for Option-bearing rows | `agdb_derive/db_type.rs:38-55` (`has_option` skips `db_id`); derive docs "all keys are always retrieved" | T0: `UserRow` keys non-empty & enforced; `ArticleRow` keys empty & tolerant — **report §2.6 corrected accordingly** |
| Drop compacts storage | `db.rs:1288-1292` Drop → `optimize_storage()`; `storage.rs:303-314` rewrite+truncate; backup/copy docs | T4: fragmented file 2 979 420 B → 159 028 B (**18.7×**) on drop |
| Upsert leaves stale keys | `insert_values_query.rs:152-165` per-key `insert_or_replace_key_value`, no removal pass; doc "insert or update key-value pairs" | C1 (round 1) |
| Missing-key asymmetry (error vs silent skip) | `select_values_query.rs:56` guard `if !is_search`; struct doc "All ids must exist… must have the requested properties" | C7 (round 1) |
| "Mapped" storage is RAM mirror + double write, not mmap | `file_storage_memory_mapped.rs:7-19` doc: "combines the FileStorage and MemoryStorage… write operations… in terms of both" | B8/B9/B10 timings consistent |
| WAL never fsyncs | `file_storage.rs:24-27`: "merely clears the WAL… relies on the OS… sync_data/sync_all… extreme slowdown" | B8 vs memory delta |
| Index multi-map linear probing | `multi_map.rs:141-142,223-224` start at `stable_hash % capacity`, walk slots, each slot a storage read | B2 index ~34 µs/q @64k |
| Edges at distance 1, target nodes at distance 2 | BFS `expand` pushes edges at d+1 then nodes at d+1; official test `full_search` ordering `[node1, edge3, edge2, edge1, node4, node3, node2]` | C3/C10 |

## 5. Traps and anti-patterns found in current graph.rs

1. **`read_rows` wraps every batch read in a search query** — 37.9× overhead for no benefit
   (B1). The direct `select().ids([...])` + `try_into::<Vec<Row>>()` path is strictly better
   and tolerates optional/extra keys (C15).
2. **Upsert-by-alias used as full update** — stale keys survive field transitions to `None`
   (C1). Callers must explicitly remove keys when clearing optional fields, or the new crate
   must offer a `replace_row` primitive that removes-then-inserts.
3. **Stringly-typed entity kinds** — `"user"` / `"article"` literals flow through
   `key("entity_type")` conditions. agdb offers first-class typing via `#[derive(DbElement)]`;
   adopting it would replace string comparisons with typed conditions — at the price of a
   data migration (`"type"` key → `"db_element_id"`).
4. **Value indexes are near-useless at nail's scale** — ~98 µs vs ~1.5 µs alias and ~1.8 ms
   scan at 10k. Any place we index-and-query could equally alias-and-resolve or scan, with
   simpler failure modes.
5. **`highest_version_number` lives in the substrate** — semver business logic inside the
   storage layer; moves back to `back` during extraction.
6. **Missing-key semantics differ by query shape** (error vs silent skip, C7) — invisible
   today because callers never hit the difference; the new API must pick one behavior
   explicitly.
7. **Rooted-search footgun** — any future refactor that converts a full-graph scan
   (`search().elements()`) into a rooted search (`search().from(id)`) silently shrinks the
   result set to the origin's connected component (T1). The new crate should expose
   scan-style reads only through methods whose names make the scope obvious.

### 5.1 Library behaviors: design or bug?

Verdict: **all ten are design, none are correctness bugs.** Every behavior is either stated
in official doc comments or directly evident in the source; several have an explicit
alternative API for the other semantics.

| Behavior | Classification | Evidence |
|---|---|---|
| Rooted search limited to connected component | Design | BFS by definition; `Elements` algorithm exists precisely for structure-free scans |
| Bare search returns empty values | Design | Doc: "Query to search for ids"; `select()` exists to fetch values |
| `db_keys()` collapses for Option-bearing rows | Design | Derive docs: "all keys are always retrieved"; `None` optionals are not stored |
| Drop-time compaction | Design (debatable ergonomics) | `Drop` impl; `backup`/`copy` docs reference size reduction |
| Property-level upsert leaves stale keys | Design | Doc: "insert or update key-value **pairs**"; no full-row-replace query exists in the API — this *is* the update semantics |
| Missing-key asymmetry (error vs skip) | Design | Struct doc states direct-path strictness; search is a filter, skipping non-matches is its contract |
| "Mapped" storage = RAM mirror + double write | Design (misleading *name*) | Struct doc describes exactly this composition |
| WAL without fsync | Documented tradeoff | Comment weighs durability vs "extreme slowdown"; consequence: OS-crash window, process-crash safe via WAL replay |
| Index multi-map linear probing slowness | Implementation limitation | Correct results, poor complexity under probing; not a defect |
| Edges at distance 1, targets at distance 2 | Design | Algorithm docs count elements on the path; consistent everywhere |

### 5.2 Business-code usage: intentional or misuse?

Audit of `repository/` against each library semantic:

| Usage | Verdict | Evidence |
|---|---|---|
| Search-wrapped `read_rows` (37.9× cost) | **Intentional but over-defensive** | `enrich_articles` builds a `HashMap` and `continue`s on missing rows (`article.rs:300-310`) — tolerance is deliberately coded for. But every call site derives ids from edges traversed *in the same lock scope*, and node removal cascades edges (C12), so a dangling id cannot occur. The defense guards an impossible state; the cost is pure waste |
| Upsert-by-alias vs stale keys | **Deliberately defused** | `latest_version_id` never transitions to `None`: deletion paths write a `""` sentinel (`delete.rs:349`), readers normalize `None`/`""` identically (`article.rs:354`) *and* recompute the truth from the graph when the pointer dangles (`live_latest_version`, `article.rs:356-369`). All other rows carry no optional fields, so upserts always write complete rows. The codebase clearly knows the semantics and routes around them |
| Stringly-typed kinds | Intentional convention | Centralized in `schema.rs` constants + `alias_of`; applied uniformly to nodes and edges. Pre-`DbElement` idiom, migration-ready |
| Value indexes | **Intentional and correct** | All 7 indexes (`seed.rs:31-39`) serve exactly one purpose: uniqueness constraints / identity lookup (email hash get-or-create, title/content_hash/name taken-checks). Constraint checks don't care about the ~34 µs probe cost; nothing uses indexes for hot-path listing speed |
| Absence-as-state (`soft_deleted` counter) | **Intentional, elegant exploitation of library semantics** | Counter removed entirely at ≤0 (`delete.rs:250-257`) so key *presence* is meaningful; listings filter alive nodes via `.not().keys(KEY_SOFT_DELETED)` (`version.rs:193`, `article.rs:437`, `search/db.rs:112,167`); flag checks use the missing-key-tolerant search form with `GreaterThan(0)` (`delete.rs:78-93`) |
| Pagination without `order_by` | Intentional, rides a **write-time invariant** | `create_version` enforces strictly-increasing semver (`NotGreater`), so edge insertion order == semver order == BFS traversal order; `versions_of` therefore returns ascending versions unsorted. User/article listings sort in memory (uuidv7 timestamps, name). The invariant is implicit — undocumented and unasserted; the new API should make it explicit (`order_by` or docs) |
| N+1 read patterns (listings, `enrich_articles`, `read_users`, `read_user_names`) | Intentional simplicity | Per-item `resolve_node_id`/`read_node`/flag-check loops throughout. Fine at nail scale; primary perf debt for the new crate's batch reads |
| Read-modify-write via held write-guard, no txn closure (`create_user`, `update_user_email`, `update_version`) | Deliberate but **single-writer-assumption-bound** | Safe today because the tokio `RwLock` write guard serializes the whole op. Becomes race-prone the moment ops move to per-op transactions or anything else shares the DB file. The new crate should push these into `write(\|w\| …)` scopes |
| Error-swallowing reads (`.ok().and_then(...)` in `owner_of`, `parent_article_of`) | Deliberate, mildly risky | Real DB errors degrade to "not found"; acceptable for denormalized display fields, but the new `Error` type should make fallthrough explicit |

Net assessment: the substrate misuses nothing. The genuine problems are (a) one
performance anti-pattern (`read_rows`) whose defensive justification cannot trigger,
(b) implicit invariants that deserve API-level enforcement, and (c) scattered N+1s —
all three are exactly what the `database` crate slice is positioned to fix.

## 6. Consequences for the `database` crate design

Carried from the accepted refactor sketch, now grounded in evidence:

- Closure-based scopes `Database::read(|r| …)` / `write(|w| …)` map 1:1 onto
  `transaction()`/`transaction_mut()`; nested `exec_mut` inside a scope joins the outer
  transaction (no extra cost, B3/B9).
- Zero agdb-type leakage: `NodeId` newtype over `DbId`; `EntityKind`/`EdgeKind` enums replace
  string constants; a `Row` trait binds row structs to kinds; crate-private `Error` with an
  explicit `NotFound` variant (resolving the C7 ambiguity at the boundary).
- `Database::open` ensures indexes exist (absorbing `existing_index_keys`) and picks the
  storage backend (`new_mapped` default; `new_memory` for tests).
- Read paths standardized on direct `select().ids()` typed selects (kills anti-pattern #1);
  writes go through explicit `set`/`clear` primitives so the stale-key trap (#2) cannot bite
  silently.
- Paged listings gain `order_by` (P-C) so pagination is deterministic, not BFS-incidental;
  the current insertion-order==semver-order invariant (§5.2) becomes documented or enforced.
- Read-modify-write flows (get-or-create, taken-checks, counter adjustments) move inside
  `write(|w| …)` scopes so correctness no longer depends on the app-level single-writer lock
  (§5.2).
- Bulk operations use the official single-query primitives (§2.8): seeding via
  aliases+values bulk insert (P-A), conditional removals via `remove().search()` (P-D),
  bulk edges via `from(many).to(many)[.each()]` (P-E).
- Business logic (`highest_version_number`, cedar entities, search indexing policy) stays out
  of the crate.

## 7. Open questions

1. Adopt `#[derive(DbElement)]` now (typed conditions, official direction) or later (requires
   migrating existing `"type"` keys)? Leans "later, behind the new API" — the enum-based kind
   system gives most of the benefit with zero data migration. When adopted, `P-B` shows the
   search-side filter comes for free; note the `db_keys()`-empty subtlety for Option-bearing
   structs (§2.6).
2. Bulk-seeding strategy if import volume ever grows: memory-db + `backup()` (22× faster
   writes) vs accepting mapped-storage write cost, with the single-query bulk insert (P-A)
   as the in-between option. Not needed at current scale.
3. Whether any remaining value index earns its keep once alias resolution covers the hot
   paths — decide during slice 2 with real query traces.
4. Shutdown compaction (`optimize_storage` on Drop, §2.7): acceptable as-is at nail scale;
   revisit only if DB size ever makes shutdown noticeable.

## Appendix: probe suite

`/tmp/opencode/agdb-probe` — single binary, tests `c1..c15`, `b1..b10`, `pa..pe`, `t0..t4`,
plus debug helpers. `cargo test --release -- --nocapture --test-threads=1`. 34/34 passing.
The crate is throwaway; the durable artifacts are the findings in this report.
