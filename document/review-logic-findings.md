# Logic Findings Review — double evidence (source + probe)

Date: 2026-08-18 · Scope: `code/back`, `code/front` · Review type: logic/correctness

Each finding records **location**, **source evidence** (the exact code that is wrong),
and **probe evidence** (a failing test that demonstrates it, or — where a probe is
not feasible without refactoring — a precise source-level demonstration).

Probe tests live in `test/unit/back/logic/probe_review_findings.rs`, wired into the
harness (`test/unit/back/harness.rs` → `logic_probe_review_findings`). Each probe
encodes the *expected correct* behavior and is currently **red** (proves the bug);
fixing the finding flips it green. Run:

```
env CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=cranelift \
  cargo +nightly test probe_review --bin server
```

---

## 1. [HIGH] `read_tags` out-of-range slice panics on a page past the end

**Location:** `code/back/src/logic/tag.rs:50-54`

### Source evidence
```rust
let offset = page.saturating_sub(1).saturating_mul(limit);
let page_tags = &tags[usize::try_from(offset).unwrap_or(usize::MAX)
    ..usize::try_from(offset + limit).unwrap_or(tags.len()).min(tags.len())];
```
When `offset >= tags.len()` the slice `tags[start..end]` has `start > end` (and often
`start > len`) → Rust panics ("range start index … out of range"). `page` is only
clamped to `MAX_PAGE=10_000` / `limit` to `MAX_PAGE_SIZE=200` at
`code/back/src/interface/tag.rs:38-43`, never to the actual tag count, so `?page=2`
with a small tag set reaches this. Sibling `read_users`/`read_roles` use the safe
`.skip(offset).take(limit)`; only this one slices. This violates the project's
panic-free rule (README §Robustness).

### Probe evidence — red
`probe_1_read_tags_must_not_panic_on_a_far_page` (catch_unwind around `read_tags(page=2, limit=200)` with one tag). Observed:

```
thread '…probe_1_read_tags_must_not_panic_on_a_far_page' panicked at src/logic/tag.rs:51:26:
range start index 200 out of range for slice of length 1
```

**Impact:** any sufficiently large `page` on `/tag/read` → 500. Fix direction: `.iter().skip(offset).take(limit)`.

---

## 2. [MEDIUM] `delete_session` keys the delete on the raw token, not the normalized one

**Location:** `code/back/src/logic/session.rs:62` (vs `:20` and `:32`)

### Source evidence
```rust
// create_session :32  — stored under the CANONICAL token
let session_key = token_key(&session_token)…;
state.caches.session.insert(&session_key, …);

// read_session   :20  — reads via normalize_token (canonical)
let key = token_key(&token)…;  // token = normalize_token(raw)

// delete_session :62  — keys on the RAW header token
let key = token_key(session_token)…;   // NOT normalized
state.caches.session.delete(&key);
```
`normalize_token` (`session.rs:9-15`) strips whitespace and canonicalizes the UUID
(`Uuid::parse_str` is case-insensitive). So a token echoed with a different case
passes `read_session` but its delete lookup misses → the session is **not deleted**.

### Probe evidence — red
`probe_2_delete_session_with_noncanonical_token_must_remove_the_session`: creates a
session, deletes it with the uppercased token, then asserts `read_session` fails.
Observed:

```
panicked: session must be deleted even when the client echoes the token in a different case
```

**Impact:** logout can silently fail for non-canonically-echoed tokens. Fix: use the
normalized token for the delete key.

---

## 3. [MEDIUM] Search pagination assumes ≤32 indexed docs per article, which comments break

**Location:** `code/back/src/repository/search.rs:241` + `code/back/src/repository/search/document.rs:144-278`

### Source evidence
```rust
// search.rs:23   const MAX_DOCS_PER_ARTICLE: u64 = 32;
// search.rs:241  window is sized in DOCS:
let top_k = usize::try_from((request.offset + request.limit * MAX_DOCS_PER_ARTICLE).max(1))…;
```
The fetch window is `offset + limit*32` **docs**, but pagination is then applied at the
**article** level (`logic/search.rs:81-88`): `article_list.skip(offset).take(limit)` and
`has_next = article_list.len() > offset+limit`. This is only correct if every article
contributes ≤32 docs. However `build_documents` emits **one doc per version plus one
per comment** with no per-article cap, so a comment-heavy article exceeds 32 docs and
consumes more window than assumed → pages come up short and `has_next` can be false
while unshown matches remain.

### Probe evidence — the premise holds (doc count exceeds 32)
`probe_3_a_comment_heavy_article_exceeds_the_32_doc_per_article_assumption`: one
article + 40 comments → `sync_all` reports the indexed doc count. Observed result:
**ok** — the test's own assertion (`doc_count > 32`) passed, confirming 41 docs for one
article. This demonstrates the `MAX_DOCS_PER_ARTICLE=32` assumption is violated in
normal use.

**Impact:** search pages over comment-rich content are truncated and `has_next` is
unreliable. Fix direction: page at the article level in the repository, or cap docs per
article during indexing.

---

## 4. [LOW/MEDIUM] Download token is consumed before the target version is verified

**Location:** `code/back/src/logic/download.rs:96-105`

### Source evidence
```rust
let consumed = state.caches.download.consume_if(&key, |entry| entry.user_id == actor_id);
let Some(consumed) = consumed else { … };
if consumed.version_id != version_id {
    return Err(LogicError::not_found("article version not found"));   // token already gone
}
```
`consume_if` destroys the token first; the version match is checked afterwards. A token
minted for version A, mis-targeted at version B, is destroyed even though the intended
target A download was legitimate.

### Probe evidence — red
`probe_4_token_must_survive_a_version_mismatch_attempt`: mint for version A, consume
against version B (expects `NotFound`), then consume again for A. Observed:

```
panicked: token must survive a version-mismatch attempt for its intended target:
  BadRequest("invalid or expired download token")
```

**Impact:** an accidental mis-target consumes a user's download token. Fix direction:
check `version_id` from a non-consuming read before `consume_if`.

---

## 5. [LOW] List-read endpoints authorize against a global `Virtual` resource; single-read endpoints authorize the concrete resource

**Location:**
- `code/back/src/logic/comment.rs:87-93` (`read_comments`) vs `comment.rs:127-134` (`read_comment`)
- `code/back/src/logic/version.rs:187-193` (`read_versions`) vs `version.rs:137-143` (`read_version`)
- `code/back/src/logic/search.rs:21-27` (`search_articles`)

### Source evidence
```rust
// list: global capability check
authorize(state, actor_id, PERMISSION_COMMENT_READ, &Resource::Virtual("any".to_string())).await?;
// single: concrete resource assembly (owner-bypass path in cedar policy 1)
authorize_or(state, actor_id, PERMISSION_COMMENT_READ, &Resource::Comment(id), "comment not found").await?;
```
The list endpoints bypass `assemble_resource` for the specific version/article/comment;
only single-read endpoints exercise the concrete resource (and thus cedar policy 1's
owner bypass). The owner-bypass path therefore applies to single reads but not to list
reads of the same content.

### Probe evidence
Not separately probed here: it is an authorization-**model** inconsistency. The actual
Cedar runtime semantics for an action applied to a `Virtual` resource (which the schema
does not declare for these actions, `schema.cedar:14/21/27`) were probed earlier in the
repo's own `infrastructure/cedar_probe.rs` / `probe_002_orthogonal_action_resource.rs`.
Verified correct (ruled out): no distance/edge-direction bug in the authorization graph
assembly (`repository/authorization.rs`).

**Impact:** low under the current policy (roles grant these actions globally via policy
3), but the owner-bypass inconsistency and reliance on Cedar *not* enforcing `appliesTo`
at runtime make it fragile. Fix direction: align list reads to the concrete resource.

---

## 6. [MEDIUM] Search empty-query path does not invalidate in-flight requests

**Location:** `code/front/src/page/article/search.rs:202-209` (with the seq guard at `157-193`)

### Source evidence
```rust
let q = q_filter.get_untracked().trim().to_string();
if q.is_empty() {
    search_list.set(Vec::new()); has_next.set(false);
    current_page.set(1); loaded.set(true); fetching.set(false);
    return;                       // <-- never bumps request_seq
}
```
Every real search calls `run_search` (bumps `request_seq`), so its completion guard
`request_seq.get_value() != my_seq` rejects stale results. The empty-query branch clears
state but **does not bump `request_seq`**. If an earlier search (`my_seq = 1`) is still
in flight when an empty query is submitted, that in-flight request later completes with
`request_seq == 1 == my_seq` and repopulates the cleared list with stale results.

### Probe evidence
Not probe-able as a pure function (reactive spawn/effect inside the component; the
frontend test harness only covers pure helpers, `test/unit/front/page/pagination/tests.rs`).
Source-level demonstration above is unambiguous. Fix direction: bump `request_seq` in the
empty branch too.

---

## 7. [MEDIUM] Article-create draft persists tags to the URL but never restores them

**Location:** `code/front/src/page/article/create.rs:23, 29-37`

### Source evidence
```rust
let selected_tags = RwSignal::new(Vec::<String>::new());           // never read from query
persist_draft(navigate, "/article/create", move || {
    vec![
        ("title",   title.get()),
        ("summary", summary.get()),
        ("tags",    selected_tags.get().join(" ")),                 // written to URL
        ("version", version.get()),
        ("note",    note.get()),
    ]
});
```
`title/summary/version/note` are restored from the query on load (`:21-25`); `selected_tags`
is initialized empty and the persisted `tags` parameter is never read back into it, so the
tag selection is dropped on navigation/reload while the other fields survive.

### Probe evidence
Not probe-able as a pure function (state restoration in the component). Source-level
evidence above. Fix direction: restore `selected_tags` from the `tags` query parameter.

---

## 8. [LOW] Search initial page is not clamped to ≥1

**Location:** `code/front/src/page/article/search.rs:113-117`

### Source evidence
```rust
let page = params.get("page").and_then(|v| v.parse::<u64>().ok()).unwrap_or(1);
current_page.set(page);
```
A URL like `?page=0` sets `current_page = 0` and fires `do_search(0)`, sending `page=0`
to the backend before it is clamped back. The sibling pages clamp (`.max(1)`):
`version/index.rs:26`, `comment.rs:45`. Search is the outlier.

### Probe evidence
Not probe-able as a pure function (component initialization). Source-level evidence above.
Fix direction: `.unwrap_or(1).max(1)`.

---

## 9. [LOW] Comment update draft is not tracked by the URL-sync effect

**Location:** `code/front/src/page/article/version/comment.rs:107-113` (reported by sub-agent; not independently re-verified)

### Source evidence (as reported)
The `sync_url` `Effect` tracks `(body, reply_body, comment_path, page)` but the
`UpdateComment` arm writes `update_body`; because `update_body` is not read inside the
effect, typing in the update form never triggers a URL sync, so the update draft (which
the component tries to restore from `?update=`) is never persisted and is lost on reload —
inconsistent with the `body`/`reply` draft handling.

### Probe evidence
Not probe-able as a pure function. Source-level evidence above; flagged with lower
confidence pending direct re-verification.

---

## Summary

| # | Severity | Location | Bug | Probe |
|---|---|---|---|---|
| 1 | HIGH | `back logic/tag.rs:50` | out-of-range slice panic (500) | red (panic `range start index 200 out of range`) |
| 2 | MEDIUM | `back logic/session.rs:62` | delete keyed on raw, not normalized token | red |
| 3 | MEDIUM | `back repository/search.rs:241` | doc-window assumes ≤32 docs/article; comments exceed it | premise proven (>32 docs) |
| 4 | LOW/MED | `back logic/download.rs:96` | token consumed before version check | red |
| 5 | LOW | `back logic/{comment,version,search}.rs` | list vs single-read auth model inconsistent | source evidence |
| 6 | MEDIUM | `front search.rs:202` | empty-query path doesn't invalidate in-flight request | source evidence |
| 7 | MEDIUM | `front create.rs:23` | tags draft persisted but not restored | source evidence |
| 8 | LOW | `front search.rs:113` | initial page not clamped ≥1 | source evidence |
| 9 | LOW | `front comment.rs:107` | update draft not URL-synced | source evidence (sub-agent) |

**Priority to fix:** #1 (directly reachable panic) and #6 (stale state overwrite). Probes
for #1–#4 are in-tree and turn green once fixed; #3 premise probe already green.