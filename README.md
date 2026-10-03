# CityLoom

Streetmix at city scale: residents compose and compare whole-city scenarios.
This repository holds three surfaces: a city map (`map.html`), the street cross-section editor (`index.html`) and the intersection editor (`intersection.html`). The map opens the other two on a street or junction of one sample city (`src/city.rs`), kept in browser storage.

- `src/`: one folder per feature (a vertical slice), each holding its own model, view-model and views, plus a small kernel:
  - `street/`, `junction/`: the editors. `model.rs` has the editing rules (`junction/` also has `geometry.rs` and `read_model.rs`); `vm.rs` is the view-model; `text.rs` the words said to a screen reader; the rest are the Leptos views, with tests beside them.
  - `map/`: the city map (view-model, camera, gestures, views). `city/`: the sample city, how it is kept in storage (`store.rs`) and the binding a page writes back through (`binding.rs`). `shell/`: the settings menu, sidebars and notes tabs every page shares.
  - `shared/`: what every slice uses: the catalogue and units, the ports a view-model reaches the browser through (`ports.rs`, with test fakes) and their browser side (`platform.rs`), the reactive `core.rs`, the `keeper.rs` that debounces writes, and the helpers views bind with.
- A view-model has no page in it: it owns the model, exposes properties that are ready to show (words, flags, lists) and takes commands, and reaches the browser only through the ports, so it is tested on the host with fakes. A view (Leptos, client-side rendered) binds to those properties and sends commands; it decides nothing. Components are drawn to HTML in tests; `shell/view.rs` binds the markup the pages share and is checked in the browser.
- A slice reaches another only through that slice's model (or `city`'s store and binding, or `shell`'s `Target`), never its view-model or views; `shared` depends on no slice.
- `web/`: the pages' markup and a few lines of start-up per page that load the module and call `mount_*`; no logic.
- All catalogue widths, rules and capacity rates are synthetic placeholders.

## Build and run

```sh
cargo test
cargo install wasm-bindgen-cli --version 0.2.129 --root .tools   # once; must match Cargo.toml
./scripts/build.sh                                               # writes web/pkg
python3 -m http.server 8137 --directory web                      # open http://127.0.0.1:8137/
```

## End-to-end tests

Playwright drives the built pages in Chromium (`e2e/`). Build the module first; a small Node server serves `web/`.

```sh
./scripts/build.sh
cd e2e && npm ci && npx playwright install chromium   # once
npm test                                              # or: npx playwright test tests/street.spec.ts
```
