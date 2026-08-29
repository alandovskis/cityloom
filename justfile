# CityLoom task runner. `just verify` is the only gate that decides "done".

set shell := ["bash", "-uc"]

_default:
    @just --list --unsorted

# ---------------------------------------------------------------------------
# The gate
# ---------------------------------------------------------------------------

# Single gate: format, lint, build (native + wasm), and test. Exit 0 = done.
verify:
    @./scripts/verify.sh

# The gate including integration and e2e tests. Needs Postgres up.
verify-full:
    @CITYLOOM_FULL=1 ./scripts/verify.sh

# ---------------------------------------------------------------------------
# Session helpers
# ---------------------------------------------------------------------------

# Where the project stands. Run this first, every session.
state:
    @./scripts/state-summary.sh

# Tasks whose dependencies are all done.
next:
    @./scripts/next-tasks.sh

# Per-feature-id coverage report (task 047).
features:
    @uv run --frozen python scripts/feature_coverage.py

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

# One-time toolchain setup. Idempotent.
setup:
    rustup target add wasm32-unknown-unknown
    cargo install wasm-pack --locked
    uv sync
    uv run --frozen playwright install chromium

# Debug build of server and wasm client.
build: build-server build-wasm

build-server:
    cargo build --workspace

build-wasm:
    wasm-pack build crates/cityloom-client --target web --out-dir ../../web/pkg --dev

# Release build of everything.
release:
    cargo build --workspace --release
    wasm-pack build crates/cityloom-client --target web --out-dir ../../web/pkg --release

# Rewrite formatting in place.
format:
    cargo fmt --all
    uv run --frozen ruff format .
    uv run --frozen ruff check --fix .

# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------

# Start Postgres in the background.
db:
    docker compose up -d db

# Apply migrations to the configured database.
migrate:
    cargo run -p cityloom-server --bin migrate

# Serve the app locally.
serve: build
    cargo run -p cityloom-server

# ---------------------------------------------------------------------------
# Test slices (verify runs all of these; these are for tightening the loop)
# ---------------------------------------------------------------------------

test-rust:
    cargo nextest run --workspace

test-int:
    uv run --frozen pytest -m integration

test-e2e:
    uv run --frozen pytest -m e2e

clean:
    cargo clean
    rm -rf web/pkg .pytest_cache
