# Handoff

## Task VII-routes: singularize 4 public comment routes (A1–A4)

**Owner**: uD7wXk
**Exec doc**: `document/exec/R4sq_route-singularize.md`
**Status**: DONE — slice 1 committed; full back split 576/576 green, fmt clean,
clippy 0 warnings, frontend `cargo check` clean.

### Slices

- 1. Coordinated 4-file change (router.rs, back http comment.rs,
  infrastructure cedar.rs, front request/comment.rs). COMPLETE.

### Commits

- `9548d0f` refactor(routes): singularize comment routes A1-A4

### Decisions / notes

- Route constants renamed to match new singular paths
  (`ROUTE_VERSION_ID_COMMENT_CREATE` / `ROUTE_COMMENT_ID_REPLY_CREATE` /
  `ROUTE_VERSION_ID_COMMENT_READ` / `ROUTE_COMMENT_ID_REPLY_READ`);
  grep-confirmed no other references.
- `test/unit/back/infrastructure/cedar.rs` route-constant literal assertion
  updated in the same slice.
- Red phase: 22 http_comment tests failed with 404 (route not found) on the
  new paths before implementation; a red-phase bug (A3 literals kept
  `/comments/` prefix) was caught by the failing green run and fixed before
  the gate.
- Frontend `cargo +nightly check` clean (fast, cached); `test/unit/front/
  request/url/tests.rs:35` uses `"comments"` only as sample data in a generic
  url-builder test — not a route, left unchanged.

### Open questions

- None.