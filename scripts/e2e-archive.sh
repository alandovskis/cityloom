#!/usr/bin/env sh
# Packs the e2e fixtures into a .citymodel archive and runs the E2E tier (e2e/tiers.json: the archive and delivery specs,
# and the UI specs of `archive_also`, map and street) over a server that unpacks it (`just e2e-archive`). Run from the repository root, after the wasm is built and the server is built and pinned (`scripts/pin-server.sh`).
set -eu
work=target/e2e-archive
rm -rf "$work"
mkdir -p "$work/in/metro" "$work/in/basemap" "$work/areas" "$work/data"
cp e2e/fixtures/plateau.osm.pbf "$work/in/metro/0_0.osm.pbf"
cp e2e/fixtures/plateau.pmtiles "$work/in/basemap/montreal.pmtiles"
printf '{"lon0":-73.6078,"lat0":45.5161,"dlon":0.0257,"dlat":0.018,"tiles":["0_0"]}\n' > "$work/in/metro/index.json"
cargo run -q -p cityloom_prepare -- pack --metro "$work/in/metro" --basemap "$work/in/basemap" \
    --out "$work/areas/plateau.citymodel" --area plateau --source plateau.osm.pbf --created 0
cd e2e
E2E_ARCHIVE=1 npx playwright test --project=e2e "$@"
