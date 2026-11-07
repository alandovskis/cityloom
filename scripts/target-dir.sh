#!/bin/sh
# Prints cargo's target directory. It is ./target unless the machine moves it (build.target-dir in ~/.cargo/config.toml,
# or CARGO_TARGET_DIR), so scripts and tests that need a built binary ask here instead of assuming ./target.
set -eu
cd "$(dirname "$0")/.."
cargo metadata --format-version 1 --no-deps | node -p 'JSON.parse(require("fs").readFileSync(0, "utf8")).target_directory'
