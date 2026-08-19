# Handoff

## Task VIa: backend LOGIC layer low-risk cleanups (K8pl)

**Owner**: L9sPvT
**Exec doc**: `document/exec/K8pl_logic-layer-cleanups.md`
**Status**: COMPLETE — all 5 slices committed and gated green

### Stages

- A. ✅ Pagination unification — `logic/pagination.rs` owns offset math + usize
  guards + skip/take + has_next via `page_offset` + `paginate`; all 7 inlined
  blocks rewired (user, role, tag, search → `paginate`; comment x2, version →
  `page_offset`). `clamp_page_limit` untouched.
- B. ✅ Token lifecycle helper — `logic/session.rs` now owns
  `hash_token(raw, invalid)` (normalize → hash, per-flow invalid message) and
  `hash_canonical_token` (hash-only). All 12 sites rewired (user x3, session
  x3, email x4, download x2); the 5 unreachable per-site hash-failure strings
  consolidated to one `"failed to hash token: {error}"` (Q1).
- C. ✅ Content-hash dedup helper — `logic/version::reject_duplicate_content_hash`
  extracted; article.rs private copy deleted; both call sites share it.
- D. ✅ Empty dir deletion — the 6 placeholder dirs
  (`logic/{authenticate,challenge,email,error,pow,session}/`) and root
  `logic/.gitkeep` removed (Q3).
- E. ✅ `repository/user.rs` off the 3 `_sync` helpers — migrated to generic
  `find_by_index` / `resolve_node_id` / `read_node`; the 3 `_sync` functions
  deleted from `repository/graph.rs`; 3 test call sites
  (repository/article.rs, repository/delete.rs, logic/delete_verify.rs)
  migrated to the generic variants.

### Decisions / deviations

- Probe 003 surfaced a real finding: the legacy has_next formulas
  (`page < total.div_ceil(limit)` vs search.rs `len > offset + limit`) disagree
  at page == 0 (unreachable in production — `clamp_page_limit` clamps to >= 1;
  only direct logic-layer test calls hit it). Helper unifies on div_ceil;
  search.rs page-0 now consistent with user/role/tag. No existing test asserted
  the old page-0 value. Documented in exec doc Change log.
- Probe 003 deleted after slice 1 (equivalence covered by permanent
  `paginate_matches_the_legacy_offset_form_has_next` + slice tests); harness.rs
  line removed.
- Q1 (hash-failure message consolidation), Q2 (eager LogicError), Q3 (root
  `logic/.gitkeep`) all APPROVED by orchestrator.
- Final full `cargo test` run was SIGKILLed (OOM loading all tests in one
  binary); every group ran green individually — see Final gate below.

### Code changes (one commit per slice)

- `401366f` docs: exec + handoff for Task VIa
- `07f2d4f` SLICE1 Stage A pagination (pagination.rs + 6 rewire files +
  test/unit/back/logic/pagination_verify.rs)
- `58956df` SLICE2 Stage B token lifecycle (session/user/email/download.rs +
  4 new session tests)
- `4aed1ef` SLICE3 Stage C content-hash dedup (version/article.rs + 2 new
  version tests)
- `85076c0` SLICE4 Stage E `_sync` helper removal (repository/user.rs,
  repository/graph.rs + 3 test files)
- `a633402` SLICE5 Stage D placeholder dir removal (7 `.gitkeep` files)

### Final gate

- `cargo fmt --check`: OK
- `cargo +nightly clippy -- -D warnings`: 0 warnings (excluding dependency
  future-incompat noise)
- Tests (split per module, per README §10): repository_ 108 ✓, logic_ 272 ✓,
  http_ 122 ✓, infrastructure_ 43 ✓, configuration_ 11 ✓ → **556/556**
  (baseline 543 + 13 new permanent tests)

### Open questions

- None. Q1–Q3 resolved (approved).