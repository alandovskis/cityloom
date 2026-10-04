#!/bin/sh
# Builds the Rust core to WebAssembly into web/pkg.
# Needs wasm-bindgen-cli at the version pinned in Cargo.toml:
#   cargo install wasm-bindgen-cli --version <that version> --root .tools
set -eu
cd "$(dirname "$0")/.."
PATH="$PWD/.tools/bin:$PATH"
# On macOS some rustup toolchains ship rust-lld without the libLLVM.dylib it
# looks for next to itself, so linking wasm32 fails with "Library not loaded".
# The library is in the sysroot's lib directory; let the loader fall back to it.
if [ "$(uname)" = Darwin ]; then
  DYLD_FALLBACK_LIBRARY_PATH="$(rustc --print sysroot)/lib${DYLD_FALLBACK_LIBRARY_PATH:+:$DYLD_FALLBACK_LIBRARY_PATH}"
  export DYLD_FALLBACK_LIBRARY_PATH
fi
cargo build --release --target wasm32-unknown-unknown --workspace
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/cityloom_editor.wasm
# The OpenStreetMap reader is its own module, loaded when a place is opened.
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/osm_import.wasm
# The map's libraries, from npm (`npm ci` at the repository root) into web/vendor. MapLibre is ES modules that
# find each other beside themselves; pmtiles.js defines a global `pmtiles`.
[ -d node_modules/maplibre-gl ] || { echo "node_modules/maplibre-gl is missing: run 'just setup' first" >&2; exit 1; }
mkdir -p web/vendor
cp node_modules/maplibre-gl/dist/maplibre-gl.mjs node_modules/maplibre-gl/dist/maplibre-gl-shared.mjs \
  node_modules/maplibre-gl/dist/maplibre-gl-worker.mjs node_modules/maplibre-gl/dist/maplibre-gl.css \
  node_modules/pmtiles/dist/pmtiles.js web/vendor/
