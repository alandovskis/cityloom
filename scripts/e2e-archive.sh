#!/usr/bin/env sh
# Packs the e2e fixtures into a .cityloom archive and runs the map, street and archive specs over a server that
# unpacks it (`just e2e-archive`). Run from the repository root, after the wasm and the server are built.
set -eu
work=target/e2e-archive
rm -rf "$work"
mkdir -p "$work/in/metro" "$work/in/basemap" "$work/areas" "$work/data"
cp e2e/fixtures/plateau.osm.pbf "$work/in/metro/0_0.osm.pbf"
cp e2e/fixtures/plateau.pmtiles "$work/in/basemap/montreal.pmtiles"
printf '{"lon0":-73.6078,"lat0":45.5161,"dlon":0.0257,"dlat":0.018,"tiles":["0_0"]}\n' > "$work/in/metro/index.json"
cargo run -q -p cityloom_prepare -- pack --metro "$work/in/metro" --basemap "$work/in/basemap" \
    --out "$work/areas/plateau.cityloom" --area plateau --source plateau.osm.pbf --created 0
cd e2e
E2E_ARCHIVE=1 npx playwright test tests/archive.spec.ts tests/map.spec.ts tests/street.spec.ts "$@"
