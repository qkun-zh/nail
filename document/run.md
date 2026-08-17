# Run

Roots: `FRONT=/home/qkun/nail/code/front`, `BACK=/home/qkun/nail/code/back`,
`PROXY=/home/qkun/nail/code/proxy/pingap-linux-gnu-x86-full`,
`CFG=/home/qkun/nail/configuration/proxy`.

Dev flags (MUST for build AND test, in `common`/`back`/`front` alike):
nightly + `-Zcodegen-backend=cranelift` + `CARGO_PROFILE_DEV_DEBUG=line-tables-only`.
Never `--release` — LLVM is official builds only.

## Build & run

1. Frontend — `env -u NO_COLOR trunk build` (from `FRONT`)
2. Backend — `env CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=cranelift cargo +nightly run --bin nail_back` (from `BACK`)
   Background: `setsid nohup env CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=cranelift cargo +nightly run --bin nail_back > /home/qkun/nail/log/back/run.log 2>&1 < /dev/null &`
3. Proxy — `PROXY -c CFG`
   Background: `setsid nohup PROXY -c CFG > /home/qkun/nail/log/proxy/run.log 2>&1 < /dev/null &`

## Test

`env CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=cranelift cargo +nightly test` (from `common`/`back`/`front`).

## Health checks

- Backend: `curl -sf http://127.0.0.1:3000/config/read`
- Via proxy: `curl -sf http://127.0.0.1:8080/api/config/read`
