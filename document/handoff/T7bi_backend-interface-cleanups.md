# Handoff

## Task VII: backend INTERFACE/INFRASTRUCTURE cleanups

**Owner**: Qm4Zt8
**Exec doc**: `document/exec/T7bi_backend-interface-cleanups.md`
**Status**: IN PROGRESS — baseline 556/556 green; plan + evidence (probe 003)
ready; awaiting orchestrator adoption gate before slice 1

### Stages (planned, in order)

- A. `AppPaged` extractor in `interface/extractor.rs` (reads page/limit,
  clamps via `logic::pagination::clamp_page_limit` + config) — replaces 6
  clamp blocks in user/comment x2/version/role/tag, deletes 5 `{page,limit}`
  param structs.
- B. Multipart helpers (`read_text_field`, `stream_pdf_field`,
  `map_multipart_error`) move from `interface/article.rs` to new
  `interface/multipart.rs`; field-scan loops of `create_article` /
  `create_version` unify into `collect_fields` with per-endpoint field tables.
- C. Error construction unify: delete `From<LogicError> for ApiError`; add
  `ApiError::from_logic` (owns the tracing) + `ApiError::with_status`;
  ~15 call sites rewired to `.map_err(ApiError::from_logic)`.
- D. `ServerConfig::max_request_body_bytes` accessor (formula verbatim);
  monster const rename → `ROUTE_ARTICLE_ID_VERSION_ID_CONTENT_READ`;
  `{role_id}` → `{id}` in 3 role routes (URL values unchanged).
- E. Shared `read_session_token(parts)` in principal.rs; token.rs switches
  `HeaderMap` → `Parts`; optionality preserved.

### Decisions / deviations

- `AppPaged` non-generic (task text said `AppPaged<T>`; no marker type
  exists after param-struct deletion) — recorded as Q1 in exec doc, pending
  orchestrator answer.
- Probe 003 (path-key agnosticism + `Parts` extraction) written and green;
  will be deleted after the adoption gate, harness line removed.
- New permanent test files: `test/unit/back/http/extractor.rs` (Stage A
  red + Stage E), `test/unit/back/http/multipart.rs` (Stage B),
  `test/unit/back/http/envelope.rs` (Stage C) — each needs one additive
  `harness.rs` line.

### Evidence (exec doc §5)

- U1/U2: axum-0.8.9 `extract/path/de.rs` (`url_params[0]`, key ignored) +
  axum-core-0.5.6 `extract/request_parts.rs:141` (`Parts` FromRequestParts).
- U3: probe 003 — `/role/{id}/read` matches `/role/xyz-123/read` via
  `AppPath<String>`; `Parts` extraction + header read work. 2/2 passed.
- U4: grep — the 5 param structs are referenced only by their handlers.

### Code changes

- None yet (awaiting adoption gate).

### Final gate

- Pending: full 556-test split per module + fmt + clippy 0 warnings.

### Open questions

- Q1 (exec doc §9): non-generic `AppPaged` vs `AppPaged<T>` — awaiting
  orchestrator decision at the adoption gate.