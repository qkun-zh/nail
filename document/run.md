# Run

Roots: `WORKSPACE=/home/qkun/nail/code`, `FRONT=/home/qkun/nail/code/front`,
`PROXY=/home/qkun/nail/code/proxy/pingap-linux-gnu-x86-full`,
`CFG=/home/qkun/nail/configuration/proxy`.

Toolchain: stable (same as CI). No extra flags, no `--release` — LLVM is
official builds only.

## Build & run

1. Frontend — `env -u NO_COLOR trunk build` (from `FRONT`)
2. Backend — `cargo run -p nail_back` (from `WORKSPACE`)
   Background: `setsid nohup cargo run -p nail_back > /home/qkun/nail/log/back/run.log 2>&1 < /dev/null &`
3. Proxy — `PROXY -c CFG`
   Background: `setsid nohup PROXY -c CFG > /home/qkun/nail/log/proxy/run.log 2>&1 < /dev/null &`

## Testing (CI-first)

**The gate is GitHub Actions, not the local machine.** Per commit:

1. `git commit` (one commit per slice — see workflow §8). Documentation-only
   changes: start the message with `[skip ci]` to skip the CI run.
2. `git push origin main` — pushes all local commits and triggers CI
   (`.github/workflows/ci.yml`): fmt, clippy, tests (pow, common, back,
   front host), wasm build, security audit.
3. Watch the run: `document/ci-watch.sh` blocks until the run completes and
   prints success/failure with the failing job names. Two variants:
   - `document/ci-watch.sh --once` — check once and exit (quick manual check).
   - `document/ci-watch.sh bg [timeout]` — launch in the background, logging
     to `/tmp/ci-watch.log`; poll with `tail -f /tmp/ci-watch.log` whenever
     convenient (e.g. while working on the next slice).

Push succeeded when `git push` exits 0; afterwards
`git log origin/main..HEAD` must be empty (local == remote).

Local tests remain available for a quick smoke pass, one crate per invocation
(parallel crate builds exhaust RAM):

- Backend: `cargo test -j 1 -p nail_back` (from `WORKSPACE`)
- Common: `cargo test -j 1 -p nail_common`; Emailer: `-p emailer`;
  Frontend: `-p nail_front` (host tests); pow: `cargo test -j 1 -p pow --all-targets`

Mandatory `-j 1`; do not omit it; do not combine crates in one command.

**Resource contention**: prefer CI over local builds. Before any local
`cargo` build/test, check machine load (`uptime` / `ps -eo pcpu`). Heavily
loaded → back off, poll periodically. Never build on a busy machine; a shared
tree means unreliable results or disrupted runs.

## Health checks

- Backend: `curl -sf http://127.0.0.1:3000/config/read`
- Via proxy: `curl -sf http://127.0.0.1:8080/api/config/read`