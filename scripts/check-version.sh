#!/bin/sh
# Fails unless Cargo.toml's version is a semantic version higher than the one on the base branch:
# every pull request bumps it (major for a breaking change, minor for a feature, patch for a fix).
#   scripts/check-version.sh [base-ref]   (default origin/main)
set -eu
cd "$(dirname "$0")/.."
base_ref="${1:-origin/main}"
version_of() { sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1; }
head_version=$(version_of < Cargo.toml)
base_version=$(git show "$base_ref:Cargo.toml" | version_of)
case "$head_version" in
  [0-9]*.[0-9]*.[0-9]*) ;;
  *) echo "Cargo.toml's version '$head_version' is not MAJOR.MINOR.PATCH" >&2; exit 1 ;;
esac
if [ "$head_version" = "$base_version" ] || [ "$(printf '%s\n%s\n' "$base_version" "$head_version" | sort -V | tail -n 1)" != "$head_version" ]; then
  echo "Bump the version in Cargo.toml (and run cargo update -p cityloom_editor): $head_version is not above $base_ref's $base_version" >&2
  exit 1
fi
echo "version $base_version -> $head_version"
