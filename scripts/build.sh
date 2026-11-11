#!/bin/sh
# Builds the Rust core to WebAssembly into web/pkg.
# Needs wasm-bindgen-cli at the version pinned in crates/cityloom_editor/Cargo.toml:
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
cargo build --release --target wasm32-unknown-unknown --workspace --exclude cityloom_server --exclude citymodel_builder --exclude osm_import
# The target directory may be shared with other checkouts, whose builds replace these files: take both at once.
dir=$(./scripts/target-dir.sh)
wasm_dir=$(mktemp -d)
trap 'rm -rf "$wasm_dir"' EXIT
cp "$dir/wasm32-unknown-unknown/release/cityloom_editor.wasm" "$dir/wasm32-unknown-unknown/release/cityloom_report_pdf.wasm" "$wasm_dir/"
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section --out-dir web/pkg \
  "$wasm_dir/cityloom_editor.wasm"
# The design-review PDF writer is its own module, loaded when a report is made.
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section --out-dir web/pkg \
  "$wasm_dir/cityloom_report_pdf.wasm"
# The editor must not carry the PDF writer (krilla): a crate that depends on `cityloom_report_pdf` would pull it in.
# `cargo tree -p cityloom_editor -e normal | grep -c krilla` must print 0.
if grep -q report_pdf web/pkg/cityloom_editor.js; then
  echo "web/pkg/cityloom_editor.js exports report_pdf: the editor links the PDF writer (cityloom_report_pdf)" >&2
  exit 1
fi
# The map's libraries, from npm (`npm ci` at the repository root) into web/vendor. MapLibre is ES modules that
# find each other beside themselves.
[ -d node_modules/maplibre-gl ] || { echo "node_modules/maplibre-gl is missing: run 'just setup' first" >&2; exit 1; }
mkdir -p web/vendor
cp node_modules/maplibre-gl/dist/maplibre-gl.mjs node_modules/maplibre-gl/dist/maplibre-gl-shared.mjs \
  node_modules/maplibre-gl/dist/maplibre-gl-worker.mjs node_modules/maplibre-gl/dist/maplibre-gl.css web/vendor/
# Compressed copies of the built files, which the server sends as they are (see the script). If this script stops
# before here (a missing tool), a copy from an earlier run is left beside a newer file; the server does not send a
# copy older than its file (static_files.rs), and the next complete run writes them again.
node scripts/precompress.mjs web/pkg web/vendor
