<img src="web/logo-wordmark.svg" alt="CityLoom" height="56">

# CityLoom

Streetmix at city scale: residents compose and compare whole-city scenarios.
This repository holds three surfaces: a city map (`map.html`), the street cross-section editor (`street.html`) and the intersection editor (`intersection.html`). The home page finds a place by name; its streets come from OpenStreetMap (found through Nominatim, read from network tiles that osm2streets made) and become a city that the map opens the other two surfaces on, a street or junction at a time. What is edited is kept in browser storage.

- `crates/`: every crate. `crates/cityloom_editor/src/`: one folder per feature (a vertical slice), each holding its own model, view-model and views, plus a small kernel:
  - `street/`, `junction/`: the editors. `model.rs` has the editing rules (`junction/` also has `geometry.rs` and `read_model.rs`); `vm.rs` is the view-model; `text.rs` the words said to a screen reader; the rest are the Leptos views, with tests beside them.
  - `map/`: the city map (view-model, the overlay and its projection onto the map, views). `city/`: the city (made from OpenStreetMap), how it is kept in storage (`store.rs`) and the binding a page writes back through (`binding.rs`). `shell/`: the settings menu, sidebars and notes tabs every page shares.
  - `shared/`: what every slice uses: the catalogue and units, the ports a view-model reaches the browser through (`ports.rs`, with test fakes) and their browser side (`platform.rs`), the reactive `core.rs`, the `keeper.rs` that debounces writes, and the helpers views bind with.
- A view-model has no page in it: it owns the model, exposes properties that are ready to show (words, flags, lists) and takes commands, and reaches the browser only through the ports, so it is tested on the host with fakes. A view (Leptos, client-side rendered) binds to those properties and sends commands; it decides nothing. Components are drawn to HTML in tests; `shell/view.rs` binds the markup the pages share and is checked in the browser.
- A slice reaches another only through that slice's model (or `city`'s store and binding, or `shell`'s `Target`), never its view-model or views; `shared` depends on no slice.
- `web/`: the pages' markup and a few lines of start-up per page that load the module and call `mount_*`; no logic.
- All catalogue widths, rules and capacity rates are synthetic placeholders.

## Build and run

[`just`](https://just.systems) wraps the common operations (`just` lists them):

```sh
just setup    # once: wasm-bindgen-cli at the version Cargo.toml pins (into .tools), and the browser tests' dependencies
just build    # the WebAssembly module, into web/pkg
just serve    # http://127.0.0.1:8137/ (cargo run -p cityloom_server)
just db       # a throwaway Postgres in Docker, for the server's tests and the account spec
just test     # cargo test, and a type-check of the wasm-only code
just e2e      # all three tiers of browser tests after a fresh build; arguments run just those specs, e.g. `just e2e tests/street.spec.ts`
just e2e-ui           # one tier alone: UI (no database, no archive)
just e2e-integration  # the real server and a Postgres (`DATABASE_URL`)
just e2e-archive      # the unpacked archive and the delivery specs
just check    # test, then e2e
just format   # rustfmt and Prettier (just format-check only checks)
```

Without `just`: `cargo test --workspace --features cityloom_server/test-provider` (the server's tests need a Postgres: `just db` starts one and prints the `DATABASE_URL` to export; `just test` checks both), `./scripts/build.sh`, `cargo run -p cityloom_server`, and `cargo build -p cityloom_server --features test-provider && ./scripts/pin-server.sh` then `cd e2e && npx playwright test` (the tests start the pinned copy of the server).

The server's tests and the account spec need a Postgres: `just db`, then `export DATABASE_URL=...` as it prints.

## End-to-end tests

Playwright drives the built pages in Chromium (`e2e/`). `just e2e` builds first; the Rust server (`cityloom_server`) serves `web/` for the run (range requests included), with the fake sign-in provider and, for the integration tier, a Postgres; the specs are in three tiers (`e2e/tiers.json`), each its own CI job; `E2E_PORT` moves it off port 8137 when something else holds that.

## Continuous integration

`.github/workflows/ci.yml` runs on pushes to `main` and on pull requests: formatting, the unit tests (with warnings denied), and the browser tests, each through `just`. A final `CI` job passes only if all of them did, so that is the one check to require in branch protection. A failed browser run keeps its Playwright report and traces as an artifact. Actions are pinned to a commit, and Dependabot proposes weekly updates for them, the Cargo dependencies and the browser tests' npm packages.

## The Montréal data

One command makes the road tiles below. It first downloads Geofabrik's Quebec extract (about 1.2 GB) into `data/` unless it is there already; `just prepare --refresh` downloads a newer one, and an interrupted download carries on where it stopped when run again:

```sh
brew install osmium-tool   # osmium keeps the roads the tiles are made of
just prepare
```

It also imports them and cuts the network into tiles, and packs the tiles into one archive, `data/montreal.citymodel` (a tar with a SHA-256 index); `just pack` packs them again without running the rest.

## Montréal metro tiles

The roads of the whole Montréal metropolitan area are imported once, with osm2streets, and the network is cut into JSON tiles that are served with the pages (1,718 tiles, about 570 MB of JSON, in `web/data/metro`; the server compresses them on the fly), so that
a place there is read from the tile that holds it and cut out of it in the browser.

The tiles are not kept in git; `web/data/metro/index.json` is a placeholder that names none, and the command above
overwrites it. Deploy the folder with the pages. Without tiles no place opens: a place no tile holds is reported as not covered.
