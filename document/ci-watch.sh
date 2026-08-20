#!/usr/bin/env bash
set -euo pipefail

URL=$(git remote get-url origin)
TOKEN=${URL#https://}
TOKEN=${TOKEN%%@*}
REPO=${URL##*@}
REPO=${REPO#github.com:}
REPO=${REPO%.git}
BRANCH=$(git branch --show-current)

run_json() {
    curl -sf -H "Authorization: Bearer $TOKEN" \
        "https://api.github.com/repos/$REPO/actions/runs?branch=$BRANCH&per_page=1" \
        || exit 1
}

json() {
    python3 -c "import json,sys; d=json.load(sys.stdin); print($1)"
}

while true; do
    JSON=$(run_json)
    if [ "$(printf '%s' "$JSON" | json "len(d['workflow_runs'])")" -eq 0 ]; then
        echo "no workflow runs for branch $BRANCH yet; retrying in 10s..."
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
        curl -sf -H "Authorization: Bearer $TOKEN" \
            "https://api.github.com/repos/$REPO/actions/runs/$ID/jobs" \
            | python3 -c "
import json,sys
d = json.load(sys.stdin)
for j in d['jobs']:
    if j['conclusion'] not in (None, 'success', 'skipped'):
        print(f\"failed job: {j['name']}\")"
        exit 1
    fi
    sleep 15
done