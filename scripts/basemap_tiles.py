#!/usr/bin/env python3
"""Builds the map's basemap tiles of the Montréal metropolitan area from a Quebec OSM extract.

    scripts/basemap_tiles.py quebec-latest.osm.pbf [output.pmtiles] [--minzoom N] [--maxzoom N] [--bounds W,S,E,N]

Needs Java 21 or later and the same Geofabrik extract as scripts/metro_tiles.py
(https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf, about 1.2 GB).
Planetiler (OpenMapTiles schema) is downloaded to .tools/planetiler.jar on first use, and with --download
it fetches about 1 GB of Natural Earth and water-polygon sources once, into data/sources.
Writes web/data/basemap/montreal.pmtiles, which the map page reads with range requests.
"""

import argparse
import subprocess
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
JAR = ROOT / ".tools" / "planetiler.jar"
JAR_URL = "https://github.com/onthegomap/planetiler/releases/latest/download/planetiler.jar"
# The grid scripts/metro_tiles.py cuts: west, south, east, north.
METRO_BOUNDS = "-74.40,45.20,-73.09,45.81"
OUT = ROOT / "web" / "data" / "basemap" / "montreal.pmtiles"


def planetiler():
    if not JAR.exists():
        JAR.parent.mkdir(parents=True, exist_ok=True)
        print(f"downloading {JAR_URL}", file=sys.stderr)
        urllib.request.urlretrieve(JAR_URL, JAR)
    return JAR


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("extract", type=Path)
    ap.add_argument("output", type=Path, nargs="?", default=OUT)
    ap.add_argument("--bounds", default=METRO_BOUNDS, help="west,south,east,north in degrees")
    ap.add_argument("--minzoom", type=int, default=0)
    ap.add_argument("--maxzoom", type=int, default=14, help="MapLibre draws deeper zooms from this one")
    ap.add_argument("--memory", default="4g")
    args = ap.parse_args()
    if not args.extract.exists():
        sys.exit(f"{args.extract} is not there: get it from https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "java", f"-Xmx{args.memory}", "-jar", str(planetiler()),
            f"--osm-path={args.extract}", f"--output={args.output}",
            f"--bounds={args.bounds}", f"--minzoom={args.minzoom}", f"--maxzoom={args.maxzoom}",
            "--download", "--force", "--tmpdir=" + str(ROOT / ".tools" / "planetiler-tmp"),
        ],
        check=True,
        cwd=ROOT,
    )
    print(f"{args.output}: {args.output.stat().st_size / 1e6:.1f} MB")


if __name__ == "__main__":
    main()
