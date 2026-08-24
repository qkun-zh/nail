# Cedar Misuse Investigation (cedr)

## 1. Requirement

`R₀`: All Cedar usage in `nail` must conform to the official Cedar documentation
(docs.cedarpolicy.com, language v4.x) and its best practices; every deviation
must be identified with evidence and be eliminable without breaking the
pinned behavioral contract (README + tests).

## 2. Research questions

- Q1 Which current mechanisms violate documented Cedar semantics or modeling?
- Q2 What exactly breaks if schema validation is enabled at request/entity time?
- Q3 What behavioral contract must a refactor preserve? (24 invariants extracted)
- Q4 Where does authorization logic live outside Cedar, and is that legitimate?
- Q5 What data/model changes do fixes require?

## 3. Evidence (source findings; probes live in `ced2` simulation report)

### F1 Fabricated Role→Action parent edges violate declared membership — CRITICAL
`authorizer/src/authorizer.rs:107-121` turns each role permission into an
`Action::<name>` entity and attaches it as a **parent of the Role entity**;
User parents = roles (:123). The schema declares bare `entity Role;`
(`cedar/schema.cedar:6`) — no permitted parent types. Schema docs: membership
"is a list of entity types that can be *direct* parents"; action grouping is a
schema-level construct whose members are Actions under Action groups, never
Roles parenting Actions (`schema/human-readable-schema.html`,
`overview/terminology.html`). **Probe E1**: `Entities::from_entities(..,
Some(&schema))` rejects production-shaped entities ("entity does not conform to
the schema"). The mechanism survives only because validation is off (F4).

### F2 Unconstrained mega-permit — CRITICAL
`cedar/policy.cedar:47-48` `permit(principal, action, resource) when { principal
in action }` is unconstrained in all three scope slots. `bp-populate-policy-scope`
names exactly this the anti-pattern ("Resist the temptation to move imperative
authorization code into `when` clauses… dead end from a scaling and analysis
perspective"); every request evaluates it; the policy store cannot answer "what
can alice do?" without simulating entity fabrication.

### F3 Permission mapping lives in graph edges, not policies — CRITICAL
Role→permission is stored as `(Role)-[RoleGrantPermission]->(Permission)` edges
(`repository/role.rs:100-120`, seeded `repository/seed.rs:22-40`) and consumed
by fabricating entities per request (`infrastructure/authorizer.rs:50-66`).
`bp-model-all-perms`: "If you have a permissions table, each row … would become
a separate Cedar policy". `other/security.html`: "Put all authorization logic in
your Cedar policies." Note policy.cedar:1-3 declares edge-driven permissions a
deliberate product rule → this finding requires a product decision, not just code.

### F4 Request and entity validation disabled — HIGH
`authorizer.rs:64-65` `Entities::from_entities(entities, None)` and :66-72
`Request::new(.., None)`. 4.x semantics (verified against docs.rs + source):
`None` skips `RequestValidationError` entirely (UndeclaredAction,
InvalidPrincipalType/ResourceType, InvalidContext). **Probe E2**: bogus action
string builds fine and silently Denies; `Article::Create` paired with an
`Article` resource builds fine and silently Denies instead of being rejected as
malformed. Policies ARE strict-validated at startup (`authorizer.rs:33`) but
that guarantee is discarded at evaluation time.

### F5 Soft-delete visibility enforced outside Cedar — MEDIUM (resolved: compound)
`logic/authorize.rs:24-42,199-206` compose "read grant" AND "undelete grant if
flagged" in Rust; repositories additionally reject writes into hidden subtrees
(`repository/version.rs:149-151`, `repository/comment.rs:385-387`, etc.).
Investigation conclusion: the *decision legs* already go through Cedar (the
second leg is `authorize(Undelete::Soft)`); composition of two PARC answers is
application response-shaping, explicitly sanctioned by `bp-compound-auth`.
A single-query forbid guard was prototyped and **empirically falsified**
(neither exemption variant reproduces hide-from-members/show-to-holders;
see `ced2` §E3-fail) → keep compound form. Not a misuse once F1-F4 are fixed.

### F6 No container hierarchy — MEDIUM
Schema lacks `Version in [Article]`, `Comment in [Version, Comment]`
(`schema.cedar:7-11`) although code already builds these parent chains per
request for owner lookup only (`infrastructure/authorizer.rs:135-188`).
`bp-resources-containers` / relationship pattern expect declared membership so
`resource in Article::"x"` scoped policies are possible.

### F7 Creation/global gates anchored on ad-hoc Virtual; registration scope-empty — LOW
`Virtual("any")` carries no meaning; `bp-authorization-patterns` prescribes a
synthetic **Application** anchor for creation and open endpoints. Policy 7
(`policy.cedar:68-71`) permits `User::Create` with fully empty scope (any
principal/resource) rather than anchoring on an application entity.

### F8 Synthetic anonymous principal — RETRACTED (correction during planning)
Initial agent-reported claim said `User::"anonymous"` is absent from entity
data. Direct re-read of `authorizer.rs:101-125` shows `build_principal`
unconditionally pushes the principal UID entity (:123), so the synthetic
anonymous entity IS present (bare, no parents) and `principal in action`
evaluates false cleanly. No defect; no change required beyond keeping the
existing behavior under schema-conformant construction.

### F9 Empty-owner sentinel `User::""` — LOW (latent bug risk)
`authorizer.rs:260-264` maps empty owner id to `User::""`;
`infrastructure/authorizer.rs:108-110` uses `unwrap_or_default()` for comment
owners. Owner edges are always written today (`article.rs:142-148`,
`comment.rs:70-76,122-128`; transfer refuses orphans `transfer.rs:143-145`),
so the sentinel is currently unreachable — but if ever hit, comment authors
silently lose self-service (owner equality can never match).

### F10 String-built EntityUids + un-normalized role names — MEDIUM
`parse_uid` string-formats UIDs (`authorizer.rs:216-220`) — docs.rs warns to
prefer `EntityUid::from_type_name_and_id`. Role names keep case and only trim
(`logic/role.rs:22-34`): `"Admin"` may coexist with `"admin"` while static
forbids reference exact literals `Role::"recycler"`/`Role::"admin"`
(`policy.cedar:59,66`) — a normalization miss silently escapes a forbid
(`bp-normalize-data-input`, security requirement tier).

### F11 App-layer observations (not Cedar misuse; recorded for the refactor)
Tag apply/unapply ignores article rights by design (`logic/tag.rs:119-144`);
`authorize_entity` (no `_or`) leaks generic 404 text (tag update/delete, user
restore/hard paths); role routes resolve rows before authorizing (only such
reads); per-decision DB cost 1–5 read txns (comment worst, reply-chain walk);
39-count test hardcodes schema size (`authorizer/src/tests/schema.rs:23-25`).

### F12 Four collection gates anchor actions outside their declared appliesTo — HIGH (discovered during implementation, S2)
With request validation ON, four existing gates were rejected as invalid PARC
pairings — the old `None`-schema path had silently tolerated them:
- search gate: `Article::Read` × `Virtual` (`logic/search.rs:21`)
- version list gate: `Version::Read` × `Article` (`logic/version.rs:171-176`)
- comment list gate: `Comment::Read` × `Version` (`logic/comment.rs:78-83`)
- admin user list: `User::Read` × `Virtual` (`logic/user.rs:118`)
Resolution (behavior-preserving): widen those four actions' `appliesTo`
resource lists in schema.cedar so the schema tells the truth about the app;
owner-bypass on the widened types is neutralized with the documented
defensive guard (`resource has owner && …`) which evaluates identically for
concrete entities and skips attr-less Virtual exactly as the old skip-on-error
did. Recorded as an accepted multi-type-read deviation from bp-map-actions.

## 4. Findings summary

| ID | Severity | Class | Fix direction |
|----|----------|-------|---------------|
| F1 | critical | off-spec mechanics | remove fabrication; grants become template-linked policies |
| F2 | critical | BP anti-pattern | delete mega-permit |
| F3 | critical | BP violation | same as F1 (links derived from existing DB edges) |
| F4 | high | validation off | pass `Some(&schema)` to both builders |
| F10| medium  | normalization | reserved names, programmatic UIDs |
| F6/F7/F8/F9 | low–medium | modeling hygiene | containers, Application anchor, real anonymous entity, drop sentinel |

Behavioral contract extracted (Q3): 24 invariants covering decision outcomes
(owner bypass set, member seed set, recycler/admin forbids, self-service,
open registration, deny-by-default messages), visibility (404-hiding incl.
subtree + search), session/PoW/download-token gates, envelope mapping — full
list embedded in `ced3` §Scope.

## 5. Impact on requirement

No revision of R₀ needed; one scoping note: F3 shows "permissions editable at
runtime via API" is a product capability that must survive the refactor, which
constrains the solution space to mechanisms that keep grants dynamic
(template-linked policies), not static hand-written policies alone.

## 6. Open items (user input)

- O1 Accept replacing `Virtual` with a semantically named `Application` entity?
- O2 Keep the DB as durable grant store; derive template links at startup and
  hot-reload on grant/revoke (recommended), or persist the linked policy set?
- O3 Reserve required role names case-insensitively (blocks `"Admin"` squatting)?
- O4 Map malformed authorization inputs (unknown action, wrong resource type) to
  400 BadRequest instead of silent Deny?
- O5 Cosmetic rename of action EIDs (`Article::Create` → camelCase) — defer?

Evidence sources: all file:line references verified in working tree; doc claims
traced to docs.cedarpolicy.com pages named inline; engine semantics verified
against cedar-policy 4.12.0 rustdoc/source and reproduced empirically (ced2).
