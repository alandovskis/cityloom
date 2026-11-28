#!/bin/sh
# Prints cargo's target directory. It is ./target unless the machine moves it (build.target-dir in a .cargo/config.toml,
# or CARGO_TARGET_DIR), so scripts and tests that need a built binary ask here instead of assuming ./target.
# Callers must take the answer into a variable of its own (`dir=$(./scripts/target-dir.sh)`) so that a failure stops them:
# the status of a command substitution inside another command's arguments is lost.
# It needs cargo only, not Node: the Rust image (dockerfiles/rust.Dockerfile) has no Node.
set -eu
cd "$(dirname "$0")/.."
command -v cargo >/dev/null || { echo "target-dir.sh: cargo is not installed" >&2; exit 1; }
meta=$(cargo metadata --format-version 1 --no-deps) || { echo "target-dir.sh: cargo metadata failed" >&2; exit 1; }
# cargo writes the field as `"target_directory":"<path>"` in compact JSON. A backslash is an escape that this reading
# does not decode (a `"` or `\` in the path, or a Windows path), so it is refused rather than printed wrong.
dir=$(printf '%s' "$meta" | sed -n 's/.*"target_directory":"\([^"\\]*\)".*/\1/p')
[ -n "$dir" ] || { echo "target-dir.sh: no plain target_directory in cargo metadata (a path with \" or \\ is not supported)" >&2; exit 1; }
printf '%s\n' "$dir"
