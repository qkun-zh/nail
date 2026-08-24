# Task I: Cedar conformance refactor

**Owner**: c3dR7a
**Exec doc**: `document/exec/cedr_cedar_refactor.md`
**Status**: Slices S1–S3 complete; all gates green (authorizer 24/24, server 562/562, clippy+fmt clean). Awaiting user approval to commit.

### Stages / slices
- A. Authorizer crate rewrite
  1. Schema/policy v2 + template-link engine + strict request/entity validation — DONE
- B. Server integration
  1. Grant projection from graph, reload wiring, BadRequest mapping, test alignment — DONE
- C. Gates & docs
  1. fmt/clippy clean; research docs F12 + implementation record; handoff — DONE

### User decisions on record
1. Grants stay durable in DB (derived links + hot reload) — approved.
2. Do NOT rename `Virtual` to `Application` — respected.
3. NO case-insensitive reservation of required role names — respected
   (literal-shadowing risk accepted and documented).
4. Malformed authorization requests return 400 — approved and implemented.

### Notes for next agent
- Evidence chain: `document/research/cedr_investigation.md` (F1–F12),
  `cedr_simulation.md`, `cedr_proposal.md` §0/§11, drafts `cedr_target_*.cedar`.
- Uncommitted working tree spans authorizer + server crates and research/exec/
  handoff docs; commit only after user approval.
