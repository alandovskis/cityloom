#!/usr/bin/env bash
# The single gate (team.md, "there is no second human"): run to a clean exit
# before anything is called done. Every step below is a merge-gate item of
# team.md's Testing Posture; none may be weakened to make a step pass.
set -euo pipefail

cd "$(dirname "$0")/.."

step() { printf '\n== verify: %s ==\n' "$1"; }

step "cargo fmt --all -- --check"
cargo fmt --all -- --check

step "cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

step "cargo build --workspace --locked"
cargo build --workspace --locked

step "cargo test --workspace"
cargo test --workspace

step "coverage floor (80% lines on the measured set; ignore pattern is the committed one)"
cargo llvm-cov -p osm-extract-proxy -p cityloom-api-types \
  --fail-under-lines 80 \
  --ignore-filename-regex '(src/bin/|/out/|proto_gen)'

step "dependency and asset manifest (docs/dependencies.md)"
manifest="docs/dependencies.md"
[ -f "$manifest" ] || { echo "verify: $manifest is missing"; exit 1; }

# No Streetmix package anywhere: not in the lockfile, not in the manifest
# (project.md, Forbidden; team.md, Code Style).
if grep -qi 'streetmix' Cargo.lock "$manifest"; then
  echo "verify: a Streetmix reference appears in Cargo.lock or $manifest"
  exit 1
fi

# Every direct dependency of every workspace crate (normal, dev, build) must
# have a row in the manifest's table, spelled as a backticked crate name.
missing=0
for toml in crates/*/Cargo.toml; do
  while IFS= read -r dep; do
    [ -z "$dep" ] && continue
    if ! grep -q "^| \`$dep\`" "$manifest"; then
      echo "verify: direct dependency '$dep' ($toml) has no row in $manifest"
      missing=1
    fi
  done < <(awk '
    /^\[/ { insec = ($0 ~ /^\[(dependencies|dev-dependencies|build-dependencies)\]$/);
            if ($0 ~ /^\[(dependencies|dev-dependencies|build-dependencies)\.[^]]+\]$/) {
              sub(/^\[[a-z-]+\./, ""); sub(/\]$/, ""); print; }
            next }
    insec && /^[A-Za-z0-9_-]+ *=/ { split($0, a, /[ =]/); print a[1] }
  ' "$toml" | sort -u)
done
# The vendored .proto files are assets and must be recorded too.
for asset in fileformat.proto osmformat.proto; do
  if ! grep -q "\`$asset\`" "$manifest"; then
    echo "verify: vendored asset '$asset' has no row in $manifest"
    missing=1
  fi
done
[ "$missing" -eq 0 ] || exit 1

printf '\n== verify: OK ==\n'
