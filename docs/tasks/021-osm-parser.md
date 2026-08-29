---
id: 021
title: "Parse OSM XML fixtures into nodes and ways"
depends_on: []
features: [OSM-001, OSM-002]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core osm::parse"
model_hint: haiku   # mechanical: XML into plain structs, no domain judgement
---

# Context
Tests must never touch the network (TEST-008), so import is driven from checked
-in OSM XML fixtures. XML rather than PBF keeps fixtures readable and diffable.

# Scope
- Parse OSM XML into raw nodes (id, lon, lat, tags) and ways (id, node refs,
  tags).
- Typed parse error on malformed XML.
- Check in at least one small real-world-shaped fixture under `tests/fixtures/`.

# Out of scope
- Any filtering or interpretation of tags. Tasks 022 and 023.
- PBF support.
- Fetching from Overpass. The fixture is the input for all tests; live fetching
  is wired in task 024 behind the job worker.

# Acceptance criteria (beyond the acceptance commands)
- Node and way counts for the fixture match hand-counted values.
- Truncated XML returns the typed error rather than a partial parse.
