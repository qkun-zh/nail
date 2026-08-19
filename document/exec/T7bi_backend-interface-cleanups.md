# Exec — Task VII-backend: backend INTERFACE/INFRASTRUCTURE cleanups

**Task**: VII-backend (REFACTOR_PLAN.md §3). **Owner**: Qm4Zt8.
**Scope root**: `code/back/src/interface/**`, `code/back/src/infrastructure/**`,
`test/unit/back/http/**`, `test/unit/back/infrastructure/**`,
`test/unit/back/harness.rs` (additive module lines only).

## 1. Requirement

Behavior-preserving cleanups, back crate only; no HTTP status change, no
public route path change, no `{code,data,message}` payload change:

- **A** — `AppPaged` extractor reads `page`/`limit` (Option<u64>), applies
  `logic::pagination::clamp_page_limit` with `config.server.search_page_size` /
  `max_search_pages`, yields `(page, limit)`. Replaces the 6 identical clamp
  blocks (user, comment x2, version, role, tag); the 5 duplicated
  `{page,limit}` param structs are deleted.
- **B** — multipart helpers `read_text_field`, `stream_pdf_field`,
  `map_multipart_error` move out of `interface/article.rs` into a new
  `interface/multipart.rs`; the duplicated field-scan loops in
  `create_article` / `create_version` unify into one helper with per-endpoint
  accepted-field tables.
- **C** — side-effecting tracing removed from `From<LogicError> for ApiError`;
  logging moves into an explicit `ApiError::from_logic` constructor; the 3
  construction styles (LogicError-then-convert / direct builder / struct
  literal) standardize on constructors only. `{code,data,message}` byte-identical.
- **D** — body-limit formula moves into `infrastructure/config/server.rs` as a
  named accessor; monster const renamed
  `ROUTE_ARTICLE_ID_VERSION_VERSION_ID_CONTENT_READ` →
  `ROUTE_ARTICLE_ID_VERSION_ID_CONTENT_READ`; `{role_id}` → `{id}` in the 3
  role route consts (URL values unchanged; placeholder key is internal axum
  syntax).
- **E** — duplicated `session-token` header read unified: shared
  `read_session_token(parts) -> Option<String>` in `principal.rs`; `Principal`
  (required) and `token.rs` (optional) both use it.

Acceptance: 556/556 back tests (per-module runs), fmt clean, clippy 0 warnings
per slice; one commit per slice.

## 2. Scope

In: `interface/{extractor,envelope,principal,token,router,user,comment,
version,role,tag,article,content,session}.rs`, new `interface/multipart.rs`,
`interface.rs` (one `pub mod` line), `infrastructure/config/server.rs`,
tests listed above, `harness.rs` additive lines (mine only).
Out: `logic/**`, `repository/**` (STOP + report if needed), `code/front/**`,
`code/common/**`, public route paths (A1–A4 are a separate task), other
agents' files/docs.

## 3. Design decisions

### Stage A — `AppPaged` in extractor.rs (non-generic; deviation, see Q1)

```rust
pub struct AppPaged(pub (u64, u64));
impl FromRequestParts<AppState> for AppPaged {
    // Query::<PageLimitParams>::from_request_parts → map_err("invalid query parameters")
    // clamp_page_limit(params.page, params.limit, state.config.server.search_page_size,
    //                  state.config.server.max_search_pages) → map_err(ApiError::from_logic)
}
```
Private `#[derive(Deserialize)] struct PageLimitParams { page: Option<u64>, limit: Option<u64> }`.
Query deserialization errors keep the exact `"invalid query parameters"` 400
(same axum Query path as today's AppQuery). `clamp_page_limit` unchanged
(pagination.rs untouched); its BadRequest carries `"page exceeds max search
pages"`. State-bound like `Principal` (needs config). Handlers:
`AppPaged((page, limit)): AppPaged` after `Principal` (same position as the
removed AppQuery; extractor order preserved).
Non-generic (no PhantomData) — no per-endpoint marker type exists after the
param-struct deletion; see Q1.

### Stage B — new module `interface/multipart.rs`

Move the 3 helpers verbatim. Add one field-collection helper:

```rust
pub(crate) enum MultipartField { Pdf(PdfUpload), Text(String) }
pub(crate) async fn collect_fields(
    state: &AppState, multipart: axum::extract::Multipart,
    pdf_fields: &[&str], text_fields: &[&str],
) -> Result<HashMap<String, MultipartField>, ApiError>
```
Per-field behavior identical to today: unknown → `drop(field)` (never read);
pdf-table names → `stream_pdf_field`; text-table names → `read_text_field`;
streaming happens during collection in arrival order; duplicates last-wins
(HashMap insert). Error propagation identical (next_field / bytes / chunk /
guard errors). Required-field checks stay in the handlers, in today's order
(article: title, summary, tags, version, note, file; version: version, note,
file) — first-missing error identical. Field tables stay per-endpoint:
article `pdf: ["file"], text: ["title","summary","tags","version","note"]`;
version `pdf: ["file"], text: ["version","note"]`.
`interface.rs`: add `pub mod multipart;` (16 .rs files in dir — at the
16-file boundary, no deepening required).

### Stage C — envelope.rs canonical constructors

- Delete `impl From<LogicError> for ApiError`.
- `pub fn from_logic(error: LogicError) -> Self` — today's match (Internal →
  `tracing::error!`, Forbidden → `tracing::warn!`) then `error.into_pair()`.
- `pub fn with_status(status: StatusCode, message: impl Into<String>) -> Self`
  — replaces the struct literal in `map_multipart_error`.
- `bad_request`/`unauthorized` stay (direct constructors = canonical).
- Every interface `?`-on-LogicError site and every `ApiError::from(LogicError::…)`
  becomes `.map_err(ApiError::from_logic)?` / `ApiError::from_logic(…)`.
  Files: principal, session, token, user, article, version, comment, role,
  tag, content, multipart. No status/message delta: `into_pair` unchanged.

### Stage D — config accessor + renames

- `ServerConfig::max_request_body_bytes() -> u64`:
  `max_pdf_size_bytes.saturating_add(max_text_field_bytes.saturating_mul(5))
  .saturating_add(64 * 1024)` — formula verbatim from router.rs; router calls it.
- Const rename in router.rs (single use, line 28/90).
- `{role_id}` → `{id}` in 3 role route consts. Handler path extraction needs
  NO change: axum `Path<String>` deserializes `url_params[0]` regardless of
  key name (source: axum-0.8.9 `extract/path/de.rs` `parse_single_value!` +
  `deserialize_str`). Evidence: source + probe 003; verified in gate by the
  existing http role tests (they hit `/role/{id}/read` etc. with real ids).

### Stage E — shared session-token read

- `principal.rs`: `pub fn read_session_token(parts: &Parts) -> Option<String>`
  (header get + `to_str().ok()` + to_string). `Principal` uses it and keeps
  its 401 `"missing session-token header"`.
- `token.rs`: handler param `headers: HeaderMap` → `parts: axum::http::request::Parts`
  (extractable: axum-core-0.5.6 `extract/request_parts.rs:141` — source
  evidence); `create_token` passes `read_session_token(&parts)`.
  Optionality preserved (None when missing).

## 4. Slice breakdown

| # | Stage | Red | Green | Exit test |
|---|---|---|---|---|
| 0 | docs | — | exec + handoff files | — |
| 1 | A | `http/extractor.rs` AppPaged unit tests (compile-fail: type missing) | extractor + 6 handlers rewired, 5 structs deleted, probe 003 deleted | fmt, clippy, http_ + logic_ groups |
| 2 | B | `http/multipart.rs` collect_fields tests (compile-fail) | multipart.rs created, article/version rewired | fmt, clippy, http_ group |
| 3 | C | `http/envelope.rs` from_logic tests (compile-fail) | envelope.rs + ~15 call sites rewired | fmt, clippy, http_ + logic_ + infrastructure_ groups |
| 4 | D | N/A — internal rewrite; existing http/config tests + `create_article_reports_body_too_large` are the proof | accessor + renames | fmt, clippy, http_ + configuration_ groups |
| 5 | E | `http/extractor.rs` read_session_token tests (compile-fail) | principal.rs helper, token.rs rewired | fmt, clippy, http_ group |
| 6 | final gate | — | full 556-test split + fmt + clippy | all green |

## 5. Open unknowns

- U1 (source): `Path<String>` key-agnostic — axum-0.8.9 `extract/path/de.rs`
  (`url_params[0]`, key ignored for String/primitive targets).
- U2 (source): `Parts` extractable in handlers — axum-core-0.5.6
  `extract/request_parts.rs:141` `impl<S> FromRequestParts<S> for Parts`.
- U3 (probe 003): end-to-end confirmation that a `{id}`-keyed role route
  matches `/role/<value>/read` and that a handler can extract `Parts`
  (compile + runtime through a real mini-router). Delete after adoption gate.
- U4 (grep): 5 `{page,limit}` param structs referenced only by their
  handlers — confirmed; no test or other module references them.

## 6. Verification plan

- Correctness: per-slice module test groups; full 556 final gate (per-module
  runs; single binary OOMs/SIGKILLs on this box — never one big run).
- Behavior change: none by construction; every payload/status/message path
  pinned by existing http tests (pagination errors, multipart errors, body
  limit 413, role CRUD, token flows, missing session header 401).
- Time/space: identical work per request (same query parse, same clamp, same
  field streaming); one `HashMap` alloc per multipart request (negligible).
- Performance: no delta (same DB/IO work).

## 7. Risks

- Concurrent agents (frontend): I never touch `code/front/**` / `code/common/**`;
  stage only my own paths explicitly; re-read files before each slice.
- axum `Path<String>` semantics if my source reading is wrong → probe 003
  contradicts → back to research (workflow loop-back).
- OOM: check `uptime` before every build; run tests split per module, serially.
- Rollback: one commit per slice; revert restores prior state.

## 8. Constraints

- No status/message/payload/route-value changes. No `unwrap`/`expect`/new
  panics. No comments restating code. English only. Never hand-edit
  Cargo.lock; never touch target/dist/data/log. One commit per slice, clean
  tree. Probe 003 unique-numbered, deleted after gate. Never `git add -A`/
  `git add .` — stage only own paths.

## 9. Questions

- Q1: Task text says `AppPaged<T>`; I plan a non-generic `AppPaged` (no
  per-endpoint marker type exists once the param structs are deleted; a
  PhantomData generic would add `AppPaged<()>` noise at 6 call sites). OK?

## Change log

- 2026-08-19: Adoption gate APPROVED by orchestrator. Q1 (non-generic
  `AppPaged`) APPROVED — generic would add noise; recorded. Probe 003 deleted
  after Stage D3 verification (slice 4).