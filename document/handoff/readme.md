# Handoff

## Task organization rules (mandatory for every handoff write/update)

1. Every task must be decomposed into a three-level hierarchy, ordered by size:
   **task → stage → slice** (task is the largest unit, stage is intermediate,
   slice is the smallest unit).
   - task is numbered with Roman numerals (e.g. `I.`, `II.`)
   - stage is numbered with capital letters (e.g. `A.`, `B.`)
   - slice is numbered with Arabic numerals (e.g. `1.`, `2.`)
2. A task, once its final gate passes with a green CI run (workflow §10), must
   have its handoff task file deleted — same unified artifact lifecycle as the
   research report and exec doc. Keep only incomplete and in-progress entries.
3. Every slice must record its status, any information requiring the user's
   confirmation, and the user's decisions/choices.
4. Each task must have a clear boundary in the handoff (partitioned by task,
   ownership labeled) to prevent confusion and interference.
5. Do not modify, delete, or interfere with tasks not owned by you; changing
   another's task requires explicit permission.
6. The entire document must be written in English.
7. Each agent's workspace must be separated by a divider of exactly 64
   em-dashes (`—`).
8. Each task must open with a task header in exactly this form, and its
   `Owner` must be a 6-character random code (A-Z, a-z, 0-9; no name/alias):
   ```markdown
   ## Task {roman}: {short title}

   **Owner**: {6-char code}
   **Exec doc**: `document/exec/{4-char code}_{slug}.md`
   **Status**: {one-line progress summary}
   ```
9. Handoff lives in this directory (`document/handoff/`). One task per file,
   following the shared `<4-char code>_<slug>.md` naming of research reports
   and exec docs: `document/handoff/{4-char code}_{slug}.md`. Cleanup follows
   workflow §10 (delete on green CI). This file (`readme.md`) holds these
   rules.