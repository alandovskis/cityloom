#!/usr/bin/env python3
"""Builds the map's basemap tiles of the Montréal metropolitan area from a Quebec OSM extract.

    scripts/basemap_tiles.py quebec-latest.osm.pbf [output.pmtiles] [--minzoom N] [--maxzoom N] [--bounds W,S,E,N]

Needs Java 21 or later and the same Geofabrik extract as scripts/metro_tiles.py
(https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf, about 1.2 GB).
Planetiler (OpenMapTiles schema), its latest release, is downloaded to .tools/planetiler.jar on first use
(delete it to take a newer one), and the first run fetches about 1 GB of Natural Earth and water-polygon
sources once, into data/sources.
Writes web/data/basemap/montreal.pmtiles, which the map page reads with range requests.
"""

import argparse
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
JAR = ROOT / ".tools" / "planetiler.jar"
# Where the kept jar came from: the release the "latest" link led to.
JAR_SOURCE = ROOT / ".tools" / "planetiler.jar.source"
JAR_URL = "https://github.com/onthegomap/planetiler/releases/latest/download/planetiler.jar"
# The grid scripts/metro_tiles.py cuts: west, south, east, north.
METRO_BOUNDS = "-74.40,45.20,-73.09,45.81"
OUT = ROOT / "web" / "data" / "basemap" / "montreal.pmtiles"


class _Release(urllib.request.HTTPRedirectHandler):
    """Follows redirects, and keeps the one that names the release (`.../releases/download/v0.10.2/...`):
    the last one is a signed address of the file that does not."""

    release = None

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        if "/releases/download/" in newurl:
            self.release = newurl
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def planetiler() -> Path:
    """The Planetiler jar, downloaded first if it is not kept yet. The download goes to a temporary name and
    is renamed only once whole, so an interrupted one never leaves a broken jar that looks kept."""
    if not JAR.exists():
        JAR.parent.mkdir(parents=True, exist_ok=True)
        partial = JAR.with_name(JAR.name + ".part")
        release = _Release()
        print(f"downloading {JAR_URL}", file=sys.stderr)
        try:
            with urllib.request.build_opener(release).open(JAR_URL) as response, partial.open("wb") as out:
                shutil.copyfileobj(response, out)
            partial.replace(JAR)
        finally:
            partial.unlink(missing_ok=True)
        JAR_SOURCE.write_text((release.release or JAR_URL) + "\n")
    source = JAR_SOURCE.read_text().strip() if JAR_SOURCE.exists() else "an earlier download"
    print(f"Planetiler: {JAR}, from {source}", file=sys.stderr)
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
