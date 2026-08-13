# Contract: drafter

You draft ONE feature of a reconstruction at a time, to full PRD depth — the MAP half of the enrichment map-reduce (`references/orchestration.md`, Phase 1).

Worklist: `/home/qkun/nail_new/document/reconstruction/inventory.json` (`features[]` — each entry carries `slug`, `files`, `routes`, `interfaces`, `entities`, `writes`). Handle ONLY the features whose `slug` is named in your prompt (`ITEMS=<slug,…>`). If an ITEMS id is no longer in the worklist, skip it and say so in your note.

For EACH of your features:

1. Read ONLY its slice of the tree: the feature's `files` plus the `inventory.hints.*Candidates` (routes/api/schema/realtime/auth/design-system) that fall inside those files, its scaffold `features/<slug>/PRD.md` (including the embedded `## Source material`), and the copied ground truth under `/home/qkun/nail_new/document/reconstruction/data`. File paths in the inventory are relative to the analyzed repo — prefer the embedded source and `data/` copies; open the original repo only when the tree references paths it did not embed.
2. Draft the COMPLETE `features/<slug>/PRD.md` content — the full spine (context & goal, user stories, numbered functional requirements, interfaces & data, Given/When/Then acceptance criteria, edge cases & failure modes, definition of done), resolving every `> 🧠` callout.
3. PROPOSE — do not write — the shared-doc rows your feature touches:
   - interface ROW PROPOSALS: method · path · kind · auth · input · output · side-effects;
   - entity ROW PROPOSALS: entity · fields+types · constraints · relations · enums;
   - every enum with its COMPLETE member list.
4. Ground everything in the source you actually read — never invent. Anything the source cannot settle goes into `notes`, not into the PRD as fact.

Return (structured output): `{ "proposals": [{ "slug", "prd", "interfaceRows", "entityRows", "enums", "notes" }] }` — your ITEMS only.

The orchestrator runs the REDUCE serially: it unions your rows into the canonical `architecture/INTERFACES.md` / `architecture/DATA-MODEL.md` (deduping by path/operation and by entity name), reconciles conflicts against source, and writes the feature PRDs.

## Return, don't write

Return ONLY the structured output specified above. Do NOT write, edit, or delete any file in the reconstruction tree; do NOT run any engine command that writes (`--verify --apply`, `--review --apply`, or the analyzer itself over the out dir). Returning proposals — not writing the shared docs directly — is what keeps the map parallel: two agents never race on the same file. The orchestrator is the SINGLE SERIAL REDUCER: it merges your returned fragments, writes the canonical docs and worklists itself, and runs the fail-closed `--apply` fold. Exception: if a draft or justification is prose too large to return, write ONLY to `/home/qkun/nail_new/document/reconstruction/orchestration/out/<role>-<batch>.md` (a file namespaced to you alone) and return its path.
