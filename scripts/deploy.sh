#!/bin/sh
# Builds, then uploads the site to the cityloom service of the Railway project of the same name (needs the
# Railway CLI, logged in). web/pkg, web/vendor and the tiles are gitignored, and `railway up` skips what git
# ignores, so the upload is a copy of web/ made outside the repository.
set -eu
cd "$(dirname "$0")/.."
for tile in web/data/basemap/montreal.pmtiles web/data/metro/index.json; do
  [ -f "$tile" ] || { echo "$tile is missing: run 'just prepare' first" >&2; exit 1; }
done
./scripts/build.sh
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir "$stage/e2e"
cp -R web "$stage/web"
cp e2e/serve.mjs "$stage/e2e/"
cp deploy/Dockerfile "$stage/Dockerfile"
find "$stage" -name .DS_Store -delete
railway up "$stage" --path-as-root --ci \
  --project ae2bfc37-376c-4ff2-b11d-afa8f0c89b78 --environment production --service cityloom
