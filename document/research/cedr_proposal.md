# Cedar Refactor Proposal (cedr)

Status: ADOPTED with amendments (user review). No production code changed yet.
Companion: `ced1` investigation, `ced2` simulation, `cedr_target_*.draft.cedar`.

## 0. Adoption rulings (user, binding)

- Ruling 1: grants stay durable in the database; template links derived at
  startup and hot-reloaded (D3 as proposed). APPROVED.
- Ruling 2: DO NOT rename `Virtual` → `Application`. Entity type name stays
  `Virtual`; only container hierarchy is added to the schema (D5 amended).
  Registration permit keeps its current form (open permit, no resource anchor);
  recorded as an accepted best-practice deviation, not a spec violation.
- Ruling 3: NO case-insensitive reservation of required role names. The
  literal-shadowing risk (policies reference exact `Role::"recycler"` /
  `Role::"admin"` literals) is ACCEPTED AS KNOWN RISK and documented here.
- Ruling 4: malformed authorization requests return 400 BadRequest (T4).
  APPROVED — this is the only intentional behavior delta.


## 1. Requirement

`R₁`: Replace all off-spec Cedar mechanics with documented ones while
preserving every pinned behavior. Acceptance: all existing tests pass
unchanged except the three listed in §7 T4; new probes prove validation is on.

### Scope — behavioral contract to preserve (from ced1 Q3, abridged)
Deny-by-default with 403 "you are denied"; owner-bypass action set; member
seed set (9 grants); recycler transfer-forbid incl. user_zero; admin-revoke
forbid; open registration via anonymous principal; user self-service set;
soft-delete 404-hiding (subtree + search) with Undelete-holder exemption;
missing-resource 404s with canonical messages; download-token binding;
session/PoW precedence over authz; envelope mapping.

Out of scope: HTTP surface, client, search internals, PoW/session, storage
engine choice, action-EID cosmetic rename (deferred, O5).

## 2. Design decisions

- **D1 — Kill fabrication.** `build_principal` no longer creates Role/Action
  entities; User entity parents = its role UIDs only. Role entities are built
  solely when a Role is the *resource*. Action entities are never built.
- **D2 — Grants become template-linked policies.** One template per action:
  `permit(principal in ?principal, action == Action::"<A>", resource);`
  (39 templates, generated from schema at startup — never string-assembled
  beyond the fixed id/action substitution mandated by the schema vocabulary).
  One link per `(role, permission)` edge: `link_{role}_{action}`.
  Grant/revoke ⇔ link/unlink (`PolicySet::link`, remove linked policy).
  Proven in ced2 E4.
- **D3 — DB remains durable grant store (O2 default).** Startup derives links
  from existing `RoleGrantPermission` edges; `update_role` mutates edges then
  hot-reloads the in-memory set (`Arc<RwLock<PolicySet>>`). Zero API/client
  change; permission nodes/edges keep their meaning as the source of truth for
  LINKS, while every effective rule is a real, inspectable Cedar policy.
- **D4 — Static policies.** Keep owner bypass (1), self-service (1b), recycler
  forbid (5), admin-revoke forbid (6) verbatim; registration permit (7)
  re-anchored: `resource == Application::"nail"`. Mega-permit (3) deleted.
- **D5 — Schema v2** (`cedr_target_schema.draft.cedar`): `Virtual` →
  `Application`; containers declared (`Version in [Article]`, `Comment in
  [Version, Comment]`); everything else unchanged (39 actions, same appliesTo).
- **D6 — Validation everywhere.** `Entities::from_entities(.., Some(&schema))`,
  `Request::new(.., Some(&schema))`. Malformed inputs → new
  `Error::InvalidRequest` → LogicError::bad_request (O4 default).
- **D7 — Visibility stays compound.** `require_visible_if_soft_deleted` and
  friends unchanged in shape (ced2 E3b falsified the single-query guard).
- **D8 — Anonymous principal materialized.** Requests for pre-auth endpoints
  pass a real entity-less-but-present `User::"anonymous"` parentless entity.
- **D9 — Normalization.** UIDs via `EntityUid::from_type_name_and_id`;
  role-name reservation case-insensitive for required roles (O3); owner
  sentinel removed — empty owner becomes a loud error (unreachable today,
  F9).

## 3. File-level change map

| File | Change |
|---|---|
| `authorizer/cedar/schema.cedar` | D5 rewrite |
| `authorizer/cedar/policy.cedar` | D4 rewrite (static half) |
| `authorizer/src/authorizer.rs` | D1/D2/D6/D8 rework: build templates+links from grant view; RwLock<PolicySet>; `reload_links(&[RoleGrant])`; builders simplified; drop `parse_uid` concat |
| `authorizer/build.rs` | unchanged (still emits PERMISSION_* consts + entity consts; add CEDAR_ENTITY_APPLICATION) |
| `authorizer/src/{lib,principal,resource,error}.rs` | Principal loses `permissions` payload? NO — keeps shape; Resource::Virtual→Application variant rename |
| `server/src/infrastructure/authorizer.rs` | pass-through reload calls; Application resource mapping; comment-owner default removed (D9) |
| `server/src/repository/{role,authorization}.rs` | add `read_all_role_grants()`; seed writes same edges (now interpreted as links) |
| `server/src/logic/{authorize,role}.rs` | authorize_global anchors on Application::"nail"; update_role/delete_role trigger reload; required-role reservation check |
| `server/src/logic/{article,user,tag,comment,version,download,search}.rs` | call sites unchanged except Virtual→Application const swap |
| tests | see §7 |

## 4. Migration & compatibility

- Graph data: **no migration required.** Existing roles/users/grants/edges are
  reinterpreted as link sources at startup. Permission nodes become inert after
  cutover but remain until a later cleanup slice (reversible).
- Restart requirement matches current ops ("Static, requires restart") plus
  hot-reload on admin grant/revoke — strictly more available than today.
- Rollback: revert commits; graph untouched by refactor (no destructive step).

## 5. Verification dimensions

| Dimension | Check |
|---|---|
| Correctness | ced2 matrix re-run inside repo as unit probe; all 42 cases |
| Behavior change | intended deltas ONLY: malformed→400 (T4); everything else byte-equal |
| Complexity | authorizer builds O(roles+grants of actor) entities instead of O(grants)+fabrications; decision-time policy count = static(9) + links(|grants|) — indexed scopes |
| Performance | DB reads/decision unchanged (principal snapshot still required); evaluation cost drops (no fabricated stores); reload O(total grants) on admin ops only |

## 6. Slice plan preview (workflow §6 shape)

1. S1 authorizer core: schema/policy v2 + template/link engine + Some(schema);
   Red: existing fabrication-dependent unit tests fail; Green: lab matrix ported green.
2. S2 infrastructure wrapper: Application mapping, anonymous entity, D9.
3. S3 repository read_all_role_grants + startup derivation wiring.
4. S4 logic layer: hot reload on grant/revoke/delete_role; name reservation.
5. S5 test/probe cleanup (39-count → derived), docs sync, e2e run.
Each slice: one commit, fmt+clippy clean, `-j 1` crate-local tests.

## 7. Test impact

T1 unchanged-green: all http/logic/repository suites (contract §1).
T2 rewritten: authorizer unit tests to builder-based construction.
T3 deleted-with-replacement: `schema.rs:23-25` count test → derived equality.
T4 intentional deltas (new expectations): unknown-action / wrong-type-pairing
requests return BadRequest (currently silent Deny).

## 8. Risks

| Risk | Mitigation |
|---|---|
| Link id collisions / duplicate links | deterministic ids `link_{role}_{action}`; idempotent link guard |
| Hot-reload race mid-request | Arc swap under RwLock; requests take one snapshot |
| Policy-set size growth (links ≈ grants ≤ 39×roles) | bounded by schema vocabulary; indexable |
| Strict validation rejects future policy edits at startup | fail-closed boot (same as today's strict check) |
| Custom roles with policy-hostile names | name validation already ASCII-restricted; reservation closes literal shadowing |

## 9. Constraints honored

No Cargo.lock edits (lab used separate lockfile); no unwrap added to prod code;
English-only; config stays toml; one commit per slice; no work discarded.

## 10. Questions for adoption (blocking)

- Q1 Adopt D3 (DB-derived links, recommended) or persist linked policy set?
- Q2 Confirm O1/O3/O4 defaults: Application rename; case-insensitive
  reservation of required role names; malformed-input → 400?
- Q3 Defer action-EID cosmetic rename (recommended: defer)?
- Q4 Approve T4 as the only intentional behavior delta?

## 11. Implementation record (post-adoption)

Slices executed; all gates green at completion.

Deltas vs the drafted design, forced by evidence:
1. Schema widening (F12): `Article::Read +Virtual`, `Version::Read +Article`,
   `Comment::Read +Version`, `User::Read +Virtual`; policy 1 condition gained
   the documented `resource has owner &&` guard. Without it, enabling request
   validation rejected four pre-existing legitimate gates.
2. Duplicate entity entries are hard errors under
   `Entities::from_entities(.., Some(&schema))` (4.x semantics), so the old
   merge/dedup-by-uid behavior was restored in `authorize()` — required for
   self-referential requests (actor == resource User).
3. `Principal.roles` simplified to role-name strings; `AuthResource::Role`
   lost its unused permissions payload; new `authorizer::Grant` +
   `Authorizer::reload()`; infrastructure wrapper re-reads all grants from
   graph on reload; `logic/role.rs` reloads after permission mutations and
   role deletion. New error chain: `Error::InvalidRequest` →
   `AuthorizationError::BadRequest` → 400 "invalid authorization request".
4. Test contract updates: `role_grant_authorizes_any_article` now calls
   `reload()` after direct repository seeding (documents that grants mutate
   capability only through the reload boundary; API path reloads implicitly).

Final verification: authorizer 24/24, server 562/562 (= baseline count),
clippy clean both crates, fmt --check clean.

Known accepted risks (per adoption rulings): role-literal shadowing by
case-variant custom roles remains possible; open registration permit retains
its unconstrained principal/resource scope.
