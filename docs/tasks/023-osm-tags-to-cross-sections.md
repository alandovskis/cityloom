---
id: 023
title: "Derive cross-sections from OSM tags"
depends_on: [005, 021]
features: [OSM-009, OSM-010, OSM-011, OSM-012, OSM-013, OSM-014, OSM-015, OSM-016, OSM-017]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core osm::tags"
model_hint: sonnet
---

# Context
This is where an imported city stops being lines on a map and becomes editable
streets. Tag coverage is deliberately narrow and explicit; anything untagged
falls back to a documented per-highway-class default.

# Scope
- `oneway=yes` and `oneway=-1` (the latter reversing the centreline).
- `lanes=N`, with per-class defaults when absent.
- `width=X`, with per-class defaults when absent.
- `sidewalk=both|left|right|no`.
- `cycleway=lane` and `cycleway:left` / `cycleway:right`.
- `parking:lane:*` on the tagged side.
- `maxspeed` preserved as edge metadata.
- A defaults table keyed by highway class, in one place.

# Out of scope
- Tags beyond this list. Unrecognised tags are ignored, not errors.
- Bus lanes, tram tracks, verges. Post-v1.
- Round-tripping edits back out to OSM. CityLoom imports; it does not publish.

# Acceptance criteria (beyond the acceptance commands)
- Each tag above has a test asserting the resulting segment list exactly, not
  just the segment count.
- `oneway=-1` is asserted to reverse geometry *and* produce outbound lanes.
- An untagged `highway=residential` produces the documented default section.
