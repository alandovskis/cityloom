---
id: 022
title: "Filter OSM ways and build network topology"
depends_on: [021, 004, 008]
features: [OSM-003, OSM-004, OSM-005, OSM-006, OSM-007, OSM-008]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core osm::topology"
model_hint: sonnet
---

# Context
OSM ways are not network edges: they run through junctions and share nodes.
Splitting at shared nodes is what turns a way soup into a connected graph.

# Scope
- Retain only ways whose `highway` tag is in a routable allow-list.
- Skip ways tagged `area=yes`.
- A way with N node refs becomes an edge with an N-point centreline.
- Split ways at every node shared with another retained way.
- Keep single-use interior nodes as shape points, not network nodes.
- Skip ways referencing absent nodes with a warning, without aborting.

# Out of scope
- Tag-to-cross-section derivation. Task 023.
- Projection. Task 024.
- Turn restrictions, relations, or routing semantics. Not v1.

# Acceptance criteria (beyond the acceptance commands)
- A fixture of two ways crossing at a shared node yields four edges and one
  degree-4 node.
- Shape-point count is asserted, so over-splitting fails the test.
- `Network::validate()` returns empty for the built topology.
