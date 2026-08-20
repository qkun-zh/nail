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

All crates: `env CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=cranelift cargo +nightly test -j 2 --workspace` (from `WORKSPACE`).

Single crate: replace `--workspace` with `-p nail_back` / `-p nail_common` / `-p nail_front`.

**Memory constraint**: cranelift + parallel compilation easily exhausts 12GB RAM.
Always use `-j 2` to cap parallelism. Do not omit this flag.

## Health checks

- Backend: `curl -sf http://127.0.0.1:3000/config/read`
- Via proxy: `curl -sf http://127.0.0.1:8080/api/config/read`
