---
id: 008
title: "Implement node removal, edge split, and edge merge"
depends_on: [004, 007]
features: [CORE-010, CORE-011, CORE-012, CORE-013, CORE-014, CORE-015, CORE-016, CORE-017]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core topology::"
model_hint: sonnet
---

# Context
Splitting is how the OSM importer builds topology at shared nodes (OSM-006) and
how drawing connects to an existing street (DRAW-004). One implementation
serves both, so it lives in core rather than in either caller.

# Scope
- `remove_node` cascading to incident edges.
- Degree derived from incident edges, correct after every mutation.
- `split_edge(edge, t)` yielding two edges sharing a new node, preserving total
  length within 1e-6 m and copying the cross-section id to both children.
- Refuse `t` outside (0, 1) with a typed error, leaving the network unchanged.
- `merge_edges` across a degree-2 node, concatenating polylines in order.
- Refuse merge when degree != 2, or when cross-section ids differ without
  `force`.

# Out of scope
- Undo. Task 041 wraps these as reversible operations.
- Snapping and merge-on-drop. Task 040.
- Persisting the result. Task 014.

# Acceptance criteria (beyond the acceptance commands)
- Every refusal path asserts the network is byte-identical afterwards.
- Split then merge at the same node round-trips to a network equal to the
  original, modulo the id of the removed intermediate node.
- `validate()` returns empty after each successful operation on a valid input.
