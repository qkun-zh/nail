# nail

Versioned-article knowledge base: authors publish versioned revisions, attach
notes/comments, tag, and search. Access via email-challenge auth with PoW,
Cedar authorization, PDF download with short-lived tokens.

## Features

- Article versioning with full revision history
- Notes and comments on articles
- Tagging and full-text search (SeekStorm)
- Email-challenge auth hardened with proof-of-work login
- Cedar policy-based authorization
- PDF export via short-lived tokens

## Tech Stack

- **Frontend**: Leptos CSR (via trunk)
- **Proxy**: pingap (static files + reverse proxy for `/api/*`)
- **Backend**: axum, agdb, SeekStorm, moka cache, cedar-policy, lettre, tokio
- **IDs/tokens**: UUIDv7; **hashing**: ascon

## Project Layout

- `code/server/` — backend (`interface → logic → repository → infrastructure`)
- `code/client/` — Leptos frontend (`router → page → request`)
- `code/common/` — shared structures (hash, PoW, tags, search, time)
- `configuration/` — toml configs (`server.toml`, `front.toml`, `email.toml`)
- `document/` — workflow docs and task records

## Building & Running

```sh
# backend (from code/server)
cargo run --bin server              # seed samples: -- seed-samples [count]

# frontend (from code/client)
trunk build                         # served through the proxy
```

Backend serves `/config/read` for the frontend. See `document/workflow.md`
(Running the stack) for the full-stack restart procedure.

## Testing

`cargo test` per crate; `cargo clippy` (zero warnings) and `cargo fmt` clean.

## For AI Agents

If you are an AI coding agent working in this repo, start with `AGENTS.md`
(project constitution and workflow), not this file.

## License

MIT — see [LICENSE](LICENSE).
