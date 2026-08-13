# Issue tracker: adjudication log + handoff (freeform)

This repo deliberately uses **no separate issue-tracking system**. Work is
tracked in two existing project documents:

- `document/adjudication.md` — the **decision/issues log**: every defect found
  in the legacy code is recorded here as a numbered item (#1..#32 so far) with
  a verdict (`fix` / `remove` / `keep`) and the reasoning.
- `document/handoff.md` — the **progress document**: current state, remaining
  phases, rules for the migrating agent, and the skill-usage guide.

## Conventions

- New findings during migration (bugs, compromises, source contradictions):
  probe first, report to the owner, then append a numbered item to
  `document/adjudication.md` with the agreed verdict — never silently preserve
  a bug or a compromise design.
- New contracts worth formalizing (e.g. item #26 `intent` parameter, item #5
  policy rewrite) are recorded as ADRs under `document/adr/` — see
  `docs/agents/domain.md`.
- Session progress lands in `document/handoff.md` at the end of each session.

## When a skill says "publish to the issue tracker"

Append a numbered entry to `document/adjudication.md` (with verdict and
reasoning), and note any progress change in `document/handoff.md`.

## When a skill says "fetch the relevant ticket"

Read the referenced numbered item in `document/adjudication.md` or the
referenced section of `document/handoff.md`. The user will normally pass the
item number or the file path directly.
