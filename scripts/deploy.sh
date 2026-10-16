#!/bin/sh
# Builds, then uploads the site and the server to the cityloom service of the Railway project of the same name
# (needs the Railway CLI, logged in, and cargo-zigbuild with zig: `just setup-deploy`). web/pkg, web/vendor and the tiles are
# gitignored, and `railway up` skips what git ignores, so the upload is a copy made outside the repository. The
# server is built here for Linux (musl, so it runs on a bare Alpine image); no Rust is compiled on Railway.
set -eu
cd "$(dirname "$0")/.."
PATH="$PWD/.tools/bin:$PATH"
for tile in web/data/basemap/montreal.pmtiles web/data/metro/index.json; do
  [ -f "$tile" ] || { echo "$tile is missing: run 'just prepare' first" >&2; exit 1; }
done
./scripts/build.sh
target=x86_64-unknown-linux-musl
cargo zigbuild --release --target "$target" -p cityloom_server
server="target/$target/release/cityloom_server"
./scripts/check-no-fake.sh "$server"
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
cp -R web "$stage/web"
cp "$server" "$stage/cityloom_server"
cp deploy/Dockerfile "$stage/Dockerfile"
find "$stage" -name .DS_Store -delete
railway up "$stage" --path-as-root --ci \
  --project ae2bfc37-376c-4ff2-b11d-afa8f0c89b78 --environment production --service cityloom
