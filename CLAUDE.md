# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

CityLoom is a street, junction and city-map editor written in Rust, compiled to WebAssembly and drawn with Leptos 0.8 (client-side rendering). The pages are `web/index.html` (home: find a place), `web/map.html`, `web/street.html` and `web/intersection.html`. The streets come from OpenStreetMap; the catalogue's rules, rates and thresholds are synthetic placeholders.

## Commands

`just` lists the recipes. The ones used most:

```sh
just test            # cargo test --workspace + `cargo build --target wasm32-unknown-unknown --workspace` (cfg(target_arch = "wasm32") code is not compiled by plain `cargo test`)
cargo test shell::vm # one module; a full test name also works
just build           # release wasm of both modules + wasm-bindgen into web/pkg (gitignored); scripts/build.sh does the work
just metro-tiles quebec-latest.osm.pbf   # cuts the metro area's roads into web/data/metro (gitignored; needs osmium-tool). Nothing needs them: without tiles every place, the default one too, comes from Overpass
just e2e             # builds, then runs the Playwright tests in e2e/; `just e2e tests/street.spec.ts` for one spec
just check           # everything
just format          # rustfmt (rustfmt.toml) + Prettier; `just format-check` only checks, as CI does
just serve           # http://127.0.0.1:8137/ (runs `node e2e/serve.mjs`: the basemap's PMTiles need range requests; E2E_PORT moves the port, for Playwright too)
just basemap-tiles quebec-latest.osm.pbf   # builds the map page's basemap, web/data/basemap/montreal.pmtiles (gitignored), with Planetiler (needs Java 21+). Without it the map page says so and shows no map
just setup           # once: wasm-bindgen-cli (version from Cargo.toml, into .tools), `npm ci` at the root and in `e2e`, Playwright's Chromium
```

CI (`.github/workflows/ci.yml`) runs `just format-check`, `just test` and `just e2e` with `RUSTFLAGS=-D warnings`, so a new compiler warning fails it; run `just format` before committing. There is no clippy configuration. `build.sh` strips the name and producers sections (without that the wasm is ~7.6 MB instead of ~2 MB). The `proc-macro-error2` future-incompat warning comes from Leptos dependencies and is harmless.

## Architecture

The code is organised by vertical slice, one folder per feature, and MVVM is used inside each slice:

- `street/`, `junction/`, `map/`, `shell/` each hold what that feature needs: `model.rs` (street, junction: the editing rules, no DOM, no signals, no I/O; edits return `bool` and a refusal carries a reason), `vm.rs` (the view-model), `text.rs` (the words said to a screen reader), the Leptos views, and `tests.rs`. `junction/` also has `geometry.rs`, `read_model.rs` and `frame.rs`.
- `map/` is the exception to the SVG rule below: the map page draws the city over a MapLibre basemap and no SVG. `projection.rs` places the layout on the earth (`Projection`, from the area's bounds, osm2streets' linear lon/lat mapping), `overlay.rs` turns the city into GeoJSON places (`hot` ids `s-<uid>` / `j-<uid>`) and `style.rs` holds the overlay's MapLibre layers; the view-model drives MapLibre through the `Mapper` port. `svg.rs` and `camera.rs` remain only for the home page's hero (`map/home.rs`). The sample city has no map: the page says there is no map to show (`NO_ROADS`).
- `city/` is the sample city (`model.rs`), how it is kept in storage (`store.rs`) and the `CityBinding` a page writes back through (`binding.rs`).
- `place/` is where a city comes from: `area.rs` (an `Area` kept under its position), `nominatim.rs` and `overpass.rs` (the requests and readers, pure), `loader.rs` (`Loader`: search, then fetch, import and keep an area's roads, through the ports), `vm.rs`/`view.rs` (`AreaVm`, the home page's place search) and the `prepare_city` export the page scripts await.
- `shared/` is the kernel: catalogue, units, atlas, symbols, the ports (`Announcer`, `Storage`, `Scheduler`, `Mapper` (with `MapEvent`, `NoMapper`, `FakeMapper`) and their fakes in `ports.rs`), `platform.rs` (their browser side; `BrowserMapper` wraps the JS adapter `web/basemap.js`), `core.rs` (`Core<M: Presents>`, the model plus a version signal plus a cached view; `view()` is tracked, `view_now()` is not, so use it in commands), `keeper.rs`, `shortcut.rs`, `bind.rs`, and `testing.rs` (test-only HTML helpers). `shared` must not depend on any slice. The ports also include `Fetcher` (HTTP), `Importer` (the OSM reader, a separate wasm module) and `Navigator`.
- **Dependency rule:** a slice may use another slice's `model` (junction uses the street model; city uses both), `city::store` / `city::binding`, `shell::Target` and `shared`; it must never reach into another slice's `vm`, views or text. New code goes in the slice that owns the feature, not in a layer folder.
- **View-models** expose presentation-ready reactive getters and take commands. They reach the browser **only through the ports**, so they are tested natively against `test_ports()` / `test_ports_with_time()`.
- **Views** bind to a view-model and forward events; they decide nothing. Rendering is by pure functions returning SVG strings (`plan_svg.rs`, `street/svg.rs`, and `map/svg.rs` for the home hero only) set with `inner_html`; pointer and keyboard logic are pure state machines with tests. `shell/view.rs` is the exception: it binds the static markup the pages share to `ShellVm` with web-sys listeners and effects.
- **Entry points** are in each slice's `mod.rs` (wasm-bindgen exports: `Plan`/`open_junction`/`mount_page`, `Sheet`/`open_street`/`mount_street_page`, `mount_map(basemap)`) and `lib.rs`. A page script (`web/app.js`, `junction.js`, `map.js`, each tiny) only loads the module, sets up the hatch `<defs>` (not `map.js`: it has no SVG; it creates the basemap with `createBasemap` from `web/basemap.js` and passes it to `mount_map`), and calls `mount_*` and `.mount_shell()`.

OpenStreetMap: the workspace has three more crates. `osm_network` is the plain network type (serde only). `osm_import` runs osm2streets (git dependencies, so they stay out of the editor's wasm) and fills it; it builds as its own wasm module (`web/pkg/osm_import*`), which `web/city.js` loads on first use and offers to the editor as `window.cityloomImportOsm`. `city/import.rs` turns a network into a `Layout` (the nodes and edges a `City` holds; the sample city is `Layout::sample()`): roads become streets (`Street::imported`, with their own `name`), a meeting of 3-5 arms the junction editor can draw becomes a junction (arms leaving within 30 degrees are spread; a meeting that cannot be drawn is a plain connection). A place is flagged only for checks a change makes worse than the city first laid out. Where a place's roads come from (`place/loader.rs`): from the one metro tile whose core holds its centre (`place/tiles.rs`: `data/metro/index.json` names the grid and the tiles there are; a tile holds every road reaching into its core widened by 0.5 km, so one tile covers an area's 0.4 km reach) and, if there is no such tile or it cannot be had, from Overpass. Whatever the source, the importer keeps only the area's box. `CityStore::current` opens the chosen area (`cityloom-area`), the default one (the Plateau Mont-Royal in Montréal, made like any other area) before one is chosen, or the sample city; the area's network and edits are kept under `cityloom-network:<key>` / `cityloom-city:<key>`. An edge keeps the road's centreline (`EdgeDef.shape`, `EdgeView.shape_mm`, in layout millimetres) and the layout remembers where it sits on the earth (`Layout.origin_m`, `CityView.origin_m`), so the map page can draw it over the basemap; only the editors and the home hero still draw an edge as a straight line between its nodes.

Persistence: a page opened from the map (`?junction=3`, `?street=7`) edits one place of the city. `CityBinding` writes it back through `CityStore` (against the city as stored *now*, so another tab's change is not overwritten) behind a 250 ms debounced `Keeper`; `flush()` is called on `pagehide`. Without the query parameter the page is a sandbox on the sample streets and keeps nothing. A write failure is said once.

## Conventions and gotchas

- E2E tests (`e2e/tests/*.spec.ts`, Playwright) cover each page: `shell.spec.ts` runs the same checks on all three, plus one spec per page. `fixtures.ts` makes every test fail on a page error and gives each a fresh browser context (empty storage), starting on the sample city (no network) unless it says `test.use({ area: "world" })`; `home.spec.ts` stubs Nominatim, Overpass and the metro tiles with `page.route`; `world.spec.ts` serves a one-place tile from `e2e/fixtures/plateau.osm.pbf`. They run against the built `web/pkg`, so rebuild after changing Rust (`just e2e` does). Pointer behaviour (drags) is only covered here, not by `cargo test`.
- The map page's e2e: every test serves `e2e/fixtures/plateau.pmtiles` as the basemap (`serveBasemap`); `mapReady(page)` waits for the places, and the map is MapLibre in a WebGL canvas (`window.cityloomMap`), so what the pointer does on it is covered only there. `E2E_PORT` moves the server and Playwright off 8137.
- Logic belongs in Rust; JS in `web/` stays start-up glue. Do not add logic there. The one exception is `web/basemap.js`, the adapter that owns the MapLibre map (the vendored MapLibre and PMTiles are in `web/vendor`).
- Add view-model behaviour with a native test against the port fakes. Component tests render to HTML on the host (each slice's `tests.rs`, Leptos `ssr` as a dev-dependency; helpers in `shared/testing.rs`); `shared::platform::browser_ports()` also works natively (thread-local recorders), which is how those tests read what was announced.
- Non-`Send` values inside reactive closures go in `StoredValue::new_local`; views take a `Copy` handle (`junction::watch::Watch`, `street::watch::SheetWatch`) over an `Rc<…Vm>`.
- `any_spawner::Executor::init_wasm_bindgen()` must run at the top of every `mount_*` function before any component creates an effect.
- `#![recursion_limit = "1024"]` is needed for the Leptos `view!` macros. New web-sys APIs need their feature added in `Cargo.toml`.
- Test-driven: write the failing test first. Commit each change separately once the tests pass.
- Checking in a real browser: a tab driven in the background throttles timers to ~1 s and pauses `requestAnimationFrame`, and `.focus()` does nothing (no document focus). Use microtask waits rather than timers, and expect focus-dependent behaviour to be unverifiable there. After rebuilding, bust the cache with `fetch(url, {cache: 'reload'})` for the wasm, the glue JS and the page script, then reload.
