# CityLoom

A Rust/WebAssembly street cross-section editor at city scale. This
repository is a Cargo workspace; `osm-extract-proxy` (Unit U9) is the first
application code in it — a server-side proxy that serves OpenStreetMap
extract clips, cut on demand from regional data built offline once a week.
The WASM client and the osm2streets adapter are separate, later Units.

## Layout

```
Cargo.toml, rust-toolchain.toml, rustfmt.toml   workspace configuration
justfile, scripts/verify.sh                     the one gate: `just verify`
scripts/state-summary.sh, scripts/next-tasks.sh session-start helpers (CLAUDE.md)
docs/dependencies.md                            the dependency and asset manifest
docs/deploy.md                                  Railway runbook and service variables
Dockerfile, railway.json                        the proxy's deploy shape
data/regions.toml                               configured source regions (region-build input)
data/current-build.toml                         the data pointer the Dockerfile reads
.github/workflows/data-build.yml                the weekly regional-data build
crates/cityloom-api-types/                      Contract 1 wire types (serde), owned by U9
crates/osm-extract-proxy/                       the proxy: service binary + region-build tool
```

`crates/osm-extract-proxy/src/lib.rs` documents the module rings (pure /
I/O / outer) in more detail; `docs/dependencies.md` records the origin and
licence of every direct dependency and vendored asset.

## The gate

```
just verify
```

runs, in order: `cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo build --workspace --locked`, `cargo
test --workspace`, the 80%-line-coverage floor (`cargo llvm-cov`, measured
set: `crates/osm-extract-proxy/src`, `crates/cityloom-api-types/src`,
excluding the two binary entry points and the generated protobuf module),
and the dependency-manifest / no-Streetmix check. Exit 0 means the work is
done (`team.md`).

Other `just` targets: `build`, `format`, `test`, `cov`, `synthetic` (builds
a tiny in-memory regional data set under `target/data`), `serve` (runs the
service against it).

## Running the proxy locally against a synthetic build

No network access and no real OSM data needed:

```
just synthetic   # writes target/data/{store.bin,manifest.json}
just serve       # serves on :8080 against that data
```

or directly:

```
cargo run --bin region-build -- --synthetic --out target/data
CITYLOOM_STORE_PATH=target/data/store.bin \
CITYLOOM_MANIFEST_PATH=target/data/manifest.json \
CITYLOOM_BUILD_ID=local PORT=8080 \
cargo run --bin osm-extract-proxy
```

Then, in another shell:

```
curl http://localhost:8080/readyz
curl -H 'x-client-build: local' \
  'http://localhost:8080/api/extract?bbox=-79.64,43.64,-79.62,43.66' -o extract.pbf
```

## The real weekly data build

`region-build --config data/regions.toml --out <dir>` downloads every
configured region and its published MD5 checksum, verifies it, filters to
what osm2streets reads (ways carrying a `highway` tag and the nodes they
reference), slices into the 0.01° grid, and publishes `store.bin` and
`manifest.json`. `.github/workflows/data-build.yml` runs this weekly (and
on demand), publishes a GitHub Release named after the content-derived
`buildId` when it changes, and commits the pointer
(`data/current-build.toml`) the `Dockerfile`'s data stage reads at image
build time.

## Testing

TDD (`team.md` Testing Posture): every module's tests were written before
its implementation. `crates/osm-extract-proxy/tests/bench.rs` is the one
`#[ignore]`d exception — a release-mode latency benchmark the slow CI tier
runs explicitly. See `unit-test-instructions.md` in this Unit's
construction record for the full command reference and the TDD cycle log.
