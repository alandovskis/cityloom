# CityLoom

Streetmix at city scale: residents compose and compare whole-city scenarios.
This repository holds three surfaces: a city map (`map.html`), the street cross-section editor (`index.html`) and the intersection editor (`intersection.html`). The map opens the other two on a street or junction of one sample city (`src/city.rs`), kept in browser storage.

- `src/`: the editing models in Rust, compiled to WebAssembly: `model.rs` for a street, `junction.rs`, `junction_view.rs` and `plan.rs` for a junction. Tests are included.
- `src/vm/`: the view-models (MVVM). A view-model has no page in it: it owns the model, exposes properties that are ready to show (words, flags, lists) and takes commands, and reaches the browser only through the ports in `ports.rs` (announce, store, timers), so it is tested on the host with fakes. There is one per page (`map.rs`, `junction.rs`, `street.rs`) and one for what every page shares (`shell.rs`: units, region, theme, settings menu, sidebars, notes tabs). Pages that edit a place in the city keep their changes there through a `CityBinding` and a `Keeper`.
- `src/ui/`: the views, drawn by [Leptos](https://leptos.dev) (client-side rendering). A view binds to a view-model's properties and sends it what the person does; it decides nothing. Components are drawn to HTML in tests; `shell.rs` binds the markup the pages share, and is checked in the browser.
- `web/`: the pages' markup and a few lines of start-up per page that load the module and call `mount_*`; no logic.
- All catalogue widths, rules and capacity rates are synthetic placeholders.

## Build and run

```sh
cargo test
cargo install wasm-bindgen-cli --version 0.2.129 --root .tools   # once; must match Cargo.toml
./scripts/build.sh                                               # writes web/pkg
python3 -m http.server 8137 --directory web                      # open http://127.0.0.1:8137/
```
