---
id: 024
title: "Assemble the deterministic OSM import pipeline"
depends_on: [022, 023, 009]
features: [OSM-018, OSM-019, OSM-020]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core osm::import"
  - "uv run --frozen pytest -m integration -k import_pipeline"
model_hint: sonnet
---

# Context
Determinism is what makes reconciliation (task 025) possible: if two imports of
the same extract differ, every re-import looks like a change. This task wires
parsing, topology, tags, and projection into one reproducible pass.

# Scope
- End-to-end: fixture in, `City` out.
- Set the city origin from the bbox centre and project all geometry to local
  metres.
- Stamp OSM provenance on every node and edge.
- Guarantee determinism: stable iteration order and stable id assignment, so
  two runs produce byte-identical JSON.
- Replace the stub worker from task 020 with this pipeline.

# Out of scope
- Reconciliation into a non-empty city. Task 025; here, importing into a
  non-empty city may fail loudly.
- Live Overpass fetching beyond a thin, untested-by-default adapter behind the
  fixture-driven interface.

# Acceptance criteria (beyond the acceptance commands)
- Two runs over one fixture serialise to identical bytes, asserted on the
  serialised string.
- Every node and edge carries an `Osm` provenance resolving to a fixture id.
- Projected coordinates round-trip back to the fixture's lon/lat within 0.01 m.
