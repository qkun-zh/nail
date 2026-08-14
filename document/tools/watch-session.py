#!/usr/bin/env python3
"""Stream the live activity of a running `dsh --profile headless` agent.

`headless` prints nothing until it exits, so it looks frozen. Its real activity
is written as zstd-compressed JSONL under ~/.dsh/sessions/. This script follows
the newest session file and prints each step and tool call as it happens.

Usage:
    python3 document/tools/watch-session.py            # follow the newest session
    python3 document/tools/watch-session.py <glob>     # follow a specific session file

Requires the `zstandard` Python package (zstd CLI is not installed on this
machine and sudo needs a password):
    pip3 install --user --break-system-packages zstandard
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


def summarize_call(event):
    data = event.get("data", {})
    name = data.get("name", "?")
    raw = data.get("arguments", "")
    try:
        args = json.dumps(json.loads(raw))[:140]
    except Exception:
        args = str(raw)[:140]
    return name, args


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
                if kind == "step/start":
                    print(f"--- step {data.get('step')} ---", flush=True)
                elif kind == "tool/call":
                    name, args = summarize_call(event)
                    print(f"  [{name}] {args}", flush=True)
                elif kind == "assistant":
                    for chunk in data.get("message", {}).get("content", []):
                        if chunk.get("type") == "text" and chunk.get("text", "").strip():
                            text = chunk["text"].strip().replace("\n", " ")
                            print(f"  AI> {text[:220]}", flush=True)
        time.sleep(1)


if __name__ == "__main__":
    main()
