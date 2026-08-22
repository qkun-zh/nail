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

The mailbox facts: `qkun-zh@foxmail.com`, `qkun-zh@qq.com`, and `3366981949@qq.com`
are the same physical mailbox (aliases) — see `document/private/email_authorization_code.txt`.
The single authorization code `bqlpvuknilcycjej` works for all three via SMTP+IMAP
(`imap.qq.com:993`). The login token email has the challenge UUID as subject and the token
UUID as the whole body.

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
8. Delete requests sent `?mode=%22soft%22` — `serde_json::to_string(&DeleteMode)` double-encoded
   the value, so every delete returned 400. Added `DeleteMode::as_str()`
   (`code/common/src/request.rs`) and replaced all six call sites
   (`code/client/src/request/{article,comment,version,user,tag,role}.rs`).
9. Download mint URL used a stale route (`/api/article/…/version/…/content/read`) that 404'd.
   Fixed to `/api/articles/{id}/versions/{vid}/content`
   (`code/server/src/logic/download.rs`); test assertions updated.
10. Version soft-delete left `latest_version_id` pointing at a deleted version. Added
    `refresh_live_latest_version` (`code/server/src/repository/delete.rs`), wired into the
    soft-delete branch of `code/server/src/logic/version.rs`.
11. Role update returned `NamedRef` while the client parses `RoleView`, surfacing as "missing
    field `permissions`". `update_role` now returns the full view
    (`code/server/src/logic/role.rs`).
12. The rename and email-change pages ignored the `:uid` route param and mutated the session
    user — visiting another user's page renamed yourself. Both now reject mismatches with a
    toast (`code/client/src/page/user/{name,email}/update.rs`).
13. Search pagination controls were re-created on every results render, so clicks landed on
    detached nodes. `PrevNext` moved out of the reactive rows closure
    (`code/client/src/page/article/search/results.rs`).
14. Version undelete cleared the flag but left `latest_version_id` on an older version.
    `undelete_soft_version` now calls `refresh_live_latest_version`
    (`code/server/src/logic/version.rs`).
15. Searching `article_id`/`version_id`/`comment_id` ranges always returned zero hits —
    quotes and dashes were irrelevant. Those schema fields were declared
    `index_lexical=false`, so no posting lists existed; only `author_id` was indexed.
    Flipped all three to `index_lexical=true` and bumped `SCHEMA_VERSION`
    (`code/searcher/src/schema.rs`) so stale indexes are wiped and reseeded on boot.
    Regression test `dashed_id_ranges_match_exact_documents`
    (`code/searcher/src/tests/read.rs`). Note: a dashed UUID survives query tokenization
    intact (only a *leading* `-` means exclude), so exact id lookup works unquoted;
    prefixes do not match (exact term only).
16. A first-ever index create (directory absent, e.g. after deleting
    `data/search`) did not set the `recreated` flag, so the server skipped
    `sync_all` and served an empty search index until some article was touched.
    Fresh creates now report recreated like wipe-recreates
    (`code/searcher/src/searcher.rs`), test renamed to
    `fresh_open_reports_recreate_and_reopen_keeps_data`.
17. Navigation audit (cost model: click 1, back 1, URL 16) found 13 orphan
    pages reachable only by typing URLs — `/article/create`, the whole `/tag`
    and `/role` domains, and `/user`; tag detail was a dead end with zero
    outbound links. Mounted per the ownership hierarchy: portal section on
    the index page (`/user /tag /role` + session "my hub"), create-article in
    the author's article list header, create-tag on the tag list, update and
    delete on tag detail mirroring role detail, and applied tag names on
    article detail now link to their tag pages. Pairwise navigation cost
    drops 9.26 → 3.67; verified by UI walkthrough.
18. Tag detail showed only the tag name plus update/delete links — its
    articles (the owned content) were invisible, forcing a manual search.
    `read_tag` now returns the full `TagListItem` (with `article_count`)
    instead of `NamedRef` (`code/client/src/request/tag.rs`), and the detail
    page lists articles found by searching `ranges=tag` for the tag name
    with title/author links (`code/client/src/page/tag/detail.rs`).
    Verified live: `/tag/{id}` shows "articles (1):" and the entry links to
    `/article/{id}`.
19. Same-class sweep after #18: every detail page was audited for hidden
    owned content. One gap remained — `/user/{uid}/role` rendered role names
    as plain text because `roles_of_user` dropped node ids, while
    `role/detail.rs` links its members back to users (asymmetric). The
    repository now returns full `RoleRow`s, `UserView.roles` and
    `UserListItem.roles` carry `RoleRef {id, name}`
    (`code/common/src/response/user.rs`,
    `code/server/src/repository/role.rs`, `code/server/src/logic/user.rs`),
    and the client renders each role as a link to `/role/{id}`
    (`code/client/src/page/user/role.rs`). Verified live: member's role page
    shows "member" linking to its role page (403 inside is the permission
    wall, consistent with the portal link).
20. Backend logs recorded almost nothing per request: tower-http's default
    TraceLayer emits access events at DEBUG, while the filter
    (`warn,server=info,common=info`) dropped them before the file appender.
    Fixed with a customized TraceLayer in `code/server/src/infrastructure/server.rs`:
    span carries only `method` + `uri`, on_request silenced, on_response emits a
    single line `status=<code> latency_ms=<n>` at INFO (WARN for >=400), and the
    download token query param is redacted to `<REDACTED>` before it can reach the
    log. Filter default gained `tower_http=info`
    (`code/server/src/infrastructure/config/logging.rs`, `configuration/server.toml`).
    Verified by isolated probe server: 200 → INFO line, 400 → WARN line,
    `?token=…` never appears raw. Restart the stack to pick the change up.

## 5.2 Full-database diff verification

Every mutating operation was replayed against full-database snapshots
(`dbdump <copy> | sort`, `diff` between consecutive states) — not just targeted queries:

| operation | exact graph delta |
|---|---|
| tag create | +1 tag node |
| tag update | tag_name in place |
| tag delete (hard) | -1 tag node |
| role create / update grant / update revoke / delete | +role; +2 edges (`user_hold_role`, `role_grant_permission`); -2 edges; -role |
| article create | +article (+title/summary/latest_version_id) +version (+content_hash/note/semver) +3 edges |
| version create | +version +hold edge, latest_version_id moves |
| version soft delete / undelete | flag +pointer rollback / flag clear +pointer restore |
| comment create / reply | +comment +author+attach edges / +reply edge instead of attach |
| comment soft delete / undelete | subtree flags ±1 (parent cascades replies) |
| article update | title in place |
| article soft delete / restore | whole subtree flagged / unflagged |
| article hard delete | -5 nodes and -8 edges exactly, no orphans |
| deregister soft | `soft_deleted` flag, session cleared, login refused |
| deregister transfer (with content) | user node removed, `user_hold_role` cleared, `user_author_article` repointed to recycler, articles preserved |

Non-mutations verified to produce zero DB diff: failed validations, 403s, logout.

Not yet covered: headless blob download save (network-level 200 verified, file-save hangs in headless).

## 5.1 Verified by walkthrough (UI + database)

- Login/logout for member and admin personas; soft-deleted accounts are refused at login
  ("email address is deactivated") but still receive challenge mails.
- Article: create, update, tag apply/unapply, soft delete + admin restore; search hides
  deleted articles. Version create/update/soft-delete with `latest_version_id` rollback.
- Tag list/detail counts match `article_apply_tag` edge counts in the DB dump.
- Roles: create, update grant/revoke, delete cascades edges; member update/delete are 403 by
  design (the UI still renders the links).
- Users: list/hub/id/article/name/email/role pages; deregister(soft) sets the flag, clears
  sessions and blocks login until an admin restores via `/user/{uid}/undelete-soft`;
  deregister(transfer) on a disposable foxmail user with one article — article repointed from
  deleted user to recycler (`user_author_article` edge `176->175` became `90->175`), user node
  and `user_hold_role` removed.
- Search: pagination (`page`/`limit`, 12-article fixture), ranges checkboxes, from/to time
  filter, empty-query hint, no-match "none". Index page is intentionally two links only.
- Email change full happy path: old/new tokens both arrive in the shared inbox
  (`qkun-zh@foxmail.com` is an alias of `qkun-zh@qq.com` — one physical mailbox, same
  authorization code; see `document/private/email_authorization_code.txt`). DB shows exactly
  one property change (`email_address_hash`), and login under the new address succeeds.
  Reverted afterwards to keep fixtures stable.

Tooling notes: `agent-browser` coordinate clicks on bottom-of-page buttons can be swallowed
by overlays — dispatch `el.click()` via `eval` instead. Sessions live in server memory and do
not survive restarts; re-login after each restart. Email sends have a ~60 s per-address
cooldown that rejects with "email already sent recently" — poll the toast and only sleep the
remaining window instead of a fixed 60 s.

## 6. Teardown

```bash
kill $(pgrep -f '^/home/qkun/nail/code/target/debug/server') $(pgrep -f 'pingap-linux-gnu-x86-full -c')
rm -rf /tmp/opencode/nail-box-conf /home/qkun/nail/data
```

Bare `pkill -f 'target/debug/server'` matches its own command line and hangs — use anchored
patterns or kill by PID.
