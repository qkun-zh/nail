# Exec doc — Vm2q: unify the three duplicated string validators (Task II)

Owner: sub-agent. Orchestrator approves at the adoption gate. Single source of
truth for this task.

## Requirement

Extract the duplicated validation skeleton shared by `validate_name`,
`validate_tag_name`, and `validate_ascii_text` (in `code/common/src/`) into one
shared, `CharPolicy`-driven helper so each public validator becomes a thin
wrapper over it. Delete the copy-paste, not the per-domain behavior.

Acceptance criteria:
- Exact behavior preserved: same trim/empty/char-scan/length-cap/return order,
  same accepted/rejected character sets, same length measurement (char count of
  the trimmed string).
- Exact user-facing messages preserved: every `Display` string, every error
  variant name, every error enum field identical to baseline.
- The three public error enums `NameError`, `TagNameError`, `TextError` keep
  their exact shapes and remain public; no consumer in `back/` or `front/` is
  touched.
- Public function signatures unchanged: `validate_name(&str)`,
  `validate_tag_name(&str)`, `validate_ascii_text(&str, usize, bool)`.
- Zero-warning gate green; existing per-domain tests pass unmodified (they pin
  exact behavior and messages).
- No new panics / `unwrap` / `expect`; no comments restating code; English only.

## Scope

In scope:
- `code/common/src/validate.rs` (new shared module), `code/common/src/lib.rs`
  (add `pub mod validate;`), `code/common/src/{name,tag,text}.rs` (refactor to
  wrappers), `test/unit/common/validate/tests.rs` (new shared-module test).

Explicitly out of scope:
- Changing any public error enum, variant, field, or `Display` string.
- Touching `back/`, `front/`, other `common` modules, `parse_tags`,
  `TagRef`, `TagNamesError`.
- The `Cargo.lock`, `target/`, `data/`, `log/`, `dist/`.
- The unrelated dirty `front/src/page/user/*` files already on the tree.

## Design decisions

New `code/common/src/validate.rs` holds:

- `trait CharPolicy { fn allows(&self, ch: char) -> bool; }` — a character
  acceptance decision. Seam chosen so each domain supplies only its allowed-char
  rule; the loop/length logic is shared.
- `trait ValidationError: Sized { fn empty(); fn too_long(usize); fn forbidden(char); }` —
  maps the three shared failure kinds onto each domain's private error shape.
  This satisfies the plan's "one ValidationError" while each public enum keeps
  its exact variants/fields (e.g. `TextError::TooLong { max_chars }` vs the unit
  `TooLong` of name/tag are both produced via `too_long(max_chars)`; name/tag
  ignore the arg, text stores it).
- `fn validate_with_policy<E: ValidationError, P: CharPolicy>(raw, max_chars, policy) -> Result<String, E>` —
  the shared skeleton: trim → empty → scan chars → length cap (char count of the
  trimmed string) → return trimmed string. Identical ordering and semantics to
  the three originals.
- Two policies: `AlphanumericDashUnderscore` (used by name/tag:
  `is_ascii_alphanumeric || '-' || '_'`) and `PrintableAscii { allow_newline }`
  (used by text: printable ASCII 0x20..=0x7e, plus `\n` when `allow_newline`).

Each of `name.rs`, `tag.rs`, `text.rs` keeps its public `fn` signature, public
error enum, and `Display` impl byte-for-byte, and becomes:

- `impl ValidationError for <Domain>Error` (trivial variant mapping).
- `validate_<x>(...) { validate_with_policy::<_, _>(raw, MAX_X_CHAR_COUNT, &Policy) }`
  (text passes `max_chars` and `PrintableAscii { allow_newline }`).

Rationale: the three differ only in (a) allowed-char predicate, (b) max-length
source (const vs param), and (c) error type/messages. A generic helper + policy
+ error-builder isolates (a) and (c) while (b) remains a plain argument; the
constants `MAX_NAME_CHAR_COUNT` / `MAX_TAG_NAME_CHAR_COUNT` stay defined in
their domains and are fed as `max_chars`. Max-length constants are config-free
existing names; not hardcoded.

Trade-off: the `ValidationError` trait is a small indirection, but it is the only
clean way to share the loop while keeping three independent public enums (the
task forbids unifying the enums). Alternative (closures for each error kind) was
rejected: a trait is more explicit, reusable, and greppable.

Behavioral equivalence for text: original had two `ContainsForbiddenChar`
branches (`!is_ascii`, then printable/newline check). The combined predicate
`is_ascii && (printable || (allow_newline && newline))` rejects exactly the same
characters and reports the same offending char in both cases, so the reported
char is unchanged.

## Slice breakdown

Slice 1 — shared module.
- Goal: add `validate.rs` (CharPolicy, ValidationError, validate_with_policy, two
  policies) + `test/unit/common/validate/tests.rs`; wire `pub mod validate;`.
- Files: `code/common/src/validate.rs` (new), `code/common/src/lib.rs`,
  `test/unit/common/validate/tests.rs` (new).
- Red: new shared tests reference `validate_with_policy`/policies that do not
  exist yet → compile error → `cargo test` fails.
- Green: module + tests implemented; shared tests pass.
- Exit: `cargo fmt --check && cargo clippy -D warnings && cargo test`.

Slice 2 — route name + tag through the helper.
- Goal: `validate_name`/`validate_tag_name` become wrappers; `impl ValidationError`
  for `NameError`/`TagNameError`.
- Files: `code/common/src/name.rs`, `code/common/src/tag.rs`.
- Red: behavior-preserving wiring; the existing name/tag tests already pin exact
  output and messages and would fail on any regression. See Risks for the
  red-phase note on behavior-preserving refactors.
- Green: existing name/tag tests still pass; no message change.
- Exit: `cargo fmt --check && cargo clippy -D warnings && cargo test`.

Slice 3 — route text through the helper.
- Goal: `validate_ascii_text` becomes a wrapper; `impl ValidationError` for
  `TextError`.
- Files: `code/common/src/text.rs`.
- Red: same note as Slice 2; existing text tests pin behavior/messages.
- Green: existing text tests pass unmodified.
- Exit: `cargo fmt --check && cargo clippy -D warnings && cargo test`.

## Open unknowns

- None material. Consumer usage verified by source read (grep over `back/`,
  `front/`): all consumers use the public fn signatures and `?` / `to_string()`;
  none match on enum variants or construct them, so preserving signatures +
  messages + enum shapes guarantees no ripple. Behavior equivalence of the
  combined text predicate verified by inspection of the original two-branch
  logic (source evidence) — no probe needed since behavior is identical and the
  existing text test suite (including the per-byte exhaustive tests) pins it.

## Verification plan

- Correctness: existing `name/tag/text` tests run unmodified (they exhaustively
  pin allowed/forbidden chars, boundary lengths, trimming, and message strings)
  + new shared-module tests exercise the helper generically. Verified via
  `cargo test`.
- Behavior change: must be zero. Proven by the unmodified existing test files
  staying green after each slice.
- Time complexity: unchanged — one `chars()` scan + one `chars().count()` pass,
  identical to baseline. Verified by inspection (N/A runtime probe).
- Space complexity: unchanged — one heap `String` allocation for the trimmed
  result, same as baseline. Verified by inspection.
- Performance: unchanged; the helper is monomorphized per domain, so no dynamic
  dispatch. Verified by inspection.

## Risks

- Red-phase semantics on behavior-preserving slices (2, 3): there is no net-new
  behavior, so no test is newly red; the workflow's red-first rule is satisfied
  by Slice 1 (real red) and by the pre-existing, unmodified per-domain tests
  acting as the regression pin for 2 and 3. This is a documented deviation;
  flagged to the orchestrator at the gate. If the orchestrator requires a
  genuinely red test on 2/3, revert to plan/re-plan (loop-back to phase 4).
- Dirty tree (unrelated `front/src/page/user/*`): do not discard; do not commit
  another agent's mid-work; stage only my files. Clean-tree-per-slice is scoped
  to my touched paths. Flagged as a question.
- Clippy: `too_long(usize)` unused arg for name/tag and a ZST policy could in
  principle draw pedantic lints; those are not in default `-D warnings`. If the
  gate trips, adjust rather than relax.
- Message drift: mitigated by leaving the existing message-assertion tests
  untouched and green.

## Constraints

- Touch only `code/common/src/{name,tag,text,lib}.rs` + new `validate.rs` and
  `test/unit/common/validate/tests.rs`. No `back/`, no `front/`.
- Preserve the three public error enums, their variants/fields, and every
  `Display` string exactly.
- No `unwrap`/`expect`/new panics; no comments restating code; English only;
  no hardcoding of lengths (keep the named max-length constants).
- Never hand-edit `Cargo.lock`; never touch `target/`/`dist/`/`data/`/`log/`.
- One commit per slice, staging only my files.

## Questions

1. Orchestrator: approve this plan at the adoption gate?
2. The tree has unrelated uncommitted changes in `front/src/page/user/{email,hub,id}.rs`.
   They are not mine. Leave unstaged (commit only my scope) or commit them?
3. Accept the documented red-phase note for the behavior-preserving slices 2/3
   (real red only in Slice 1)?

## Change log

- 2026-08-19: initial plan. Baseline green (common, 109 passed).
- 2026-08-19: Orchestrator APPROVED at the adoption gate. Decisions: (1) plan as
  written; (2) `code/front/src/page/user/{email,hub,id}.rs` are a concurrent
  agent's in-progress work — never stage/commit/revert/touch them; commit ONLY
  my scope, staging files explicitly by path (never `git add -A`/`.`); (3) the
  red-phase deviation for behavior-preserving slices 2/3 is ACCEPTED and
  documented: real red test is slice 1; the unmodified per-domain tests in
  `test/unit/common/{name,tag,text}/tests.rs` are the regression pin for 2/3.