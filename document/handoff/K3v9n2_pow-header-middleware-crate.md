# Handoff

## Task I: PoW Header Transport — x-pow middleware + standalone pow crate

**Owner**: K3v9n2
**Exec doc**: (deleted — task complete)
**Status**: Complete — all slices done, CI green

### Stages

A. ✅ Slice 1 — standalone `pow` crate (`code/pow/`), stateless, no payload field; 11 tests green
B. ✅ Slice 2 — `nail_common` swaps to pow crate; request types lose PoW (payload → body/query)
C. ✅ Slice 3 — `x-pow` middleware (`code/back/src/interface/pow_layer.rs`), all routes except `POST /challenges`; handlers/logic de-PoW'd; 581 back tests green
D. ✅ Slice 4 — frontend auto-proves a challenge and attaches `x-pow` to every request (incl. PDF download); business data moved from PoW payload into body/query
E. ✅ Final gate — CI green (run #32364835515): fmt, clippy, tests (pow/common/back/front), wasm build, audit; workflow switched to stable toolchain + CI-first

### Decisions made

- PoW carried in single `x-pow` header (JSON-serialized `Pow`); business data lives in body/query, never in PoW (no payload field)
- Middleware consumes issued challenge from back's existing moka cache, then calls `pow::verify`; one PoW per request
- `POST /challenges` is the bootstrap exception
- **No Pow extractor** — middleware verifies and discards; handlers never receive PoW (deviation from exec-doc plan)
- `verify_issued_pow` retained in `back/src/logic/pow.rs`, sole caller is the middleware
- `UserDeleteQuery.token` is `Option<String>` (hard delete needs no token)
- Frontend attaches PoW at the request layer (each request fn calls `prove_pow().await?`), pages unchanged

### Code changes

- `code/pow/` — new crate: `issue_challenge` / `prove(&Challenge)` / `verify`, types `Challenge`/`Pow` (no payload)
- `code/common/src/pow.rs` — re-export of pow crate
- `code/common/src/request.rs` — `CreateTokenRequest { purpose, email?, old_email?, new_email? }`, `TokenRequest { token }`, `UserUpdateRequest { name?, old_email_token?, new_email_token? }`, `UserDeleteQuery { mode?, token? }`; removed `LogoutRequest`, `UserDeleteRequest`
- `code/back/src/interface/pow_layer.rs` — `require_pow` middleware + `X_POW_HEADER`; mounted via `from_fn_with_state` in router
- `code/back/src/handlers` + `logic` — de-PoW'd (email/session/user); dead `Configurator` methods removed; `cedar::action_uid` → `#[cfg(test)]`
- `code/front/src/request/http.rs` — `attach_pow` helper + `pow: Option<&Pow>` on all helpers
- `code/front/src/request/*.rs` — each fn calls `prove_pow().await?` then passes `Some(&pow)`; `prove_pow()` no longer takes a payload
- Tests: `test/unit/back/http/pow_layer.rs` (6), front request/auth+user tests rewritten for no-payload request types

### Commits

- `c998d5c` Slice 1 pow crate; `9711eff` ICE-log cleanup
- `1c48167` Slice 2 common swap
- `1644697` Slice 3 middleware + back de-PoW
- `0d478b3` Slice 4 frontend x-pow
- `0466618` CI-first workflow + handoff for pow task
- `c8eb948` workflow.md references CI-first flow
- `535cabd` ci-watch bg mode; run.md documents full CI-first workflow

### Remaining risks / pending

None — task complete, CI green on `main`.