# CityLoom — one spelling for every command CLAUDE.md names.
# `just verify` is the single gate: exit 0 means the work is done.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Build every crate from the committed lockfile.
build:
    cargo build --workspace --locked

# Format every crate in place.
format:
    cargo fmt --all

# Run every test in the workspace.
test:
    cargo test --workspace

# Line coverage on the measured set with the 80% floor (team.md Testing Posture).
cov:
    cargo llvm-cov -p osm-extract-proxy -p cityloom-api-types --fail-under-lines 80 --ignore-filename-regex '(src/bin/|/out/|proto_gen)'

# The single gate: fmt, clippy -D warnings, build --locked, tests, coverage floor, dependency manifest.
verify:
    ./scripts/verify.sh

# Build a tiny synthetic data build under target/data (for running the service locally).
synthetic:
    cargo run --bin region-build -- --synthetic --out target/data

# Run the service locally against target/data (run `just synthetic` first).
serve:
    CITYLOOM_STORE_PATH=target/data/store.bin CITYLOOM_MANIFEST_PATH=target/data/manifest.json CITYLOOM_BUILD_ID=local PORT=8080 cargo run --bin osm-extract-proxy
