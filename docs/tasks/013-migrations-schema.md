---
id: 013
title: "Write the PostGIS schema migrations"
depends_on: [003]
features: [PERSIST-001, PERSIST-002, PERSIST-003, PERSIST-004, PERSIST-005]
status: todo
acceptance:
  - "just verify"
  - "just db && just migrate"
  - "uv run --frozen pytest -m integration -k schema"
model_hint: sonnet
---

# Context
The schema has to hold a document that two producers write and reconciliation
diffs, so provenance and segment ordering are columns, not afterthoughts.

# Scope
- `cities`: id, slug, name, schema_version, version, created_at, updated_at.
- `nodes`: `geometry(Point,4326)` with a GIST index, city FK, provenance.
- `edges`: `geometry(LineString,4326)`, from/to node FKs, cross-section FK,
  provenance.
- `cross_sections` and `segments`, the latter unique on
  (cross_section_id, position).
- `edge_metadata` keyed uniquely on (edge_id, key).
- Enable the PostGIS extension.

# Out of scope
- Repository code. Task 014.
- Indexes tuned for the 50k-edge target. Task 016 adds those from measurement.
- Down-migrations; forward-only for v1.

# Acceptance criteria (beyond the acceptance commands)
- Deleting a city cascades to nodes, edges, cross-sections, segments, and
  metadata, asserted by row counts.
- Inserting two segments at the same position in one cross-section violates the
  unique constraint.
