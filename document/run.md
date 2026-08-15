1. Frontend
   Path: `/home/qkun/nail_new/code/front`
   Build: `cd /home/qkun/nail_new/code/front && env -u NO_COLOR trunk build`

2. Backend
   Path: `/home/qkun/nail_new/code/back`
   Run: `cd /home/qkun/nail_new/code/back && cargo run --bin nail_back`
   Background: `cd /home/qkun/nail_new/code/back && setsid nohup cargo run --bin nail_back > /home/qkun/nail_new/log/back/run.log 2>&1 < /dev/null &`

3. Proxy
   Path: `/home/qkun/nail_new/code/proxy/pingap-linux-gnu-x86-full`
   Run: `/home/qkun/nail_new/code/proxy/pingap-linux-gnu-x86-full -c /home/qkun/nail_new/configuration/proxy`
   Background: `setsid nohup /home/qkun/nail_new/code/proxy/pingap-linux-gnu-x86-full -c /home/qkun/nail_new/configuration/proxy > /home/qkun/nail_new/log/proxy/run.log 2>&1 < /dev/null &`

Health checks: 
`curl -sf http://127.0.0.1:3000/config/read` for the backend
`curl -sf http://127.0.0.1:8080/api/config/read` for the stack via the proxy.
