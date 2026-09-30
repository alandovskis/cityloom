#!/bin/sh
# Builds the Rust core to WebAssembly into web/pkg.
# Needs wasm-bindgen-cli at the version pinned in Cargo.toml:
#   cargo install wasm-bindgen-cli --version <that version> --root .tools
set -eu
cd "$(dirname "$0")/.."
PATH="$PWD/.tools/bin:$PATH"
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/cityloom_editor.wasm
