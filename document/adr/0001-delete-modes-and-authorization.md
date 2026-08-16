# ADR-0001: Delete modes as first-class Cedar actions, recycler as sink

## Status
Accepted

## Context

The system deletes resources through `DeleteMode` (`Transfer` | `Hard`), a
request-level parameter. Today both modes are authorized under the SAME Cedar
action (`Article::Delete` / `Comment::Delete`), so the authorization layer
cannot distinguish "soft delete (transfer ownership to a recycler)" from "hard
delete (cascade remove)". We need to express per-mode permissions and the
invariant "recycler-owned content can never be transferred".

A concrete bug surfaced: single-asset transfer calls
`pick_recycler_target(db, &[])` with an EMPTY exclude (transfer.rs:107), so when
the acting user holds the recycler role it can pick itself -> the ownership edge
is rewired from the user back to the same user -> a silent no-op that the API
reports as success. The seed grants `ROLE_RECYCLER` to the bootstrap `user_zero`
(seed.rs:22-24), so the admin account is itself a recycler.

A second bug: `transfer_target_ownership` returns `Ok(())` when the target has no
owner edge (transfer.rs:128-130), a silent success indistinguishable from a real
transfer.

## Decision

1. **All delete actions are split by mode.** Every resource type gets
   `Delete::{Hard, Transfer, Soft}` actions, including Version and User (some
   variants are future-facing placeholders). Cedar action names are opaque
   strings: there is NO hierarchy, granting `Delete::Hard` does not grant
   `Delete::Transfer`; each must be seeded and granted explicitly.

2. **Authorization is entirely Cedar's job.** Backend logic performs plain
   CRUD and calls `authorize`/`authorize_or` with the specific per-mode action;
   it does not re-implement authorization rules.

3. **Published content is a platform asset: an owner may only soft-delete
   (Transfer) their own content, never Hard-delete it.** The owner-bypass policy
   enumerates only the Transfer variants for Article/Comment delete. Hard delete
   is admin-only (admin override + admin holds all `Delete::Hard`).

4. **Recycler is a terminal sink: it can never transfer.** Principal-side Cedar
   forbid (option A, not resource-side):
   ```
   forbid (principal in Role::"recycler",
           action in [Action::"Article::Delete::Transfer",
                      Action::"Comment::Delete::Transfer"],
           resource);
   ```
   Chosen over the resource-side form (`resource.owner in Role::"recycler"`)
   because the resource assembly does not currently include the owner user
   entity with its role memberships (authorization.rs:196-272), so that
   expression cannot be evaluated without extra assembly work.

5. **Grant matrix:** admin holds all Hard + Transfer; member holds only
   Article/Comment Transfer; recycler holds no Transfer.

6. **Dev database is reset** (system not yet live): delete `data/agdb` and
   re-seed. No migration script.

## Consequences

- The transfer-to-self no-op is closed: a recycler (including admin user_zero)
  can no longer initiate transfer. The admin's moderation path becomes **Hard
  delete** (a separate, unaffected action).
- Because user_zero is BOTH admin and recycler, admin loses the ability to
  transfer/soft-delete any content (accepted trade-off). Admin and recycler
  roles should be separated if admin transfer is ever wanted again.
- The owner-bypass policy (policy.cedar) must enumerate every `Delete::*`
  variant; this list grows each time a mode (`::Soft`) is added. Cedar action
  sets have no wildcard.
- Introducing `::Soft` later is cheap on the authorization side (add action +
  permission + owner-bypass enumeration), but the real cost is read-path
  filtering (exclude soft-deleted nodes), restore, comment-subtree consistency,
  search-index sync, and PDF linkage -- NOT the Cedar action split.
- `is_author` (authorize.rs:104, currently uses `PERMISSION_COMMENT_DELETE`)
  must switch to a "may transfer" check after the split.

## Follow-up (resolved 2026-08-16)

- `pick_recycler_target`: for single-asset transfer, exclude the current owner
  of the target (defense in depth).
- `transfer_target_ownership`: stop returning `Ok(())` when the owner edge is
  absent; return an explicit, observable result (`TargetOwnerMissing`) instead
  of a silent success.