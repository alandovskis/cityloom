# CityLoom

Streetmix at city scale: residents compose and compare whole-city scenarios.
This repository holds three surfaces: a city map (`map.html`), the street cross-section editor (`index.html`) and the intersection editor (`intersection.html`). The map opens the other two on a street or junction of one sample city (`src/city.rs`), kept in browser storage.

- `src/`: the editing models in Rust, compiled to WebAssembly: `model.rs` for a street, `junction.rs`, `junction_view.rs` and `plan.rs` for a junction. Tests are included.
- `src/ui/`: the page's components, drawn from the model's view by [Leptos](https://leptos.dev) (client-side rendering). The page is moving into Rust one panel at a time; `shared.rs` is the bridge that lets these components and the remaining script edit the same junction. So far: the junction's notes (turn table, crossings, conflicts, checks, changes).
- `web/`: the page. It draws the model's view and relays input; it holds no editing rules.
- All catalogue widths, rules and capacity rates are synthetic placeholders.

## Build and run

```sh
cargo test
cargo install wasm-bindgen-cli --version 0.2.129 --root .tools   # once; must match Cargo.toml
./scripts/build.sh                                               # writes web/pkg
python3 -m http.server 8137 --directory web                      # open http://127.0.0.1:8137/
```
