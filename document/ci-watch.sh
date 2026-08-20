#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-wait}"   # wait (block until done) | once | bg (background)
TIMEOUT="${2:-0}"   # seconds to wait before giving up (0 = forever)
LOG="${CI_WATCH_LOG:-/tmp/ci-watch.log}"

if [ "$MODE" = "bg" ]; then
    nohup bash "$0" wait "$TIMEOUT" >"$LOG" 2>&1 < /dev/null &
    echo "ci-watch running in background (pid $!); poll with: tail -f $LOG"
    exit 0
fi

URL=$(git remote get-url origin)
TOKEN=${URL#https://}
TOKEN=${TOKEN%%@*}
REPO=${URL##*@}
REPO=${REPO#github.com:}
REPO=${REPO#github.com/}
REPO=${REPO%.git}
BRANCH=$(git branch --show-current)

run_json() {
    curl -sf --max-time 30 -H "Authorization: Bearer $TOKEN" \
        "https://api.github.com/repos/$REPO/actions/runs?branch=$BRANCH&per_page=1" \
        || exit 1
}

jobs_json() {
    curl -sf --max-time 30 -H "Authorization: Bearer $TOKEN" \
        "https://api.github.com/repos/$REPO/actions/runs/$1/jobs"
}

json() {
    python3 -c "import json,sys; d=json.load(sys.stdin); print($1)"
}

START=$(date +%s)
while true; do
    JSON=$(run_json)
    if [ "$(printf '%s' "$JSON" | json "len(d['workflow_runs'])")" -eq 0 ]; then
        echo "no workflow runs for branch $BRANCH yet"
        [ "$MODE" = "once" ] && exit 1
        sleep 10
        continue
    fi
    ID=$(printf '%s' "$JSON" | json "d['workflow_runs'][0]['id']")
    STATUS=$(printf '%s' "$JSON" | json "d['workflow_runs'][0]['status']")
    HEAD_SHA=$(printf '%s' "$JSON" | json "d['workflow_runs'][0]['head_sha'][:8]")
    echo "run #$ID ($HEAD_SHA) status: $STATUS"
    if [ "$STATUS" = "completed" ]; then
        CONCLUSION=$(printf '%s' "$JSON" | json "d['workflow_runs'][0]['conclusion']")
        echo "conclusion: $CONCLUSION"
        if [ "$CONCLUSION" = "success" ]; then
            exit 0
        fi
        jobs_json "$ID" | python3 -c "
import json,sys
d = json.load(sys.stdin)
for j in d['jobs']:
    if j['conclusion'] not in (None, 'success', 'skipped'):
        print(f\"failed job: {j['name']}\")"
        exit 1
    fi
    jobs_json "$ID" | python3 -c "
import json,sys
d = json.load(sys.stdin)
for j in d['jobs']:
    print(f\"  {j['name']}: {j['status']}\")" 2>/dev/null || true
    [ "$MODE" = "once" ] && exit 0
    if [ "$TIMEOUT" -gt 0 ] && [ $(( $(date +%s) - START )) -ge "$TIMEOUT" ]; then
        echo "timed out after ${TIMEOUT}s; run still $STATUS"
        exit 1
    fi
    sleep 15
done