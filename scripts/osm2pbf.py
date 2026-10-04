"""Writes OSM XML as a PBF file: `uv run --with osmium scripts/osm2pbf.py in.osm out.osm.pbf`.

This is how the default area that ships with the app (web/data/default.osm.pbf) is made
from an Overpass answer. Needs only uv; pyosmium is fetched into a throwaway environment."""

import sys

import osmium

if len(sys.argv) != 3:
    sys.exit("usage: osm2pbf.py in.osm out.osm.pbf")

writer = osmium.SimpleWriter(sys.argv[2])
try:
    for obj in osmium.FileProcessor(sys.argv[1]):
        writer.add(obj)
finally:
    writer.close()
