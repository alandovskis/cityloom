---
id: getting-started
title: Getting started
sidebar_position: 2
---

# Getting started

This page is for developers. The repository uses [`just`](https://just.systems/) to run its tasks; run `just` with no
argument to list them.

## What you need

- A Rust toolchain with the `wasm32-unknown-unknown` target.
- Node.js and npm.
- `just`.

`just setup` installs the rest, once:

```sh
just setup
```

It runs `npm ci` at the root (the map's runtime libraries, MapLibre and PMTiles) and in `e2e` (Playwright), installs
`wasm-bindgen-cli` at the version pinned in `Cargo.toml` into `.tools`, installs Playwright's Chromium, and runs
`npm ci` in `website` (this documentation).

## Build and serve

```sh
just build   # release wasm of both modules, then wasm-bindgen into web/pkg
just serve   # http://127.0.0.1:8137/
```

`just build` runs `scripts/build.sh`. It builds the editor and the OpenStreetMap reader as two WebAssembly modules,
turns them into JavaScript glue with `wasm-bindgen` in `web/pkg` (not kept in git), and copies the map's libraries into
`web/vendor`.

`just serve` runs `node e2e/serve.mjs`. It answers range requests, which the basemap's PMTiles need. Set `E2E_PORT` to
use another port.

## Map data

```sh
just prepare
```

This downloads Geofabrik's Quebec extract into `data/` if it is not there (about 1.2 GB; `just prepare --refresh` takes
a newer one). It then cuts the metropolitan area's roads into tiles in `web/data/metro` (needs `osmium-tool`) and builds
the map page's basemap, `web/data/basemap/montreal.pmtiles`, with Planetiler (needs Java 21 or later). Both outputs are
ignored by git.

Nothing needs the road tiles: without them every place, the default one included, comes from Overpass. Without the
basemap, the map page says so and shows no map.

## Tests and formatting

```sh
just test          # cargo test --workspace, and a wasm32 build of the code plain `cargo test` does not compile
just e2e           # builds, then runs the Playwright tests in e2e/
just check         # both
just format        # rustfmt and Prettier
just format-check  # the same, without changing anything
```

CI runs `just format-check`, `just test` and `just e2e` with `RUSTFLAGS=-D warnings`, so a new compiler warning fails
it. Every pull request raises the version in the root `Cargo.toml`.

## This documentation

```sh
just docs        # serve the site with live reload
just docs-build  # build both languages into website/build
```

The English pages are in `website/docs/` and the French ones in
`website/i18n/fr-CA/docusaurus-plugin-content-docs/current/`. A page exists in both languages.
