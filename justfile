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
    cargo test
    cargo build --target wasm32-unknown-unknown

# Browser tests, after a fresh build. Arguments go to Playwright: `just e2e tests/street.spec.ts`.
e2e *args: build
    cd e2e && npx playwright test {{args}}

# Everything: unit tests, then the browser tests.
check: test e2e

# Serve the pages at http://127.0.0.1:8137/ (build first).
serve:
    python3 -m http.server 8137 --directory web

# Remove build output.
clean:
    cargo clean
    rm -rf web/pkg e2e/test-results e2e/playwright-report
