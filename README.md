# CityLoom

Streetmix at city scale: residents compose and compare whole-city scenarios.
This repository holds two surfaces: the street cross-section editor (`index.html`) and the intersection editor (`intersection.html`).

- `src/`: the editing models in Rust, compiled to WebAssembly: `model.rs` for a street, `junction.rs`, `junction_view.rs` and `plan.rs` for a junction. Tests are included.
- `web/`: the page. It draws the model's view and relays input; it holds no editing rules.
- All catalogue widths, rules and capacity rates are synthetic placeholders.

## Build and run

```sh
cargo test
cargo install wasm-bindgen-cli --version 0.2.129 --root .tools   # once; must match Cargo.toml
./scripts/build.sh                                               # writes web/pkg
python3 -m http.server 8137 --directory web                      # open http://127.0.0.1:8137/
```
