# DeepSeek Harness CLI (`dsh`) — agent runner reference

How the migrating agents are launched. The owner drives each agent session
with `dsh --profile headless "<instruction>"`; this document records the
installed tool, its configuration, and the verification steps. Config state is
as of 2026-08-14; re-verify with `--dump-config` before relying on it.

## What it is

`@deepseek-ai/dsh` (Node.js launcher) boots a Harness profile — an ordered
stack of plugin bundles plus user config overlays. The `headless` profile runs
one job in a fresh persistent session, prints the final answer, and exits.

## Install (already done)

```bash
npm install -g @deepseek-ai/dsh     # binary at ~/.npm-global/bin/dsh
```

Environment: node v22, npm prefix `~/.npm-global`.

## Config location — the gotcha

`$DSH_HOME` defaults to **`~/.dsh`** (NOT `~/.deepseek-harness`). Files:

- `~/.dsh/cordis.patch.yml` — home-level config patch (the user overlay layer).
- `~/.dsh/profiles/` — profiles; `headless` auto-initializes from its bundled
  template on first use.
- `~/.dsh/sessions/` — session persistence (JSONL).

A patch written to `~/.deepseek-harness/` is silently ignored.

## Current configuration (2026-08-14)

`~/.dsh/cordis.patch.yml`:

```yaml
- id: agent-default-model
  config:
    provider: deepseek-official
    model: deepseek-v4-pro
- id: llm
  config:
    reasoningEffort: high
```

| Key | Value | Notes |
| --- | --- | --- |
| Model | `deepseek-v4-pro` ("ds pro") | The default is `deepseek-v4-flash`; verified against `GET /models`. |
| `reasoningEffort` | `high` | Adapter values `off \| high \| max`; omitted ⇒ `high`. `high`/`max` enable thinking and serialize as the official `reasoning_effort`; `off` disables thinking. |
| API key | env `DEEPSEEK_API_KEY` | Config carries only the env NAME, never a literal key. Set in `~/.bashrc`. Resolved via the credentials seam, then the environment. |
| baseURL | `https://api.deepseek.com` | `$DEEPSEEK_BASE_URL` overrides; the adapter default already matches. |

Provider route: `deepseek-official` (adapter `@deepseek-ai/dsh-llm-deepseek`).

## Commands

```bash
# one-shot task — cwd becomes the workspace root (run from nail_new!)
dsh --profile headless "<task>"

# inspect the merged config tree (includes your patch)
dsh --profile headless --dump-config
# ... without the user layer
dsh --profile headless --dump-default-config

# extra one-off overlay
dsh --profile headless --patch /path/to/extra.yml "<task>"

# manage a profile's plugins (forwards to pnpm)
dsh plugin --profile <name> <pnpm args>

# launcher help
dsh --help
```

## Usage notes

- **Workspace root = the directory you run from.** Launch from
  `/home/qkun/nail_new` so the agent's file access is scoped to the repo.
- New shells pick up the key from `~/.bashrc`; for a one-off run without it,
  prefix the command: `DEEPSEEK_API_KEY=<key> DEEPSEEK_BASE_URL=https://api.deepseek.com dsh --profile headless "<task>"`.
- Long instructions: pass the full self-contained instruction as the quoted
  argument (the agent has no conversation history, so include the read-first
  list and current state). Escape or avoid inner double quotes, or write the
  instruction to a file and paste its content.
- Interactive mode (`dsh chat`, `dsh doctor`) comes from a separate Python
  supplement package (`deepseek-harness-cli`) per the vendor docs; it is NOT
  installed or verified here.

## Verification

```bash
# 1. key + endpoint (free, no tokens): expect the model list
curl -s https://api.deepseek.com/models -H "Authorization: Bearer <KEY>"

# 2. config applied
dsh --profile headless --dump-config | grep -A3 agent-default-model

# 3. end-to-end (spends a little credit)
dsh --profile headless "Reply with exactly: harness-ok"
```

## Security

- The literal API key must never go into this repo (README §11: secrets stay
  out of version control). It lives in `~/.bashrc` and `~/.dsh/`, both outside
  the repository.
- The key has been shared in chat history and shell commands; rotate it in the
  DeepSeek console if that history is ever shared.
