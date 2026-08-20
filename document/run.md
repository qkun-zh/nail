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

## Test

**CI-first**: commit, then `git push` to `main`; `.github/workflows/ci.yml`
runs fmt, clippy, tests (pow, common, back, front), wasm build and audit on
GitHub runners. CI is the gate — do not rely on local tests.

Check the run result after pushing:

```
document/ci-watch.sh
```

Polls the latest workflow run for the current branch until it completes, then
prints success/failure and the failing job names.

Local tests remain available for a quick smoke pass, one crate per invocation
(parallel crate builds exhaust RAM):

- Backend: `cargo test -j 1 -p nail_back` (from `WORKSPACE`)
- Common: `cargo test -j 1 -p nail_common`; Emailer: `-p emailer`;
  Frontend: `-p nail_front` (host tests); pow: `cargo test -j 1 -p pow --all-targets`

Mandatory `-j 1`; do not omit it; do not combine crates in one command.

## Health checks

- Backend: `curl -sf http://127.0.0.1:3000/config/read`
- Via proxy: `curl -sf http://127.0.0.1:8080/api/config/read`