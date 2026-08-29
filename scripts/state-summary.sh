#!/usr/bin/env bash
# Session opener. Prints, in under a screenful: where the project stands, what
# the last session left behind, and what is ready to pick up. CLAUDE.md tells
# every agent to run this before anything else, so it must stay fast and
# read-only.

set -euo pipefail

cd "$(dirname "$0")/.."

FEATURES="docs/state/features.md"
HANDOFF="docs/state/handoff.md"
TASKS="docs/tasks"

bold() { printf '\033[1m%s\033[0m\n' "$*"; }
dim()  { printf '\033[2m%s\033[0m\n' "$*"; }

bold "CityLoom — state summary"
echo

# --- Features ---------------------------------------------------------------

if [[ -f "$FEATURES" ]]; then
    total=$(grep -c '^- \[' "$FEATURES" || true)
    passing=$(grep -c '^- \[PASSING\]' "$FEATURES" || true)
    failing=$(grep -c '^- \[FAILING\]' "$FEATURES" || true)
    blocked=$(grep -c '^- \[BLOCKED' "$FEATURES" || true)

    pct=0
    [[ "$total" -gt 0 ]] && pct=$(( passing * 100 / total ))

    bold "Features"
    printf '  %s/%s passing (%s%%)   %s failing   %s blocked\n' \
        "$passing" "$total" "$pct" "$failing" "$blocked"

    if [[ "$blocked" -gt 0 ]]; then
        grep '^- \[BLOCKED' "$FEATURES" | sed 's/^/  /' | head -n 5
    fi
else
    bold "Features"; echo "  (missing $FEATURES)"
fi
echo

# --- Tasks ------------------------------------------------------------------

bold "Tasks"
if [[ -d "$TASKS" ]] && compgen -G "$TASKS/*.md" >/dev/null; then
    for state in done in_progress todo blocked; do
        # grep exits 1 on no match, which pipefail would turn into a script
        # abort; a zero count is a normal result here.
        n=$( { grep -l "^status: $state$" "$TASKS"/*.md 2>/dev/null || true; } | wc -l | tr -d ' ')
        printf '  %-12s %s\n' "$state" "$n"
    done

    if grep -l '^status: in_progress$' "$TASKS"/*.md >/dev/null 2>&1; then
        echo
        dim "  in progress:"
        grep -l '^status: in_progress$' "$TASKS"/*.md | while read -r f; do
            title=$(grep -m1 '^title:' "$f" | sed 's/^title: *//; s/^"//; s/"$//')
            printf '    %s  %s\n' "$(basename "$f" .md)" "$title"
        done
    fi
else
    echo "  (no task files)"
fi
echo

# --- Handoff ----------------------------------------------------------------

bold "Last session's handoff"
if [[ -f "$HANDOFF" ]]; then
    sed 's/^/  /' "$HANDOFF" | head -n 30
else
    echo "  (missing $HANDOFF)"
fi
echo

# --- Next -------------------------------------------------------------------

bold "Ready to start"
./scripts/next-tasks.sh | head -n 12
echo
dim "full list: ./scripts/next-tasks.sh   |   gate: just verify"
