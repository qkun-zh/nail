# End-to-End Testing Guide

## 0. Already done — do not redo

- `configuration/proxy/plugins.toml` fixed to `code/client/dist`.
- Full auth + tag + article + search + comment/reply flow verified in browser (2026-08-22).
- Fixes applied along the way (see §5); all gates green (fmt, clippy zero warnings, tests).
- Stack left running (server :3000, pingap :8080); check `ss -tln | grep -E ':3000|:8080'`.

## 1. Topology

```
chromium (agent-browser)
   -> http://127.0.0.1:8080   pingap (static dist + /api/* reverse proxy)
      -> http://127.0.0.1:3000  axum server (agdb + SeekStorm + SMTP)
```

- Client routes: `code/client/src/router.rs`.
- API routes: `code/server/src/interface/router.rs`. Every mutating request needs an `x-pow` header; the frontend computes it.

## 2. Secrets

The server reads config from `CONF_DIR` (`code/server/src/infrastructure/config.rs:93`). Use a private dir outside the repo; do not commit credentials:

```
/tmp/opencode/nail-box-conf/
  server.toml, emailer.toml, email.toml, cache.toml
```

`emailer.toml` is filled from `document/private/email_authorization_code.txt`.

The mailbox facts: `3366981949@qq.com` (user_zero_email) and `qkun-zh@qq.com` are the same
mailbox; the authorization code works for both SMTP and IMAP (`imap.qq.com:993`). The login
token email has the challenge UUID as subject and the token UUID as the whole body.

## 3. Running the stack

```bash
CONF_DIR=/tmp/opencode/nail-box-conf setsid code/target/debug/server > /tmp/nail-server.log 2>&1 < /dev/null &
setsid code/proxy/pingap-linux-gnu-x86-full -c configuration/proxy > /tmp/pingap.log 2>&1 < /dev/null &
```

- Use `setsid`; bare `nohup ... &` dies when the shell is reaped.
- Smoke check: `curl -s http://127.0.0.1:8080/` returns index.html.

If the UI loads but API calls fail with route or PoW errors, the dist is stale — rebuild:

```bash
cd code/client && trunk build
```

## 4. Browser walkthrough

```bash
agent-browser open http://127.0.0.1:8080/
agent-browser snapshot
agent-browser click @e2         # "authenticate"
agent-browser fill @e1 "qkun-zh@qq.com"
agent-browser click @e2         # "send"
```

Fetch the emailed token over IMAP (authorization code is the IMAP password):

```python
import imaplib, email
m = imaplib.IMAP4_SSL("imap.qq.com", 993)
m.login("qkun-zh@qq.com", "<authorization_code>")
m.select("INBOX")
_, data = m.search(None, "ALL")
_, d = m.fetch(data[0].split()[-1], "(RFC822)")
msg = email.message_from_bytes(d[0][1])
print(msg.get_payload(decode=True).decode())
m.logout()
```

```bash
agent-browser fill @e3 "<token>"
agent-browser click @e4          # POST /api/users -> session in localStorage
agent-browser network requests
```

## 5. Known findings — all fixed

1. ROLE_MEMBER lacked `Tag::Read/Create/Apply` permissions → tag list 403 dead-ended article
   creation. Fixed in `code/server/src/repository/seed.rs`; baseline test updated.
2. Search snippets for the tag field rendered the raw JSON array (`["e2e"]`). The tags field is
   `StringSet16`, whose highlighter falls back to `value.to_string()`. Fixed by stripping the
   JSON shell server-side in `code/server/src/infrastructure/search.rs` (`clean_tag_snippet`).
3. Version number accepted free text; now semver-validated in the client
   (`code/client/src/page/validation.rs`), wired into article and version create forms.
4. Comment/reply textarea kept its text after a successful post. Fixed in
   `code/client/src/page/article/version/comment/state.rs` (clear the signal on success).
5. Article create stayed on the form after success. Now navigates to `/article/{id}` like
   tag create does (`code/client/src/page/article/create.rs`).
6. Toast CSS was missing entirely (toasts invisible). Styles added to `code/client/search.css`.
7. Default user name was the user_id with dashes stripped (unreadable). Now `"anonymous"`
   (`code/server/src/repository/user.rs`); tests updated.

Not yet covered: pagination edges, download flow, delete/undelete/recycle, role admin UI.

## 6. Teardown

```bash
kill $(pgrep -f '^/home/qkun/nail/code/target/debug/server') $(pgrep -f 'pingap-linux-gnu-x86-full -c')
rm -rf /tmp/opencode/nail-box-conf /home/qkun/nail/data
```

Bare `pkill -f 'target/debug/server'` matches its own command line and hangs — use anchored
patterns or kill by PID.
