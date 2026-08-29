#!/usr/bin/env bash
# Lists READY tasks: status is todo and every id in depends_on is done.
#
# Implemented in awk rather than bash associative arrays so it runs on the
# bash 3.2 that ships with macOS. Read-only.

set -euo pipefail

cd "$(dirname "$0")/.."

TASKS="docs/tasks"

if [[ ! -d "$TASKS" ]] || ! compgen -G "$TASKS/*.md" >/dev/null; then
    echo "  (no task files in $TASKS)"
    exit 0
fi

awk '
FNR == 1 {
    file = FILENAME
    files[++nfiles] = file
    infm = 0; fmdone = 0
    id[file] = ""; title[file] = ""; status[file] = ""
    deps[file] = ""; hint[file] = ""; feats[file] = ""
}

fmdone { next }

/^---[[:space:]]*$/ {
    if (!infm) { infm = 1; next }
    else       { fmdone = 1; next }
}

infm {
    line = $0
    sub(/[[:space:]]+$/, "", line)

    if (line ~ /^id:/)          { v = line; sub(/^id:[[:space:]]*/, "", v);          gsub(/"/, "", v); id[FILENAME] = v }
    else if (line ~ /^title:/)  { v = line; sub(/^title:[[:space:]]*/, "", v);       gsub(/"/, "", v); title[FILENAME] = v }
    else if (line ~ /^status:/) { v = line; sub(/^status:[[:space:]]*/, "", v);      gsub(/"/, "", v); status[FILENAME] = v }
    else if (line ~ /^model_hint:/) {
        v = line; sub(/^model_hint:[[:space:]]*/, "", v)
        sub(/[[:space:]]*#.*$/, "", v); gsub(/"/, "", v); hint[FILENAME] = v
    }
    else if (line ~ /^depends_on:/) {
        v = line; sub(/^depends_on:[[:space:]]*/, "", v)
        gsub(/[\[\]"]/, "", v); gsub(/[[:space:]]/, "", v); deps[FILENAME] = v
    }
    else if (line ~ /^features:/) {
        v = line; sub(/^features:[[:space:]]*/, "", v)
        gsub(/[\[\]"]/, "", v); feats[FILENAME] = v
    }
}

END {
    # id -> status, for dependency lookup
    for (i = 1; i <= nfiles; i++) {
        f = files[i]
        if (id[f] != "") statusof[id[f]] = status[f]
    }

    ready = 0
    for (i = 1; i <= nfiles; i++) {
        f = files[i]
        if (status[f] != "todo") continue

        blocked = 0; blockers = ""
        n = split(deps[f], d, ",")
        for (j = 1; j <= n; j++) {
            if (d[j] == "") continue
            if (statusof[d[j]] != "done") {
                blocked = 1
                blockers = blockers (blockers == "" ? "" : ",") d[j]
            }
        }
        if (blocked) { nblocked++; continue }

        ready++
        nf = split(feats[f], ff, ",")
        printf "  %-5s %-9s %s\n", id[f], "[" hint[f] "]", title[f]
        printf "        %s   %d features\n", "docs/tasks/" substr(f, index(f, "/tasks/") + 7), nf
    }

    if (ready == 0) {
        if (nblocked > 0) print "  (nothing ready — " nblocked " todo task(s) still blocked by dependencies)"
        else              print "  (no todo tasks remain)"
    }
}
' "$TASKS"/*.md
