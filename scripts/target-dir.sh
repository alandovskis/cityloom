#!/bin/sh
# Prints cargo's target directory. It is ./target unless the machine moves it (build.target-dir in a .cargo/config.toml,
# or CARGO_TARGET_DIR), so scripts and tests that need a built binary ask here instead of assuming ./target.
# Callers must take the answer into a variable of its own (`dir=$(./scripts/target-dir.sh)`) so that a failure stops them:
# the status of a command substitution inside another command's arguments is lost.
set -eu
cd "$(dirname "$0")/.."
command -v cargo >/dev/null || { echo "target-dir.sh: cargo is not installed" >&2; exit 1; }
command -v node >/dev/null || { echo "target-dir.sh: node is not installed" >&2; exit 1; }
meta=$(cargo metadata --format-version 1 --no-deps) || { echo "target-dir.sh: cargo metadata failed" >&2; exit 1; }
printf '%s' "$meta" | node -p 'JSON.parse(require("fs").readFileSync(0, "utf8")).target_directory'
