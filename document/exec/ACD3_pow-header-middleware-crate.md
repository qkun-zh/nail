# Exec Doc: ACD3 — PoW header transport, global middleware, standalone pow crate

## 1. Requirement

Refactor Proof-of-Work (PoW) in the nail project:

1. **Standalone `pow` crate** (`code/pow/`) with three stateless public APIs,
   owning no storage and depending on no nail crate:
   - `issue_challenge(difficulty: u64) -> Challenge`
   - `prove(&Challenge) -> Result<Pow, PowError>`
   - `verify(&Pow, difficulty: u64) -> bool`
   - Types: `Challenge { id: Uuid, difficulty: u64 }`,
     `Pow { challenge: Challenge, solution: String }` — **no payload field**.
2. **PoW carried in one header** `x-pow` (JSON-serialized `Pow`) on every
   request, business data in body/query as normal fields (never read from PoW).
3. **Middleware performs PoW verification** for all routes except
   `POST /challenges` (chicken-and-egg bootstrap endpoint). The middleware
   consumes the issued challenge from the existing moka cache (owned by
   `nail_back`, not the pow crate) and then calls `pow::verify`.
4. Single PoW per request (email-change also uses one PoW; old+new emails are
   business fields, not separate proofs).

**Acceptance criteria**:
- `code/pow/` builds standalone; `nail_common` re-exports or drops its own pow.
- All backend mutation and read routes (all except `POST /challenges`) require
  a valid `x-pow` header; missing/invalid/unissued PoW → 400 envelope.
- `POST /challenges` works without any PoW.
- Frontend attaches `x-pow` to every request helper; business data stays in
  body/query.
- All tests green (common 117, back 577, emailer 26, front wasm harness),
  clippy 0 warnings, fmt clean.

## 2. Scope

**In-scope**:
- New `code/pow/` crate (algorithm migration + simplification).
- `nail_common`: remove `pow` module or re-export from `pow` crate; remove
  PoW fields from request types; remove payload-dependent structs.
- `nail_back`: new middleware, `Pow` extractor, router layering, handler/logic
  cleanup (remove `verify_issued_pow` calls from logic), DELETE query cleanup.
- `nail_front`: HTTP helpers attach `x-pow`; request modules stop embedding
  PoW in body/query; email-change uses one PoW.
- Tests: update all HTTP-level tests, context helper, pow unit tests.

**Out-of-scope**:
- Challenge cache storage (stays in `nail_back` moka `TokenCaches.challenge`).
- Cryptography primitives change (ascon-xof128, pso-vdf stay).
- Proxy/pingap config (no method restrictions).
- Authorization logic.

## 3. Design Decisions

- **pow crate owns no state**: storage/cache lives in `nail_back`; pow is a
  pure algorithm library (issue/prove/verify). Cost of self-managed cache
  (extra moka instance, WASM incompat) rejected by user.
- **No payload in `Pow`**: business data is carried in body/query; PoW binds
  only to a challenge (single-use enforced by cache consume).
- **Single PoW per request**: email-change proof is one PoW; old/new email are
  body fields, not payload.
- **Header name**: `x-pow` (constant in pow crate or back).
- **Middleware applies globally**: router-level `from_fn_with_state` on all
  routes; skip only `POST /challenges` via method+path check.
- **Verify flow in middleware**: consume challenge from cache → missing →
  400; call `pow::verify(&pow, difficulty)` → false → 400; insert `Pow` into
  request extensions; handler uses a `Pow` extractor.
- **DELETE /users/{id}**: `mode` stays in query; PoW in header; delete token
  moves to query/body (business field).
- **frontend**: `post_json`/`patch_json`/`put_json`/`delete_json`/`get_json`
  take `Option<&Pow>`; `prove_pow(payload)` keeps payload param? No — prove
  takes only challenge; payload is business data, so `prove_pow()` issues a
  challenge only (client proves without payload), request helpers attach it.

## 4. Slice Breakdown

### Slice 1: pow crate skeleton + algorithm + unit tests
- **Goal**: `code/pow/` compiles standalone with issue/prove/verify.
- **Files**: new `code/pow/Cargo.toml`, `code/pow/src/lib.rs`,
  `test/unit/pow/tests.rs`, `code/Cargo.toml` (workspace member).
- **Red**: tests in `test/unit/pow/tests.rs` reference crate APIs not yet
  existing → build fails.
- **Green**: `cargo +nightly test -j 1 -p pow` passes.
- **Exit test**: `cargo +nightly test -j 1 -p pow` (run.md flags).

### Slice 2: nail_common swaps to pow crate; request types lose PoW
- **Goal**: `nail_common` uses `pow` crate (remove own pow.rs or re-export);
  request/response structs drop pow fields.
- **Files**: `code/common/Cargo.toml`, `code/common/src/lib.rs`,
  `code/common/src/pow.rs`, `code/common/src/request.rs`,
  `test/unit/common/pow/tests.rs`, `test/unit/common/request/tests.rs`.
- **Red**: after removing pow.rs, `nail_common` build fails (dangling uses).
- **Green**: `nail_common` builds; its tests pass.
- **Exit test**: `cargo +nightly test -j 1 -p nail_common`.

### Slice 3: backend middleware + extractor + router; handlers/logic cleanup
- **Goal**: middleware verifies `x-pow` on all routes except
  `POST /challenges`; logic no longer calls `verify_issued_pow`.
- **Files**: `code/back/src/interface/pow_layer.rs` (new),
  `code/back/src/interface/extractor.rs` (Pow extractor),
  `code/back/src/interface/router.rs`, all interface handlers using PoW
  (`user.rs`, `session.rs`, `token.rs`), logic files (`email.rs`,
  `session.rs`, `user.rs`, `pow.rs`, `challenge.rs`),
  `code/back/src/infrastructure/state.rs` (if needed).
- **Red**: existing HTTP tests send PoW in body/query → middleware requires
  header → tests fail.
- **Green**: HTTP tests updated to header transport pass.
- **Exit test**: `cargo +nightly test -j 1 -p nail_back`.

### Slice 4: frontend attaches x-pow
- **Goal**: frontend request helpers attach `x-pow` header; business data in
  body/query.
- **Files**: `code/front/src/request/http.rs`, `code/front/src/request/*.rs`
  (auth, user, pow), `code/front/src/infrastructure/pow.rs`.
- **Red**: build fails (helper signatures changed).
- **Green**: `cargo +nightly build -p nail_front` passes (host build).
- **Exit test**: `cargo +nightly build -j 1 -p nail_front`.

### Slice 5: final gate + cleanup
- **Goal**: full verification, remove leftover code, update exec docs.
- **Files**: any stragglers found by clippy/fmt; `document/handoff/`.
- **Red**: none (cleanup).
- **Green**: all tests, clippy 0 warnings, fmt clean.
- **Exit test**: per-crate tests as run.md.

## 5. Open Unknowns

- Whether `pow::prove` may be useful without a payload-based challenge for
  frontend proof-of-work — resolved by design (prove takes challenge only).
- Whether `GET /config` should require PoW — yes, all except `/challenges`.
- Frontend host build feasibility — evidence: check how front tests run
  (wasm harness) vs host build (`cargo +nightly build -p nail_front`).
- Exact `x-pow` header name constant location (pow crate or back).

## 6. Verification Plan

| Dimension | Check |
|-----------|-------|
| Correctness | All tests pass; middleware rejects missing/bad PoW; /challenges free |
| Behavior change | PoW transport body/query → header; logic no longer verifies |
| Time complexity | O(1) header extraction + VDF verify unchanged |
| Space complexity | No new allocations beyond one Pow deserialize |
| Performance | VDF verify dominates; unchanged |

## 7. Risks

- Broad test churn (all HTTP tests). Mitigation: slice 3 includes test
  updates in one commit.
- Frontend WASM build; mitigate by host build check and wasm harness run.
- Breaking common API (Pow without payload) — mitigated by slices 1-2 before
  backend/frontend switch.

## 8. Constraints

- Follow `document/run.md` test flags (nightly, cranelift, `-j 1`, one crate
  per invocation).
- No `unwrap`/`expect`; no hand-edited `Cargo.lock`; English code/comments.
- Don't touch `target/`, `dist/`, `data/`, `log/`.
- One commit per slice; never discard work.

## 9. Questions

- None — design confirmed with user.

## Change log

- 2026-08-20: Initial version.