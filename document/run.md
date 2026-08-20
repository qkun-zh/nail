# Run

Roots: `WORKSPACE=/home/qkun/nail/code`, `FRONT=/home/qkun/nail/code/front`,
`PROXY=/home/qkun/nail/code/proxy/pingap-linux-gnu-x86-full`,
`CFG=/home/qkun/nail/configuration/proxy`.

Dev flags (MUST for build AND test):
nightly + `-Zcodegen-backend=cranelift` + `CARGO_PROFILE_DEV_DEBUG=line-tables-only`.
Never `--release` — LLVM is official builds only.

## Build & run

1. Frontend — `env -u NO_COLOR trunk build` (from `FRONT`)
2. Backend — `env CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=cranelift cargo +nightly run -p nail_back` (from `WORKSPACE`)
   Background: `setsid nohup env CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=cranelift cargo +nightly run -p nail_back > /home/qkun/nail/log/back/run.log 2>&1 < /dev/null &`
3. Proxy — `PROXY -c CFG`
   Background: `setsid nohup PROXY -c CFG > /home/qkun/nail/log/proxy/run.log 2>&1 < /dev/null &`

## Test

Single crate only — never `--workspace` (parallel crate builds exhaust RAM).

Backend: `env CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=cranelift cargo +nightly test -j 1 -p nail_back` (from `WORKSPACE`).

Common: `-p nail_common`; Emailer: `-p emailer`; Frontend: `-p nail_front`. One crate at a time.

**Memory constraint**: cranelift + parallel compilation easily exhausts 12GB RAM.
Mandatory: `-j 1` (single-threaded compilation) and one crate per invocation.
Do not omit `-j 1`; do not combine crates in one command.

## Health checks

- Backend: `curl -sf http://127.0.0.1:3000/config/read`
- Via proxy: `curl -sf http://127.0.0.1:8080/api/config/read`
