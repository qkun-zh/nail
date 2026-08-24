# Cedar Target-Design Simulation (cedr)

## 1. Requirement

`R₀` (from ced1): prove, with the real engine, that (a) the current mechanism
fails under official validation semantics, (b) a compliant target design
reproduces the pinned behavioral contract exactly, and (c) dynamic permission
management survives.

## 2. Method

Standalone lab crate outside the repo (`/tmp/opencode/cedarlab`, own lockfile;
repo untouched) linking **cedar-policy 4.12.0** — the exact version pinned by
`code/authorizer/Cargo.toml`. Artifacts: `cedr_target_schema.draft.cedar`,
`cedr_target_policy.draft.cedar` (copies in this directory), full transcript
`simulation_full.log` reproduced in §5. Every experiment is reproducible via
`cargo run` in the lab.

## 3. Experiments

### E1 — Current design under schema-conformant validation
Reproduces `build_principal`/`build_resource` fabrication (Role parents =
Action UIDs) and calls `Entities::from_entities(.., Some(&schema))`.
Result: **rejected** ("entity does not conform to the schema") — confirms F1:
with validation on, production entities are illegal. Control run with `None`
everywhere: owner `Article::Update` → Allow (today's silent coexistence).
Also shows request validation alone catches `Create`-on-`Article`
("resource type `Article` is not valid for …").

### E2 — Request::new(None) vs Some(&schema)
| input | None | Some(&schema) |
|---|---|---|
| action `Action::"Bogus::TotallyUnknown"` | builds; evaluates Deny | Err: UndeclaredAction |
| `Article::Create` on `Article::"a1"` | builds; evaluates Deny silently | Err: invalid resource type |
Confirms F4: today malformed inputs are indistinguishable from denials.

### E3 — Target design decision matrix
Policy set = static draft + 39 templates
(`permit(principal in ?principal, action == Action::"<A>", resource);`) linked
per seeded grants (admin=39, member=9), then **strictly validated as a whole**
(`Validator::validate(ValidationMode::Strict)` over templates+links+static —
passes). Requests built with `Some(&schema)`; entity stores built with
`Some(&schema)`. 42 cases covering every invariant class:

- creation/global ops anchored on `Application::"nail"` (member Allow,
  outsider Deny, anonymous registration Allow)
- article/version/comment read-update-delete-transfer matrix for
  {owner-no-roles, member, admin, outsider, user_zero(admin+recycler)}
- comment authorship vs article ownership distinction (reply chains through
  `Comment in [Comment]` membership)
- user self-service vs admin-only reads; tag create/read/apply/unapply vs
  update/delete split; role CRUD incl. forbid #6 (nobody revokes from admin)
- malformed-input rejection now surfaces as errors

**Result: 42/42 PASS** (`simulation_full.log` §E3).

### E3b — Soft-delete visibility as compound authorization
First prototype used a static `deleted`-attribute forbid guard; it FAILED 3
matrix rows: exempting `Read` exposed deleted content to everyone; not
exempting it blocked holder inspection — no single-query guard reproduces
hide-from-members / show-to-holders. Falsified and removed.
Compound form (leg 1 = permission query; leg 2 = `Undelete::Soft` query when
flagged; composed by app): **7/7 PASS**, matching
`require_entity_readable`/HTTP tests exactly, including member-owner restore
→ Deny and admin restore → Allow.

### E4 — Dynamic grant parity + guardrail integrity
- link `editor × Article::Delete::Transfer` at runtime → editor transfer Allow;
- rebuild without link (revoke) → Deny;
- editor who also holds `recycler`: transfer **Deny** — forbid #5 beats the new
  template-linked permit;
- unknown action at request time → UndeclaredAction error.
Proves API-level grant/revoke capability is preserved with guardrails intact.

## 4. Transcript (verbatim, abridged headers)

```
=== E1 === strict policy validation passes: true
E1 RESULT: entities REJECTED with Some(schema): entity does not conform to the schema
E1 control (None everywhere): Article::Update by owner -> Allow
E1 request validation catches Create-on-Article: Some("resource type `Article` is not valid ...")
=== E2 === bogus action, None: built ok = true / Some: UndeclaredAction
Create-on-Article, None: evaluates silently to Deny / Some: invalid resource type
=== E3 === 49 initial cases → after guard falsification & rework: 42 pass, 0 fail
PASS member creates article / outsider cannot create / anonymous registration open
PASS member reads stranger article / outsider cannot read / owner reads own
PASS member cannot update stranger / owner updates own / owner cannot hard-delete own
PASS admin hard-deletes stranger article / owner transfers own
PASS user_zero (admin+recycler) cannot transfer / plain admin transfers
PASS owner creates version on own article / member cannot create version elsewhere
PASS member reads version / owner updates version note / member cannot hard-delete version
PASS member comments (application anchor) / comment author edits own
PASS article owner cannot edit stranger comment / reply author edits own reply
PASS self read / member cannot read other / admin reads other / self deletes softly
PASS admin hard-deletes user / tag split (create/read/apply/unapply Allow; update Deny)
PASS role split (admin Create/Read/Grant Allow; member Deny; revoke-from-admin Deny)
=== E3b === 7/7 PASS (compound visibility parity incl. restore legs)
=== E4 === dynamic grant Allow / revoke Deny / recycler-forbid Deny / UndeclaredAction
```

## 5. Findings

1. Target design is strictly valid end-to-end (policy set incl. links, requests,
   entities) — the compliance goal is achievable without behavior drift.
2. Seeded-role semantics reproduce byte-identically at engine level (matrix).
3. Visibility must remain compound (documented pattern); do not add a guard forbid.
4. Template-linked policies are the sanctioned dynamic-grant mechanism; link /
   unlink map 1:1 onto Role::Grant / Role::Revoke.
5. With `Some(&schema)` everywhere, malformed authorization inputs become loud
   errors — an intentional, testable tightening.

## 6. Impact

Unblocks planning (`ced3`). No open engine-level unknowns remain; remaining
uncertainty is integration-level (hot-reload wiring, migration ordering) and is
flagged there.
