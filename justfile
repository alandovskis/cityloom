# Common operations. `just` lists them.

wasm_bindgen := `sed -n 's/^wasm-bindgen = "\(.*\)"/\1/p' Cargo.toml`

[private]
default:
    @just --list

# Install what building and the browser tests need (once).
setup:
    npm ci
    cargo install wasm-bindgen-cli --version {{wasm_bindgen}} --root .tools
    cd e2e && npm ci && npx playwright install chromium

# Build the WebAssembly module into web/pkg.
build:
    ./scripts/build.sh

# Unit tests on the host, and a type-check of the wasm-only code that they do not compile.
test:
    @[ -n "${DATABASE_URL:-}" ] || { echo "DATABASE_URL is not set: the server's tests need a Postgres (run 'just db' and export the URL it prints)" >&2; exit 1; }
    cargo test --workspace --features cityloom_server/test-provider
    cargo build --target wasm32-unknown-unknown --workspace --exclude cityloom_server

# A throwaway Postgres for the server's tests, on 127.0.0.1 only (port 55432 unless given). Running it again starts the same container. It prints the DATABASE_URL to export; `just db-stop` removes it.
db port="55432":
    #!/usr/bin/env sh
    set -eu
    if docker container inspect cityloom-db >/dev/null 2>&1; then
        docker start cityloom-db >/dev/null
        port=$(docker port cityloom-db 5432/tcp | head -n 1 | sed 's/.*://')
    else
        docker run -d --name cityloom-db -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=cityloom -p 127.0.0.1:{{port}}:5432 postgres:18 >/dev/null
        port={{port}}
    fi
    echo "export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:$port/cityloom"

# Remove the Postgres that `just db` started.
db-stop:
    docker rm -f cityloom-db

# Browser tests, after a fresh build. Arguments go to Playwright: `just e2e tests/street.spec.ts`.
e2e *args: build
    cargo build -p cityloom_server
    cd e2e && npx playwright test {{args}}

# Everything: unit tests, then the browser tests.
check: test e2e

# Format the Rust (rustfmt.toml), the scripts and the documentation site (.prettierrc.json).
format:
    cargo fmt
    cd e2e && npx prettier --ignore-path ../.prettierignore --write . ../web/*.js ../website

# Fail if anything is not formatted, without changing it.
format-check:
    cargo fmt --check
    cd e2e && npx prettier --ignore-path ../.prettierignore --check . ../web/*.js ../website

# What the SonarCloud analysis reads, in target/sonar: the Rust coverage as LCOV (host tests only: the wasm-only code and the browser tests are not measured) and Clippy's findings as JSON. Sonar judges the findings, so lints are capped at warnings here; a build error still fails. Needs cargo-llvm-cov (`cargo install cargo-llvm-cov --locked`) and the llvm-tools component.
sonar-reports:
    mkdir -p target/sonar
    cargo llvm-cov --workspace --exclude cityloom_server --lcov --output-path target/sonar/lcov.info
    cargo clippy --workspace --message-format=json -- --cap-lints warn > target/sonar/clippy.json

# Make the Montréal metropolitan area's data: download Geofabrik's Quebec extract into data/ if it is not there, cut the roads into tiles in web/data/metro (needs osmium-tool) and build the basemap in web/data/basemap (needs Java 21+). `just prepare --refresh` downloads a newer extract.
prepare *args:
    ./scripts/prepare.py {{args}}

# Serve the pages at http://127.0.0.1:8137/ (build first). The server answers range requests, as the map's tiles need.
serve:
    cargo run -p cityloom_server

# Install the documentation site's packages (once; `just setup` does not).
setup-docs:
    cd website && npm ci

# Serve the documentation site (website/, English and fr-CA) with live reload at http://localhost:3000/. Run one language with `just docs --locale fr-CA`.
docs *args:
    cd website && npm start -- {{args}}

# Build the documentation site for both languages into website/build; fails on a broken link.
docs-build:
    cd website && npm run build

# Build, then upload the site to Railway (the cityloom project; needs the Railway CLI, logged in, and `just prepare`'s tiles).
deploy:
    ./scripts/deploy.sh

# Remove build output.
clean:
    cargo clean
    rm -rf web/pkg e2e/test-results e2e/playwright-report
