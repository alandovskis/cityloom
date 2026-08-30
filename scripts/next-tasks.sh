#!/usr/bin/env bash
# next-tasks.sh — lists READY tasks: status todo + all deps done.
set -uo pipefail
cd "$(dirname "$0")/../docs/tasks"

# done_ids: ids with status done, space-separated
done_ids=$(grep -l '^status: done' *.md | xargs -r grep -h '^id:' | awk '{print $2}' | tr '\n' ' ')

for f in *.md; do
  grep -q '^status: todo' "$f" || continue
  deps=$(grep '^depends_on:' "$f" | sed 's/.*\[\(.*\)\].*/\1/' | tr -d ',')
  ready=1
  for d in $deps; do
    echo " $done_ids " | grep -q " $d " || ready=0
  done
  [ "$ready" = "1" ] && \
    echo "READY: $(grep '^id:' "$f" | awk '{print $2}') | $(grep '^title:' "$f" | cut -d'"' -f2) | $(grep '^model_hint:' "$f" | awk '{print $2}')"
done
