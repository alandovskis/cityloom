#!/usr/bin/env python3
"""Cuts the roads of the Montréal metropolitan area out of a Quebec OSM extract into tiles.

    scripts/metro_tiles.py quebec-latest.osm.pbf [workdir]

Needs osmium-tool (`brew install osmium-tool`) and a Geofabrik extract
(https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf, about 1.2 GB).
Writes web/data/metro/X_Y.osm.pbf and web/data/metro/index.json, which the app reads (src/place/tiles.rs).

A tile is a rectangle (its core) holding every road that reaches into the core widened by a margin,
whole, with the nodes it needs. The margin is wider than half an area (400 m), so a place whose
centre is in a tile's core is read from that one tile.
"""

import json
import os
import resource
import subprocess
import sys
from pathlib import Path

# The grid. The south-west corner of tile 0_0, and the size of a core: about 2 km square at this latitude.
LON0, LAT0 = -74.40, 45.20
DLON, DLAT = 0.0257, 0.018
NX, NY = 51, 34  # to -73.09 east and 45.81 north: the metropolitan area and some country round it
# 0.5 km: wider than the 0.4 km an area reaches from its centre.
MARGIN_LON, MARGIN_LAT = 0.0064, 0.0045

HIGHWAYS = (
    "motorway,motorway_link,trunk,trunk_link,primary,primary_link,secondary,secondary_link,"
    "tertiary,tertiary_link,unclassified,residential,living_street,service,pedestrian"
)
# What osmium allows in one run.
MAX_EXTRACTS = 500
# An empty tile is a file with only a header.
EMPTY_BYTES = 400

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "web" / "data" / "metro"


def osmium(*args):
    subprocess.run(["osmium", *map(str, args)], check=True)


def main():
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__)
    source = Path(sys.argv[1]).resolve()
    if not source.is_file():
        sys.exit(
            f"{source} does not exist. Download the Quebec extract first (about 1.2 GB):\n"
            "  curl -L -o quebec-latest.osm.pbf https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf"
        )
    work = Path(sys.argv[2] if len(sys.argv) == 3 else "/tmp/cityloom-metro")
    work.mkdir(parents=True, exist_ok=True)

    west, south = LON0 - MARGIN_LON, LAT0 - MARGIN_LAT
    east, north = LON0 + NX * DLON + MARGIN_LON, LAT0 + NY * DLAT + MARGIN_LAT

    metro, roads = work / "metro.osm.pbf", work / "roads.osm.pbf"
    print("cutting the metropolitan area out of", source.name, flush=True)
    osmium("extract", "-b", f"{west},{south},{east},{north}", source, "-o", metro, "--overwrite")
    print("keeping the roads", flush=True)
    osmium("tags-filter", metro, f"w/highway={HIGHWAYS}", "-o", roads, "--overwrite")

    for old in OUT.glob("*.osm.pbf"):
        old.unlink()
    OUT.mkdir(parents=True, exist_ok=True)

    # osmium cuts at most 500 extracts in a run, and has them all open at once.
    soft, hard = resource.getrlimit(resource.RLIMIT_NOFILE)
    resource.setrlimit(resource.RLIMIT_NOFILE, (min(max(soft, MAX_EXTRACTS + 64), hard), hard))
    extracts = [
        {
            "output": f"{x}_{y}.osm.pbf",
            "output_format": "pbf",
            "bbox": [
                LON0 + x * DLON - MARGIN_LON,
                LAT0 + y * DLAT - MARGIN_LAT,
                LON0 + (x + 1) * DLON + MARGIN_LON,
                LAT0 + (y + 1) * DLAT + MARGIN_LAT,
            ],
        }
        for x in range(NX)
        for y in range(NY)
    ]
    config = work / "tiles.json"
    print(f"cutting {len(extracts)} tiles", flush=True)
    for i in range(0, len(extracts), MAX_EXTRACTS):
        config.write_text(json.dumps({"directory": str(OUT), "extracts": extracts[i : i + MAX_EXTRACTS]}))
        osmium("extract", "--config", config, roads, "--overwrite")

    kept = sorted(p for p in OUT.glob("*.osm.pbf") if p.stat().st_size > EMPTY_BYTES)
    for p in OUT.glob("*.osm.pbf"):
        if p not in kept:
            p.unlink()
    index = {"lon0": LON0, "lat0": LAT0, "dlon": DLON, "dlat": DLAT, "tiles": [p.name.removesuffix(".osm.pbf") for p in kept]}
    (OUT / "index.json").write_text(json.dumps(index, separators=(",", ":")) + "\n")
    total = sum(p.stat().st_size for p in kept)
    print(f"{len(kept)} tiles, {total / 1e6:.1f} MB, in {OUT}")


if __name__ == "__main__":
    main()
