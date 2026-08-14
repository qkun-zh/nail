#!/usr/bin/env python3
"""Stream the live activity of a running `dsh --profile headless` agent.

`headless` prints nothing until it exits, so it looks frozen. Its real activity
is written as zstd-compressed JSONL under ~/.dsh/sessions/. This script follows
the newest session file and prints the full, untruncated picture — the model's
reasoning, its text, every tool call, and every tool result — as it happens.

Usage:
    python3 document/tools/watch-session.py            # follow the newest session
    python3 document/tools/watch-session.py <glob>     # follow a specific session file

Requires the `zstandard` Python package (zstd CLI is not installed on this
machine and sudo needs a password):
    pip3 install --user --break-system-packages zstandard

Output is never truncated; the terminal scrollback grows, so clear it
periodically. The session files under ~/.dsh/sessions/ also accumulate and
should be pruned from time to time.
"""

import glob
import json
import os
import sys
import time


def _decoder():
    try:
        import zstandard
    except ImportError:
        sys.exit(
            "zstandard not found; install it first:\n"
            "  pip3 install --user --break-system-packages zstandard"
        )
    return zstandard.ZstdDecompressor()


def latest_session_file(pattern):
    files = glob.glob(pattern)
    if not files:
        return None
    return max(files, key=os.path.getmtime)


def read_all(path, decoder):
    try:
        with open(path, "rb") as fh:
            return decoder.stream_reader(fh).read().decode("utf-8", "replace")
    except Exception:
        return ""


def result_text(event):
    """Pull the human text out of a tool/result event."""
    try:
        blocks = event["data"]["message"]["content"]
        for block in blocks:
            for inner in block.get("content", []):
                if inner.get("type") == "text":
                    return inner.get("text", "")
    except Exception:
        pass
    return ""


def print_block(prefix, text):
    if text is None:
        return
    text = str(text).rstrip("\n")
    for line in text.splitlines() or [""]:
        print(f"{prefix} {line}", flush=True)


def render_message(event):
    data = event.get("data", {})
    step = data.get("step", "?")
    print(f"\n=== step {step} ===  (seq {event.get('seq')})", flush=True)
    for block in data.get("message", {}).get("content", []):
        kind = block.get("type")
        if kind == "reasoning":
            print_block("  [think]", block.get("text"))
        elif kind == "text" and block.get("text", "").strip():
            print_block("  [text]", block["text"])
        elif kind in ("tool-call", "tool_use"):
            args = block.get("arguments", block.get("input", ""))
            if isinstance(args, (dict, list)):
                args = json.dumps(args, indent=2)
            print_block("  [call]", f"{block.get('name', '?')} {args}")


def main():
    pattern = (
        sys.argv[1]
        if len(sys.argv) > 1
        else os.path.expanduser("~/.dsh/sessions/*/*/session.jsonl.zstd")
    )
    decoder = _decoder()
    last_seq = -1
    current = None
    print("watching dsh session ... (Ctrl-C to stop)", flush=True)
    while True:
        path = latest_session_file(pattern)
        if path and path != current:
            current = path
            last_seq = -1
            print("=== session:", os.path.basename(os.path.dirname(path)), flush=True)
        if current:
            for line in read_all(current, decoder).splitlines():
                try:
                    event = json.loads(line)
                except Exception:
                    continue
                seq = event.get("seq", 0)
                if seq <= last_seq:
                    continue
                last_seq = seq
                kind = event.get("type", "?")
                data = event.get("data", {})
                if kind == "assistant/message":
                    render_message(event)
                elif kind == "tool/result":
                    print_block("  [result]", result_text(event))
                elif kind in ("llm/retry", "llm/retry-started"):
                    print_block("  !!", f"{kind} {json.dumps(data, indent=2)}")
        time.sleep(1)


if __name__ == "__main__":
    main()
