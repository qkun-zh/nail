# D7kq — database crate extraction

## 1. Requirement

Extract a standalone workspace crate `code/database` that owns all agdb access,
then split `authorizer` and `searcher` out of `back` the same way. Pinned:

- R1: `database` is a leaf crate (like `cache`/`emailer`); its public API leaks
  zero agdb types.
- R2: `back` no longer contains a storage substrate: `repository/graph.rs` and
  `repository/schema.rs` substrate parts are deleted; row structs remain in
  `back` as domain models implementing `database::Row`.
- R3: Behavior preserved: same endpoints, same data on disk (no migration of
  existing `.agdb` files), same soft-delete semantics, same uniqueness
  constraints.
- R4: `authorizer` crate owns Cedar compilation/entities/policies;
  `searcher` crate owns SeekStorm indexing/querying. Both depend on
  `database`, never on `back`.
- R5: Every slice: red → green → fmt/clippy clean → one commit → push → CI
  green.

Acceptance: full suite green in CI after each slice; final tree has crates
`database`, `authorizer`, `searcher` as workspace members; no `agdb` reference
remains anywhere under `code/back`.

## 2. Scope

In scope: `code/database` (new), `code/back/src/repository/**`,
`code/back/src/logic/**` (call-site adjustments only),
`code/back/Cargo.toml`, `code/Cargo.toml`, `test/unit/database/**`,
`code/authorizer` (new), `code/searcher` (new), build.rs moves.

Out of scope: frontend, proxy, common, pow, cache, emailer, configuration
files, data migration of existing databases, API changes.

## 3. Design decisions

1. **Closure-scoped transactions** — `Database::read(|r| …)` /
   `Database::write(|w| …)` map to agdb `transaction`/`transaction_mut`;
   write scope commits on `Ok`, rolls back on `Err`. Replaces held-guard
   read-modify-write; correctness no longer rests on the app-level single
   writer. Closures are synchronous (µs-scale ops).
2. **Node vocabulary, one name per concept** — node (stored thing), edge
   (link), row (typed data on a node). No `entity`/`record` synonyms.
3. **Identity** — alias scheme `"{kind}:{business_id}"`; `business_id` is the
   domain UUIDv7 string used by logic/interface and stored in row fields;
   `NodeId` is the opaque graph handle valid only inside database calls. The
   crate owns both directions (`resolve`, alias generation). Indexes derive
   from `NodeKind`/`EdgeKind` tables and are ensured at `open` (absorbs
   `existing_index_keys`).
4. **Rows stay in `back`** — domain structs implement `database::Row`
   (`const KIND`, `business_id`, key/value conversion via the crate's own
   `Value` mirror). The crate stays generic over row content.
5. **Reads are direct by default** — `read_nodes` uses the direct ids select
   (37.9× faster than the old search-wrapped wrapper); strict: missing id is
   `NotFound`. Scans are separate methods with explicit naming (rooted-search
   footgun eliminated by construction). Deterministic pagination via
   `Order` + offset/limit pushdown (`order_by`).
6. **Writes are explicit** — `insert_node` is upsert-by-alias with full-row
   replace semantics: diff existing keys, clear keys absent from the new row,
   then write (stale-key trap fixed inside the crate). Optional-field
   transitions and counters use `set_key`/`clear_key`. Edges idempotent.
7. **Own error type** — `Error::{NotFound{kind,id}, Conflict, Invalid,
   Storage}`; one rule: absent required row/key/value = `NotFound`.
8. **Storage** — `open_mapped(path)` for file-backed (current behavior:
   mirror + WAL), `open_memory(name)` for tests.
9. **Business logic leaves the substrate** — `highest_version_number` returns
   to `back`; soft-delete counter manipulation stays in
   `back::repository::delete` expressed via `read_value`/`set_key`/`clear_key`.

### 3.1 Public API contract (slice 1 implementation target)

```rust
// Values & identity
pub enum Value { Int(i64), Text(String) }
pub struct NodeId(/* opaque u64 */);
pub enum NodeKind { User, Article, Version, Comment, Tag, Role, Permission }
pub enum EdgeKind { /* 8 variants = current schema constants */ }

pub enum Error {
    NotFound { kind: NodeKind, id: String },
    Conflict(String),
    Invalid(String),
    Storage(String),
}

// Row protocol (implemented by back's domain structs)
pub trait Row: Sized {
    const KIND: NodeKind;
    fn business_id(&self) -> &str;
    fn to_row(&self) -> Vec<(String, Value)>;          // present fields only
    fn from_lookup(lookup: &dyn ValueLookup) -> Result<Self, Error>;
}
pub trait ValueLookup { fn get(&self, key: &str) -> Option<Value>; }

// Open (indexes ensured inside)
Database::open_mapped(path: &Path) -> Result<Database, Error>
Database::open_memory(name: &str)  -> Result<Database, Error>
// Database: Clone (Arc inside) + Send + Sync

// Scope entry
database.read(|r: &ReadScope|   -> Result<T, Error>)   // read-only txn
database.write(|w: &WriteScope| -> Result<T, Error>)   // Ok commit / Err rollback
// WriteScope has every ReadScope method plus the writes below.

// Read primitives
r.resolve(kind, business_id) -> Result<Option<NodeId>, Error>
r.read_node<T: Row>(id)           -> Result<Option<T>, Error>
r.read_nodes<T: Row>(&[NodeId])   -> Result<Vec<T>, Error>       // strict
r.all_nodes(kind)                 -> Result<Vec<NodeId>, Error>
r.scan_nodes(kind, Option<&Condition>, Order, offset, limit)
                                  -> Result<Vec<NodeId>, Error>
r.count_nodes(kind, Option<&Condition>) -> Result<u64, Error>
r.outgoing(id, EdgeKind) / r.incoming(id, EdgeKind)
                                  -> Result<Vec<NodeId>, Error>
r.count_outgoing(id, EdgeKind) / r.count_incoming(id, EdgeKind)
                                  -> Result<u64, Error>
r.read_value<T: TryFrom<Value>>(id, key) -> Result<Option<T>, Error>

pub enum Condition {
    KeyEquals(String, Value),
    KeyGreaterThan(String, Value),
    KeyNotExists(String),
    All(Vec<Condition>),
}
pub struct Order { pub key: String, pub ascending: bool }

// Write primitives
w.insert_node<T: Row>(&row)       -> Result<NodeId, Error>       // upsert, full-row replace
w.insert_nodes<T: Row>(&[row])    -> Result<Vec<NodeId>, Error>  // single-query bulk
w.insert_edge(from, EdgeKind, to) -> Result<(), Error>           // idempotent
w.remove_edge(from, EdgeKind, to) -> Result<(), Error>
w.remove(&[NodeId])               -> Result<(), Error>           // cascades edges
w.set_key(id, key, Value)         -> Result<(), Error>
w.clear_key(id, key)              -> Result<(), Error>

// Deliberately NOT exposed: query builder, raw id construction, any agdb
// type, partial-field upsert, rooted search.
```

Coverage check: all 19 current repository query shapes expressible — alias
resolution, single/batch reads, alive-filtered scans, one-hop navigation
(distance semantics handled internally), counted pagination, conditional bulk
removal as `remove(scan result)`, bulk seeding.

## 4. Slice breakdown

| # | Goal | Files | Red | Green | Exit |
| --- | --- | --- | --- | --- | --- |
| 1 | `database` crate skeleton implementing §3.1 with full unit tests, additive | `code/Cargo.toml`, `code/database/{Cargo.toml,src/*}`, `test/unit/database/tests.rs` | new tests fail (crate absent) | tests pass; back untouched | `cargo test -j 1 -p database` |
| 2 | Migrate user/role/seed/authorization repositories onto `database` | `repository/{user*,role*,seed*,authorization*}.rs` (+subdirs), `schema.rs` (Row impls) | those modules no longer import `graph` (compile-enforced) | modules build against new API; back tests green | `cargo test -j 1 -p nail_back` |
| 3 | Migrate remaining repositories; delete substrate | `repository/{article,version,comment,tag,delete,transfer,search/**}.rs`, delete `graph*/`, substrate parts of `schema*`, `logic/version.rs` hosts semver helper | grep proves no `graph::` imports remain | back builds without substrate; all tests green | `cargo test -j 1 -p nail_back` + clippy |
| 4 | Extract `authorizer` crate | `code/authorizer/**` (cedar.rs, cedar/, build.rs), `back` imports it | `cargo tree -p nail_back` shows no direct cedar dep | CI green | `cargo test -j 1 -p authorizer && cargo test -j 1 -p nail_back` |
| 5 | Extract `searcher` crate | `code/searcher/**` (seekstorm, search repo/index), `back` imports it | `cargo tree` shows no direct seekstorm dep | CI green; final gate | full CI |

## 5. Open unknowns

None blocking — resolved by research report
(`document/database-crate-research.md`, source + 34 probes):
transaction rollback (C-probes), bulk insert shapes (P-A/P-B), order_by
pagination (P-C), conditional removal (P-D), search-insert (P-E), storage
tradeoffs (B-probes), trap verification (T0–T4).

## 6. Verification plan

Per slice: correctness (unit tests normal+edge), behavior change (none
expected — endpoint-level back tests are the net), time complexity (direct
reads O(ids) vs old O(scan); noted where touched), space complexity (N/A —
same storage engine), performance (B1 evidence justifies direct reads; no new
benchmarks unless a slice touches a hot path structurally).

Dimensions tracked in this file's change log per slice; unevidenced dimension
blocks the gate.

## 7. Risks

- **Slice 2/3 churn across many files** — mitigated by keeping row structs and
  function signatures stable; mechanical replacement only.
- **Hidden substrate callers** — mitigated by compiler: delete substrate last
  (slice 3), let rustc enumerate stragglers.
- **Data compatibility** — no format change: same keys, aliases, indexes;
  mapped storage opened identically. Verified against current `graph.rs::open`
  during slice 1 test authoring.
- **CI-only gate latency** — local smoke `-j 1` per run.md before each push.
- Rollback: each slice is one commit; revert the commit to recover.

## 8. Constraints

- Never touch `target/`, `dist/`, `data/`, `log/`.
- No `unwrap`/`expect` anywhere including new crate.
- English only; CRUD verbs; file ≤ 512 lines; no dead code.
- No hand-edited `Cargo.lock`; deps added alphabetically via `cargo add`.
- One commit per slice; `[skip ci]` only for docs.
- Do not rename HTTP-visible identifiers; no API changes.

## 9. Questions

None — user adopted the design (node vocabulary, §3.1 contract) and mandated
best-practice redesign of the substrate rather than a file move.

## Change log

- Created at workflow §5 after research gate (report committed 0fe1a06).
- §3 rewritten with adopted final API contract (§3.1): node vocabulary,
  business_id/NodeId split, Condition/Order pushdown, full-row-replace upsert.
