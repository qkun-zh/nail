# By3c — Frontend URL Restructure

## 1. Requirement

Restructure frontend URLs: remove `/public/`, `/private/`, `/admin/` prefixes.
Root page (`/`) links to `/authenticate` and `/search`. User pages under
`/user/:uid`. All article/version/comment routes at top level without prefix.
Remove `RootGate` authentication gate. All pages accessible without auth.

**Acceptance criteria**:
- `cargo test` passes (69/69)
- `cargo clippy` zero warnings
- `cargo fmt` clean
- `trunk build` succeeds
- Every URL in the new design maps to the correct page component

## 2. Scope

**In**: frontend routing, page components, internal link updates, module tree
restructure, removing admin/auth gate
**Out**: backend changes, API changes, new functionality, behavioral changes

## 3. Design Decisions

- Flat route structure: `/article/:aid` not `/user/:uid/article/:aid` (articles
  are public resources)
- Version and comment nested under article (hierarchy reflects data ownership)
- User pages use `:uid` route param (not session) for consistency
- `persist_draft` paths updated to new URL structure with dynamic uid
- `RootGate` and `session_gate.rs` removed entirely (auth redesign deferred)
- `author_gate.rs` retained (still needed for article mutation authorization)
- `admin` module deleted entirely (no admin pages in new design)
- `PublicLayout`, `PrivateLayout`, `AdminLayout` removed (no layout wrappers)

## 4. Slice Breakdown

| Slice | Goal | Files | Exit test |
|-------|------|-------|-----------|
| S1 | Create new page files: `page/user/hub.rs`, rewrite `page/index.rs` | 2 new/rewritten | `cargo test` |
| S2 | Move files + restructure module tree | ~15 moves, `page/mod.rs` rewrite | `cargo test` |
| S3 | Rewrite `router.rs` + update all 35+ internal links | `router.rs` + ~15 pages | `cargo test` + `trunk build` |
| S4 | Remove old modules (`admin`, `session_gate`, old layouts) | ~8 deletes, `page/mod.rs` update | `cargo test` + `trunk build` + `cargo clippy` + `cargo fmt` |
| S5 | Handoff: update `document/handoff/`, delete this exec doc | `document/handoff/` | clean tree |

## 5. Open Unknowns

None — all decisions confirmed by user.

## 6. Verification Plan

| Dimension | Method |
|-----------|--------|
| Correctness | `cargo test` (69 front tests) |
| Behavior change | N/A — pure structural, no behavioral change |
| Time complexity | N/A |
| Space complexity | N/A |
| Performance | `trunk build` succeeds, no new wasm size |

## 7. Risks

- S2+S3 are tightly coupled (file moves + router rewrite must happen together)
  — mitigated by doing them in sequence within one session
- `persist_draft` paths need dynamic uid — must extract uid from params
- Proxy rewrite regex may need update for new URL patterns — check
  `configuration/proxy/locations.toml`

## 8. Constraints

- Do not touch backend code
- Do not change API contracts
- Do not add new functionality
- English only
- No `unwrap`/`expect`
- Zero-warning gate

## 9. Questions

None
