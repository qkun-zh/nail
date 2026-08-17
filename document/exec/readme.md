# Exec — execution documents

Execution documents live in this directory. Written at workflow §5 before any
code; single source of truth during execution.

## Rules (mandatory)

1. One exec doc per task, named with a 4-character random alphanumeric code and
   a short slug, matching the task's handoff file slug:
   `document/exec/{4-char code}_{slug}.md`. Code is unique; no reuse.
2. Under 300 lines.
3. Written in English.
4. Required sections (workflow §5), each "N/A" only with a one-line reason:
   Requirement, Scope, Design decisions, Slice breakdown, Open unknowns,
   Verification plan, Risks, Constraints, Questions.
5. Update in place when evidence contradicts; append `## Change log` at bottom.
6. The exec doc is the single source of truth — read it at the start of every
   slice. Do not modify a task's exec doc unless you own the task.
7. When the task is fully complete, delete its exec doc, so only in-progress
   exec docs remain.