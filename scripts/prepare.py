#!/usr/bin/env python3
"""Prepares the data the app reads for the Montréal metropolitan area: the Quebec OSM extract, downloaded if it
is not there, then the road tiles (scripts/metro_tiles.py) and the basemap's tiles (scripts/basemap_tiles.py)
cut from it.

    scripts/prepare.py [quebec-latest.osm.pbf] [--refresh]

The extract is Geofabrik's (https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf, about
1.2 GB). It is kept in data/ (not in git); a later run reads it from there, and --refresh downloads a newer one.
An interrupted download goes to a temporary name and is picked up where it stopped by the next run, so it never
leaves a broken extract that looks kept.
Needs osmium-tool (for the road tiles) and Java 21 or later (for the basemap).
"""

import argparse
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = Path(__file__).resolve().parent
URL = "https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf"
EXTRACT = ROOT / "data" / "quebec-latest.osm.pbf"
CHUNK = 1 << 20
# Progress is said every 100 MB.
REPORT = 100 << 20


def download(url: str, target: Path) -> None:
    """Downloads `url` to `target`, renamed from a `.part` file only once whole. A `.part` file left by an earlier
    attempt is continued when the server allows it, and started again when it does not."""
    target.parent.mkdir(parents=True, exist_ok=True)
    part = target.with_name(target.name + ".part")
    have = part.stat().st_size if part.exists() else 0
    request = urllib.request.Request(url, headers={"Range": f"bytes={have}-"} if have else {})
    try:
        response = urllib.request.urlopen(request)
    except urllib.error.HTTPError as error:
        if error.code != 416 or not have:
            raise
        # The file asked from beyond its end: the earlier attempt had it whole.
        part.replace(target)
        return
    with response:
        resumed = response.status == 206
        if have and not resumed:
            have = 0
        total = response.headers.get("Content-Length")
        total = int(total) + have if total else None
        print(f"downloading {url}" + (f" (from {have / 1e6:.0f} MB)" if have else ""), file=sys.stderr)
        done, reported = have, have
        with part.open("ab" if resumed else "wb") as out:
            while chunk := response.read(CHUNK):
                out.write(chunk)
                done += len(chunk)
                if done - reported >= REPORT:
                    reported = done
                    of = f" of {total / 1e6:.0f}" if total else ""
                    print(f"  {done / 1e6:.0f}{of} MB", file=sys.stderr)
    if total is not None and done != total:
        sys.exit(f"{url}: got {done} bytes of {total}; run again to continue")
    part.replace(target)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("extract", type=Path, nargs="?", default=EXTRACT, help="where the extract is, or is put")
    ap.add_argument("--refresh", action="store_true", help="download the extract again, even if it is there")
    args = ap.parse_args()
    if args.refresh or not args.extract.exists():
        download(URL, args.extract)
    print(f"extract: {args.extract} ({args.extract.stat().st_size / 1e6:.0f} MB)", file=sys.stderr)
    for script in ("metro_tiles.py", "basemap_tiles.py"):
        print(f"\n== {script}", file=sys.stderr)
        subprocess.run([sys.executable, str(SCRIPTS / script), str(args.extract)], check=True, cwd=ROOT)


if __name__ == "__main__":
    main()
