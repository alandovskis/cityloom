---
id: 007
title: "Implement total Network validation"
depends_on: [004, 006]
features: [CORE-005, CORE-006, CORE-007, CORE-008, CORE-009]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core validate::"
model_hint: sonnet
---

# Context
Two producers write networks — the OSM importer and the drawing tools — and the
API must reject malformed documents with 422 rather than storing them. One
shared validator keeps both honest.

# Scope
- `Network::validate() -> Vec<Violation>` reporting dangling endpoints,
  duplicate node ids, duplicate edge ids, zero-length edges (< 0.01 m), and
  edges referencing a missing cross-section id.
- Return every violation found, never short-circuit on the first.
- Each `Violation` names the offending id and a stable machine-readable code.

# Out of scope
- Repairing anything. Validation reports; it never mutates.
- HTTP status mapping. Task 019.
- Geometric self-intersection checks. Not a v1 requirement.

# Acceptance criteria (beyond the acceptance commands)
- A fixture carrying four distinct violations returns exactly four, and the
  test asserts on the codes, not the count alone.
- A valid network returns an empty vector.
