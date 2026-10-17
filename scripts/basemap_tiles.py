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

from __future__ import annotations

import argparse
import re
import sys
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from include.progress import Progress, Spec, Step, StepError  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
TOOLS = ROOT / ".tools"
JAR = TOOLS / "planetiler.jar"
# Where the kept jar came from: the release the "latest" link led to.
JAR_SOURCE = TOOLS / "planetiler.jar.source"
JAR_URL = "https://github.com/onthegomap/planetiler/releases/latest/download/planetiler.jar"
# The grid scripts/metro_tiles.py cuts: west, south, east, north.
METRO_BOUNDS = "-74.40,45.20,-73.09,45.81"
OUT = ROOT / "web" / "data" / "basemap" / "montreal.pmtiles"
LOGS = ROOT / "data" / "logs"
CHUNK = 1 << 20
MINZOOM, MAXZOOM, MEMORY = 0, 14, "4g"
# What the run does, in order, for the progress display (scripts/prepare.py adds these to its own).
STEPS = [Spec("planetiler", "basemap", JAR.name), Spec("basemap", "basemap", OUT.name)]


BOUNDS_FORMAT = re.compile(r"-?\d+(\.\d+)?(,-?\d+(\.\d+)?){3}")
MEMORY_FORMAT = re.compile(r"\d+[kmg]", re.IGNORECASE)


def matching(pattern: re.Pattern, what: str):
    """An argparse type that accepts only text of the form `pattern`, so what is handed to Java is one setting's value."""

    def check(text: str) -> str:
        if not pattern.fullmatch(text):
            raise argparse.ArgumentTypeError(f"{text!r} is not {what}")
        return text

    return check


class _Release(urllib.request.HTTPRedirectHandler):
    """Follows redirects, and keeps the one that names the release (`.../releases/download/v0.10.2/...`):
    the last one is a signed address of the file that does not."""

    release = None

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        if "/releases/download/" in newurl:
            self.release = newurl
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def planetiler(p: Progress, step: Step) -> Path:
    """The Planetiler jar, downloaded first if it is not kept yet. The download goes to a temporary name and
    is renamed only once whole, so an interrupted one never leaves a broken jar that looks kept."""
    if JAR.exists():
        p.update(step, detail="kept")
        return JAR
    JAR.parent.mkdir(parents=True, exist_ok=True)
    partial = JAR.with_name(JAR.name + ".part")
    release = _Release()
    try:
        with urllib.request.build_opener(release).open(JAR_URL) as response, partial.open("wb") as out:
            length = response.headers.get("Content-Length")
            total, done = int(length) if length else None, 0
            while chunk := response.read(CHUNK):
                out.write(chunk)
                done += len(chunk)
                of = f" of {total / 1e6:,.0f}" if total else ""
                p.update(step, detail=f"{done / 1e6:,.0f}{of} MB", fraction=done / total if total else None)
        partial.replace(JAR)
    finally:
        partial.unlink(missing_ok=True)
    JAR_SOURCE.write_text((release.release or JAR_URL) + "\n")
    p.update(step, detail=f"{JAR.stat().st_size / 1e6:,.0f} MB")
    return JAR


def run(p: Progress, extract: Path, output: Path, bounds: str, minzoom: int, maxzoom: int, memory: str):
    """Builds the basemap from `extract`, step by step as STEPS says, with Java's output in the steps' logs."""
    with p.step("planetiler") as step:
        if not extract.exists():
            raise StepError(
                f"{extract} is not there: get it from https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf"
            )
        jar = planetiler(p, step)
    with p.step("basemap") as step:
        output.parent.mkdir(parents=True, exist_ok=True)
        p.run(
            step,
            [
                "java", f"-Xmx{memory}", "-jar", str(jar),
                f"--osm-path={extract}", f"--output={output}",
                f"--bounds={bounds}", f"--minzoom={minzoom}", f"--maxzoom={maxzoom}",
                "--download", "--force", "--tmpdir=" + str(TOOLS / "planetiler-tmp"),
            ],
            cwd=ROOT,
        )
        p.update(step, subject=output.name, detail=f"{output.stat().st_size / 1e6:.1f} MB")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("extract", type=Path)
    ap.add_argument("output", type=Path, nargs="?", default=OUT)
    ap.add_argument("--bounds", type=matching(BOUNDS_FORMAT, "west,south,east,north in degrees"), default=METRO_BOUNDS, help="west,south,east,north in degrees")
    ap.add_argument("--minzoom", type=int, default=MINZOOM)
    ap.add_argument("--maxzoom", type=int, default=MAXZOOM, help="MapLibre draws deeper zooms from this one")
    ap.add_argument("--memory", type=matching(MEMORY_FORMAT, "a size such as 4g"), default=MEMORY)
    args = ap.parse_args()
    # Absolute paths, so none can be read by Java or Planetiler as an option.
    args.extract, args.output = args.extract.resolve(), args.output.resolve()
    with Progress(STEPS, LOGS, ROOT) as p:
        run(p, args.extract, args.output, args.bounds, args.minzoom, args.maxzoom, args.memory)


if __name__ == "__main__":
    main()
