#!/bin/sh
# Builds, then uploads the site and the server to the cityloom service of the Railway project of the same name
# (needs the Railway CLI, logged in, and cargo-zigbuild with zig: `just setup-deploy`). web/pkg, web/vendor and the tiles are
# gitignored, and `railway up` skips what git ignores, so the upload is a copy made outside the repository. The
# server is built here for Linux (musl, so it runs on a bare Alpine image); no Rust is compiled on Railway.
set -eu
cd "$(dirname "$0")/.."
PATH="$PWD/.tools/bin:$PATH"
# Fail before the long wasm build if the server cannot be cross-built.
missing=
command -v cargo-zigbuild >/dev/null || missing="$missing cargo-zigbuild"
command -v zig >/dev/null || python3 -m ziglang version >/dev/null 2>&1 || missing="$missing zig"
rustup target list --installed 2>/dev/null | grep -qx x86_64-unknown-linux-musl || missing="$missing x86_64-unknown-linux-musl-target"
[ -z "$missing" ] || { echo "missing:$missing: run 'just setup-deploy' (it says how to get zig)" >&2; exit 1; }
[ -f web/data/metro/index.json ] || { echo "web/data/metro/index.json is missing: run 'just prepare' first" >&2; exit 1; }
./scripts/build.sh
target=x86_64-unknown-linux-musl
cargo zigbuild --release --target "$target" -p cityloom_server
dir=$(./scripts/target-dir.sh)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
cp -R web "$stage/web"
# The target directory may be shared with other checkouts, which can replace the binary at any time: check the copy that is uploaded.
cp "$dir/$target/release/cityloom_server" "$stage/cityloom_server"
./scripts/check-no-fake.sh "$stage/cityloom_server"
cp deploy/Dockerfile "$stage/Dockerfile"
find "$stage" -name .DS_Store -delete
railway up "$stage" --path-as-root --ci \
  --project ae2bfc37-376c-4ff2-b11d-afa8f0c89b78 --environment production --service cityloom
