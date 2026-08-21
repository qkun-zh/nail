# Exec Doc: `searcher` Crate Extraction (migrate + refactor)

Task code: sear · Research: `document/research/sear_searcher_crate.md`

## 1. Requirement

R: Extract the search engine wrapper into an independent workspace crate `searcher`
(zero workspace dependencies) as a **migration + refactor**: no legacy carried over.
The crate is independent, elegant, solid, reliable, high-performance, with a clean API.
Server keeps only an adapter (graph→IndexDoc, enrichment, orchestration, response trees).

Acceptance criteria:
1. `cargo tree -p searcher`: no workspace members in deps.
2. No seekstorm imports in server outside `repository/search` adapter glue.
3. read() cannot fail on uncommitted hits (F1: fetch flag == search flag).
4. Schema v6: title/author_name Text+facet=false (F7 fixed); dead `doc_type` removed.
5. Batched-commit path exists (replace_articles) and sync_user uses it.
6. Corrupt index dir at startup auto-heals via rebuild instead of panicking.
7. All tests green; searcher unit tests cover probe-verified invariants; fmt/clippy clean.

## 2. Scope

In: new `code/searcher/`, adapter rewrite of `repository/search*`, schema v6 bump,
typed document contract, batched commits, stats+tombstone policy hook, removal of dead
schema field/params.
Out: client/WASM (`common::search::SearchRange` unchanged), emailer/pow, response-tree
assembly (stays in logic), compaction features (engine lacks them).

## 3. Design

### 3.1 Modules

```
code/searcher/src/
├── lib.rs      # public re-exports + crate invariant docs
├── error.rs    # Error { Io, IndexCorrupt, Engine } (impl std::error::Error)
├── schema.rs   # v6 fields (serde-built), version marker, meta, dir pre-validation
├── field.rs    # SearchField enum (12 variants) ↔ engine field names
├── doc.rs      # VersionDoc / CommentDoc / IndexDoc → engine Document (single exit)
├── index.rs    # SearchIndex lifecycle + writes + stats
├── read.rs     # SearchRequest execution: filter/highlight/fetch
└── outcome.rs  # FieldHit / VersionHit / CommentHit / DocHit / SearchOutcome
```

Deps: seekstorm (default-features=false), serde_json. Dev-deps: tokio (rt+macros),
tempfile-style temp dirs via std. No indexmap/anyhow/tokio in public API.

### 3.2 Public API

```rust
pub enum IndexDoc { Version(VersionDoc), Comment(CommentDoc) }

pub struct SearchRequest {
    pub query: Option<String>,      // None => empty outcome (today's semantics)
    pub fields: Vec<SearchField>,   // empty => empty outcome
    pub from_seconds: Option<u64>,
    pub to_seconds: Option<u64>,
    pub offset: usize,              // RAW hit window; grouping/pagination policy lives in server
    pub limit: usize,
}

pub struct Stats { pub indexed: usize, pub live: usize, pub deleted: usize }

impl SearchIndex {
    pub async fn open_or_create(path: &str) -> Result<Self, Error>;
    pub fn was_recreated(&self) -> bool;
    pub async fn replace_article(&self, article_id: &str, docs: Vec<IndexDoc>) -> Result<(), Error>;
    pub async fn replace_articles(&self, batch: Vec<(String, Vec<IndexDoc>)>) -> Result<usize, Error>;
    pub async fn rebuild(&self, articles: impl IntoIterator<Item = (String, Vec<IndexDoc>)>) -> Result<usize, Error>;
    pub async fn read(&self, request: SearchRequest) -> Result<SearchOutcome, Error>;
    pub async fn stats(&self) -> Stats;
    pub async fn close(&self);
}
```

Outcomes are lean — only what the index can provide:

```rust
pub struct FieldHit { pub field: SearchField, pub snippet: String }
pub struct VersionHit { article_id, version_id, version_number, title,
                        author_id, author_name, field_hits: Vec<FieldHit>,
                        version_number_hit: bool }
pub struct CommentHit { article_id, version_id, comment_id,
                        author_id, author_name, content }
pub enum DocHit { Version(VersionHit), Comment(CommentHit) }
pub struct SearchOutcome { pub docs: Vec<DocHit> }
```

### 3.3 Refactors vs current code (no legacy)

| # | Legacy | New design |
|---|---|---|
| RF1 | fetch flag mismatch fails reads during sync (F1) | fetch always mirrors search flag |
| RF2 | title/author_name String16+facet silently corrupt past 65_535 (F7) | Text + facet=false; schema v6 marker forces one-time rebuild |
| RF3 | dead `doc_type` field indexed on every doc | removed (verified never read; discriminator = comment_id key) |
| RF4 | json! stringly-typed documents | typed VersionDoc/CommentDoc; Document conversion in one place |
| RF5 | 10-positional-arg SchemaField | serde-built from readable literals |
| RF6 | per-article commit (~27× overhead, P5) | replace_articles: N deletes+indexes, ONE commit |
| RF7 | "search-all then delete ids" wipe | rebuild = clear_index + streamed reindex |
| RF8 | tombstones grow unbounded, invisible | stats() exposes deleted count; server policy may trigger rebuild |
| RF9 | corrupt index panics process at open | pre-validation → auto-heal rebuild path (recreated=true) |
| RF10 | commit even when nothing changed | skip commit on no-op writes |
| RF11 | dead segment_number_bits param (hardcoded 11 on reopen) | dropped |
| RF12 | fat outcomes carrying db-enrichment placeholders | lean outcomes; server types unchanged; adapter maps |
| RF13 | sync_user swallows per-article errors via is_ok() | build-all-first (fail fast, nothing written), then single commit |

### 3.4 Crate invariants (documented in lib.rs, unit-tested)

1. Every write method commits before returning (only happens-before edge; P1/P2).
2. get_document realtime flag always equals the flag of the search that produced hits.
3. Never use pure match-all for committed-only snapshots (iterator ignores the flag).
4. Unit tests force single shard for deterministic id order.

### 3.5 Data flow

Write: handler → logic (best_effort wrappers stay) → repository.sync
→ build_documents (GraphRead/rows/soft-delete checks — server) → Vec<IndexDoc>
→ searcher.replace_article → facet-filtered delete + index + commit (crate).

Read: GET /search → logic (auth/parse/clamp — unchanged) → adapter maps
SearchRange↔SearchField and expands limit×32 (MAX_DOCS_PER_ARTICLE policy stays
server-side) → searcher.read (Topk realtime + ts range facet filter + <mark> highlight
+ consistent-flag fetch) → DocHit → adapter maps back to existing SearchDocOutcome +
db enrichment → assemble_tree/paginate (logic, zero changes).

Startup: open_or_create (marker mismatch or corrupt dir → wipe + recreated=true)
→ was_recreated() → sync_all via rebuild.

## 4. Slice breakdown

| Slice | Goal | Files | Red | Green | Exit |
|---|---|---|---|---|---|
| sear1 | Scaffold crate; schema v6 serde-built; marker; pre-validation; Error | workspace Cargo.toml, searcher/{Cargo.toml, src/{lib,error,schema}.rs} | `cargo test -p searcher` fails (missing) | schema/marker/validation tests pass; title==Text | `cargo test -p searcher` |
| sear2 | Typed contract + conversion; discriminator preserved | searcher/src/{doc,field}.rs | conversion test fails | values round-trip; version docs omit comment_id key | `cargo test -p searcher` |
| sear3 | Lifecycle + writes: open_or_create/replace_article(s)/rebuild/stats/close | searcher/src/index.rs | visibility test fails | P1/P2/P4-style tempdir tests: commit edge, delete immediacy, clear semantics, no-op skip | `cargo test -p searcher` |
| sear4 | Read path + outcomes + highlight + RF1 fix | searcher/src/{read,outcome}.rs | uncommitted-hit read fails old-style | P2-style concurrent-window test passes; snippets correct | `cargo test -p searcher` |
| sear5 | Server adapter: rows→IndexDoc stays; mapping both ways; sync_user batched (RF13) | server repository/search*.rs, infrastructure/{state,server}.rs | compile break mid-move | all server tests green; no seekstorm outside adapter | `cargo test -j 1 -p server` |
| sear6 | Cleanup: delete moved/dead code, tombstone-policy hook wiring optional | touched files | n/a | fmt+clippy pedantic clean; full baseline green | §1 baseline suite |

## 5. Open unknowns

None blocking. Highlight ±3-byte trim accounting cosmetic (research §8).

## 6. Verification plan

Per slice targeted `cargo test`; sear5-6 add `-j 1 -p server` + full baseline
(server/common/emailer/client + pow). CI gate via push after each slice.

## 7. Risks

- First deploy of v6 wipes indexes once (expected; warn log exists today for mismatches).
- seekstorm-internal panics on mid-session corruption remain (documented; unchanged risk).
- Multi-shard nondeterminism in tests → force_shard_number Some(1) where order matters.
- sync_user memory: builds all IndexDocs before writing (same as sync_all today; fine at nail scale).

## 8. Constraints

No unwrap/expect/new panics in crate code; English only; CRUD verbs; no Cargo.lock edits;
never touch target/dist/data/log; one commit per slice; never discard work.

## 9. Questions for user (gate-adopt)

1. Confirm schema v6 field changes (Text title/author_name, drop doc_type) — one-time rebuild?
2. Tombstone policy threshold (e.g. rebuild when deleted > 25% of indexed): wire now or defer?
3. sync_user fail-fast semantics change (RF13) acceptable?
