# Handoff

## Task I: PoW Header Transport — x-pow middleware + standalone pow crate

**Owner**: K3v9n2
**Exec doc**: `document/exec/ACD3_pow-header-middleware-crate.md`
**Status**: Slices 1–4 complete; final gate in progress (back re-test deferred on resource)

### Stages

A. ✅ Slice 1 — standalone `pow` crate (`code/pow/`), stateless, no payload field; 11 tests green
B. ✅ Slice 2 — `nail_common` swaps to pow crate; request types lose PoW (payload → body/query)
C. ✅ Slice 3 — `x-pow` middleware (`code/back/src/interface/pow_layer.rs`), all routes except `POST /challenges`; handlers/logic de-PoW'd; 581 back tests green
D. ✅ Slice 4 — frontend auto-proves a challenge and attaches `x-pow` to every request (incl. PDF download); business data moved from PoW payload into body/query
E. 🔄 Final gate — fmt clean; pow 11, common 105, front 81 tests green; back re-test deferred

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

### Remaining risks / pending

- **Back re-test deferred**: `cargo +nightly test -j 1 -p nail_back` OOM'd WSL twice (rust-analyzer/zed contend memory). Back is untouched since `1644697` where 581 tests were green; needs one clean re-run when machine is quiet to close the final gate.
- Exec doc to be deleted once gate closes (workflow §9).