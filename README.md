<img src="web/logo-wordmark.svg" alt="CityLoom" height="56">

# CityLoom

Streetmix at city scale: residents compose and compare whole-city scenarios.
This repository holds three surfaces: a city map (`map.html`), the street cross-section editor (`street.html`) and the intersection editor (`intersection.html`). The home page finds a place by name; its streets come from OpenStreetMap (through Nominatim, Overpass and osm2streets) and become a city that the map opens the other two surfaces on, a street or junction at a time. What is edited is kept in browser storage. The Plateau Mont-Royal in Montréal comes with the app, as an OpenStreetMap PBF extract.

- `src/`: one folder per feature (a vertical slice), each holding its own model, view-model and views, plus a small kernel:
  - `street/`, `junction/`: the editors. `model.rs` has the editing rules (`junction/` also has `geometry.rs` and `read_model.rs`); `vm.rs` is the view-model; `text.rs` the words said to a screen reader; the rest are the Leptos views, with tests beside them.
  - `map/`: the city map (view-model, camera, gestures, views). `city/`: the city (the sample one, or one made from OpenStreetMap), how it is kept in storage (`store.rs`) and the binding a page writes back through (`binding.rs`). `shell/`: the settings menu, sidebars and notes tabs every page shares.
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
just serve    # http://127.0.0.1:8137/
just test     # cargo test, and a type-check of the wasm-only code
just e2e      # browser tests after a fresh build; arguments go to Playwright, e.g. `just e2e tests/street.spec.ts`
just check    # test, then e2e
just format   # rustfmt and Prettier (just format-check only checks)
```

Without `just`: `cargo test`, `./scripts/build.sh`, `python3 -m http.server 8137 --directory web`, and `cd e2e && npx playwright test`.

## End-to-end tests

Playwright drives the built pages in Chromium (`e2e/`). `just e2e` builds first; a small Node server (`e2e/serve.mjs`) serves `web/` for the run.

## Continuous integration

`.github/workflows/ci.yml` runs on pushes to `main` and on pull requests: formatting, the unit tests (with warnings denied), and the browser tests, each through `just`. A final `CI` job passes only if all of them did, so that is the one check to require in branch protection. A failed browser run keeps its Playwright report and traces as an artifact. Actions are pinned to a commit, and Dependabot proposes weekly updates for them, the Cargo dependencies and the browser tests' npm packages.

## Montréal metro tiles

The roads of the whole Montréal metropolitan area can be cut into tiles that are served with the pages, so that
a place there is read from a file of a few kilobytes instead of being asked of Overpass:

```sh
curl -LO https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf   # about 1.2 GB
brew install osmium-tool
just metro-tiles quebec-latest.osm.pbf   # about 3 minutes; 1,700 tiles, about 49 MB, in web/data/metro
```

The tiles are not kept in git; `web/data/metro/index.json` is a placeholder that names none, and the command above
overwrites it. Deploy the folder with the pages. Without tiles every place other than the default one comes from Overpass.
