#!/usr/bin/env bash
# state-summary.sh — session-start state summary.
# Contract: <= 250 tokens; distills, never lists; stable across runs.
set -uo pipefail

echo "== cityloom: state =="

# git: one line
branch=$(git branch --show-current)
dirty=$(git status --porcelain | wc -l | tr -d ' ')
echo "git: $branch | $dirty uncommitted file(s) | last: $(git log -1 --format='%h %s' | cut -c1-60)"

# features: scoreboard + current milestone's FAILING
total=$(grep -c '^\- \[' docs/state/features.md)
passing=$(grep -c '^\- \[PASSING\]' docs/state/features.md || true)
echo "features: $passing/$total passing"
echo "failing in current milestone:"
awk '/^## /{sec=$0} /^\- \[FAILING\]/ && sec==m {print "  " substr($0,13)}' \
    m="## $(cat docs/state/current-milestone)" docs/state/features.md | head -n 8

# handoff: in full (the 30-line cap is enforced there, not here)
echo "-- handoff --"
cat docs/state/handoff.md

# quick verify: verdict only
echo "-- verify --"
./scripts/verify.sh --quick | tail -n 1
