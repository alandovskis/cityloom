---
id: 004
title: "Define the Network model with provenance-tagged ids"
depends_on: []
features: [CORE-001, CORE-002, CORE-003, CORE-004, CORE-018, CORE-019, CORE-020, CORE-035]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core network::"
model_hint: sonnet
---

# Context
Both authoring paths are peers in v1, so ids must carry provenance from the
first line of model code: an id is either `Osm { id }` or `Authored`. Bolting
this on later would mean rewriting every persistence and reconciliation path.

# Scope
- `NodeId`/`EdgeId` newtypes carrying a `Provenance` discriminant.
- `Node` with position in city-local metres and incident edge ids.
- `Edge` with endpoints, an ordered centreline polyline, and a cross-section id.
- `Network` owning nodes and edges, with serde derives.
- An id allocator that never collides with ids already in the document.
- Arbitrary string key/value edge metadata.

# Out of scope
- Validation. That is task 007, deliberately separate so the model stays a
  plain data structure.
- Topology mutation (split/merge/remove). Task 008.
- Cross-section internals. Task 005 owns those; here it is an opaque id.
- Any database or wasm concern.

# Acceptance criteria (beyond the acceptance commands)
- A JSON round-trip over a fixture network is structurally equal, ids included.
- The centreline endpoint invariant (CORE-004) is asserted to 1e-9 m.
- Allocating an id into a network already holding that id's numeric value
  returns a fresh, non-colliding id.
