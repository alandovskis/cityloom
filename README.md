# CityLoom

Streetmix at city scale: residents compose and compare whole-city scenarios.
This repository holds the first surface, the street cross-section editor.

- `src/`: the editing model in Rust (`model.rs`, tests included), compiled to WebAssembly.
- `web/`: the page. It draws the model's view and relays input; it holds no editing rules.
- All catalogue widths, rules and capacity rates are synthetic placeholders.

## Build and run

```sh
cargo test
cargo install wasm-bindgen-cli --version 0.2.129 --root .tools   # once; must match Cargo.toml
./scripts/build.sh                                               # writes web/pkg
python3 -m http.server 8137 --directory web                      # open http://127.0.0.1:8137/
```
