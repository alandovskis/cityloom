---
id: 014
title: "Implement the city repository save/load round-trip"
depends_on: [013, 006]
features: [PERSIST-006, PERSIST-007, PERSIST-008, PERSIST-009, PERSIST-014, PERSIST-016]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m integration -k repository"
model_hint: sonnet
---

# Context
`save_city` / `load_city` is the boundary where the in-memory `City` meets
Postgres. Equality after a round-trip is the single property everything else
depends on.

# Scope
- `save_city` and `load_city` against the task 013 schema.
- Whole save in one transaction; a mid-save error leaves prior state intact.
- Saving an existing slug updates in place rather than inserting a duplicate.
- OSM provenance preserved across the round-trip.
- FK-restricted node deletion surfaced as a typed error, not a raw driver error.

# Out of scope
- Optimistic concurrency and slug conflicts. Task 015.
- Performance at 50k edges. Task 016.
- HTTP. Task 018.

# Acceptance criteria (beyond the acceptance commands)
- A fixture city with shared cross-sections round-trips with the sharing
  intact — two edges still point at one library entry after load.
- Provenance discriminants survive, asserted per node and per edge.
- Injecting a failure between the edge and segment writes leaves the database
  at its pre-save state.
