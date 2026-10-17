#!/usr/bin/env python3
"""Prepares the data the app reads for the Montréal metropolitan area: the Quebec OSM extract, downloaded if it
is not there, then the road tiles (scripts/metro_tiles.py) and the basemap's tiles (scripts/basemap_tiles.py)
cut from it.

    scripts/prepare.py [quebec-latest.osm.pbf] [--refresh] [--batch N]

The extract is Geofabrik's (https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf, about
1.2 GB). It is kept in data/ (not in git); a later run reads it from there, and --refresh downloads a newer one.
An interrupted download goes to a temporary name and is picked up where it stopped by the next run, so it never
leaves a broken extract that looks kept.
Needs osmium-tool (for the road tiles) and Java 21 or later (for the basemap).
"""

from __future__ import annotations

import argparse
import sys
import urllib.error
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import basemap_tiles  # noqa: E402
import metro_tiles  # noqa: E402
from include.progress import Progress, Spec, Step, StepError  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
URL = "https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf"
EXTRACT = ROOT / "data" / "quebec-latest.osm.pbf"
CHUNK = 1 << 20


def open_from(url: str, have: int):
    """The response for `url` from byte `have` on, or None when the file asked from beyond its end: the earlier
    attempt had it whole."""
    request = urllib.request.Request(url, headers={"Range": f"bytes={have}-"} if have else {})
    try:
        return urllib.request.urlopen(request)
    except urllib.error.HTTPError as error:
        if error.code == 416 and have:
            return None
        raise


def megabytes(done: int, total: int | None) -> str:
    return f"{done / 1e6:,.0f}" + (f" of {total / 1e6:,.0f}" if total else "") + " MB"


def copy_with_progress(response, out, done: int, total: int | None, say) -> int:
    """Writes the response to `out`, telling `say(done, total)` how far it is; returns the bytes `out` then has."""
    while chunk := response.read(CHUNK):
        out.write(chunk)
        done += len(chunk)
        say(done, total)
    return done


def download(url: str, target: Path, say) -> None:
    """Downloads `url` to `target`, renamed from a `.part` file only once whole. A `.part` file left by an earlier
    attempt is continued when the server allows it, and started again when it does not."""
    target.parent.mkdir(parents=True, exist_ok=True)
    part = target.with_name(target.name + ".part")
    have = part.stat().st_size if part.exists() else 0
    response = open_from(url, have)
    if response is None:
        part.replace(target)
        return
    with response:
        resumed = response.status == 206
        if not resumed:
            have = 0
        length = response.headers.get("Content-Length")
        total = int(length) + have if length else None
        with part.open("ab" if resumed else "wb") as out:
            done = copy_with_progress(response, out, have, total, say)
    if total is not None and done != total:
        raise StepError(f"{url}: got {done} bytes of {total}; run again to continue")
    part.replace(target)


def get_extract(p: Progress, step: Step, extract: Path, refresh: bool):
    """The extract, downloaded unless it is kept (and not --refresh); the step says its size, and "kept" if it was."""
    downloading = refresh or not extract.exists()
    if downloading:
        def say(done, total):
            p.update(step, detail=megabytes(done, total), fraction=done / total if total else None)

        download(URL, extract, say)
    size = f"{extract.stat().st_size / 1e6:,.0f} MB"
    p.update(step, detail=size if downloading else f"{size}, kept")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("extract", type=Path, nargs="?", default=EXTRACT, help="where the extract is, or is put")
    ap.add_argument("--refresh", action="store_true", help="download the extract again, even if it is there")
    ap.add_argument(
        "--batch", type=metro_tiles.batch_size, default=metro_tiles.MAX_EXTRACTS,
        help="tiles cut in one osmium run (metro_tiles.py's --batch): fewer take less memory",
    )
    args = ap.parse_args()
    # An absolute path, so it cannot be read by a tool it is handed to as an option.
    extract = args.extract.resolve()
    steps = [Spec("extract", "extract", extract.name), *metro_tiles.STEPS, *basemap_tiles.STEPS]
    with Progress(steps, metro_tiles.LOGS, ROOT) as p:
        with p.step("extract") as step:
            get_extract(p, step, extract, args.refresh)
        metro_tiles.run(p, extract, metro_tiles.WORK, args.batch)
        basemap_tiles.run(
            p, extract, basemap_tiles.OUT, basemap_tiles.METRO_BOUNDS, basemap_tiles.MINZOOM, basemap_tiles.MAXZOOM,
            basemap_tiles.MEMORY,
        )


if __name__ == "__main__":
    main()
