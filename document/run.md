1. Frontend
   Path: `/home/qkun/nail/code/front`
   Build: `cd /home/qkun/nail/code/front && env -u NO_COLOR trunk build`

2. Backend
   Path: `/home/qkun/nail/code/back`
   Run: `cd /home/qkun/nail/code/back && env CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=cranelift cargo +nightly run --bin nail_back`
   Background: `cd /home/qkun/nail/code/back && setsid nohup env CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=cranelift cargo +nightly run --bin nail_back > /home/qkun/nail/log/back/run.log 2>&1 < /dev/null &`

   Dev builds MUST use the Cranelift codegen backend (`-Zcodegen-backend=cranelift`)
   with nightly toolchain and line-tables-only debug info. Dev builds MUST NOT use
   `--release`; release profile (LLVM) is reserved for official builds only.

3. Proxy
   Path: `/home/qkun/nail/code/proxy/pingap-linux-gnu-x86-full`
   Run: `/home/qkun/nail/code/proxy/pingap-linux-gnu-x86-full -c /home/qkun/nail/configuration/proxy`
   Background: `setsid nohup /home/qkun/nail/code/proxy/pingap-linux-gnu-x86-full -c /home/qkun/nail/configuration/proxy > /home/qkun/nail/log/proxy/run.log 2>&1 < /dev/null &`

Health checks: 
`curl -sf http://127.0.0.1:3000/config/read` for the backend
`curl -sf http://127.0.0.1:8080/api/config/read` for the stack via the proxy.
