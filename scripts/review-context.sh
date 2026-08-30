#!/usr/bin/env bash
# review-context.sh — assembles the review package deterministically.
set -uo pipefail
base=$(git merge-base main HEAD)
echo "== diff vs main (stat) =="
git diff --stat "$base"
echo "== diff (code, no lockfiles) =="
git diff "$base" -- ':!*.lock' ':!package-lock.json' | head -n 400
echo "== task in progress =="
grep -l '^status: doing' docs/tasks/*.md | xargs -r cat
