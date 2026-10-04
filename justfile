# Common operations. `just` lists them.

wasm_bindgen := `sed -n 's/^wasm-bindgen = "\(.*\)"/\1/p' Cargo.toml`

[private]
default:
    @just --list

# Install what building and the browser tests need (once).
setup:
    cargo install wasm-bindgen-cli --version {{wasm_bindgen}} --root .tools
    cd e2e && npm ci && npx playwright install chromium

# Build the WebAssembly module into web/pkg.
build:
    ./scripts/build.sh

# Unit tests on the host, and a type-check of the wasm-only code that they do not compile.
test:
    cargo test --workspace
    cargo build --target wasm32-unknown-unknown --workspace

# Browser tests, after a fresh build. Arguments go to Playwright: `just e2e tests/street.spec.ts`.
e2e *args: build
    cd e2e && npx playwright test {{args}}

# Everything: unit tests, then the browser tests.
check: test e2e

# Format the Rust (rustfmt.toml) and the scripts (.prettierrc.json).
format:
    cargo fmt
    cd e2e && npx prettier --ignore-path ../.prettierignore --write . ../web/*.js

# Fail if anything is not formatted, without changing it.
format-check:
    cargo fmt --check
    cd e2e && npx prettier --ignore-path ../.prettierignore --check . ../web/*.js

# Remake the default area's data from an OSM XML extract: `just default-area osm_import/tests/data/plateau.osm`. Needs uv.
default-area extract:
    uv run --with osmium scripts/osm2pbf.py {{extract}} web/data/default.osm.pbf

# Cut the Montréal metropolitan area's roads into tiles in web/data/metro, from a Geofabrik Quebec extract. Needs osmium-tool.
metro-tiles quebec:
    ./scripts/metro_tiles.py {{quebec}}

# Serve the pages at http://127.0.0.1:8137/ (build first).
serve:
    python3 -m http.server 8137 --directory web

# Remove build output.
clean:
    cargo clean
    rm -rf web/pkg e2e/test-results e2e/playwright-report
