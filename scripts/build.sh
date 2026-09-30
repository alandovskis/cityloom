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
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/cityloom_editor.wasm
