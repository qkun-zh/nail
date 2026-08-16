# handoff

> Rule: handoff stays lean. Once an independent task is fully done (committed
> and verified), remove it from this file — don't let completed items pile up.
> Keep only: current state, tasks not yet finished, and unresolved decisions.
> If the file stops being a compact handover, prune it.

## State

- Backend (axum) + frontend (Leptos CSR) + proxy (pingap) knowledge base; agdb
  graph, SeekStorm search, email-challenge + PoW auth, Cedar authz.
- Uncommitted (this slice): `document/workflow.md`, `AGENTS.md`, `README.md` —
  the double-evidence workflow refactor (see Done). The dead-interface slice
  below is also uncommitted.

## Done

- **Workflow refactor**: added double evidence (source + probe) for every
  unknown and a Phase 5.5 adoption gate — no code until evidence is presented
  and the user adopts the plan. Synced wording in `AGENTS.md` and `README.md`.
  `document/decisions.md` abolished; removed all references.
- **Dead-interface deletion**: removed frontend-unused `read_articles`
  (plain-list) and `read_users` across `repository`, `logic`, `interface`,
  `router`, and `common/response`, plus their tests. 304 tests pass.
- **P2/P3 perf refactors (committed 20fdeb4, cf701c4)**: localized `enrich_articles`
  (`repository/article.rs`, targeted `.to()/.from()` + batch `read_rows`, O(E)→O(1));
  and batched `enrich_comment_headers` (`repository/search.rs`, per-comment round trips
  → O(#distinct ids) resolves + constant batch reads). Probes verified identical output
  and refuted the old `where_.ids` plan. 306 tests pass. Tracked in
  `document/performance-refactor.md`.

## Next

- Commit the uncommitted slices (one commit each, clean tree).
- Soft delete (mode Soft, delete-flag 1) still undecided.
- Perf: P2, P3 closed. P6 recycler HashSet not approved. P1/P5/total-cursor open.