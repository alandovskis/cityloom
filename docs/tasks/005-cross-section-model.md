---
id: 005
title: "Define the CrossSection and Segment model"
depends_on: []
features: [CORE-021, CORE-022, CORE-023, CORE-024, CORE-025, CORE-026, CORE-027, CORE-028, CORE-029]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core cross_section::"
model_hint: sonnet
---

# Context
The cross-section is the thing users actually edit — CityLoom's Streetmix
inheritance. It is also what the renderer tessellates and what reconciliation
compares, so its width arithmetic must be exact and total.

# Scope
- `SegmentKind` covering at least: sidewalk, bike lane, drive lane, parking
  lane, transit lane, turn lane, median, planting strip, buffer.
- Per-kind declaration of whether direction is meaningful.
- `Segment` with kind, width in metres, and `Direction::{Inbound, Outbound, None}`.
- `CrossSection` as an ordered segment list with derived total width.
- Insert, remove, and reorder operations preserving the width invariants.
- Cumulative left-kerb offset per segment.
- Rejection of negative and non-finite widths with a typed error.

# Out of scope
- Warnings (bike lane beside drive lane, minimum sidewalk width). Task 046.
- Sharing semantics across edges. Task 006.
- Rendering colours. Task 031.
- Defaults derived from OSM tags. Task 023.

# Acceptance criteria (beyond the acceptance commands)
- Reorder is asserted to preserve total width exactly, not approximately.
- Offset of segment i equals the sum of widths 0..i, checked on a fixture with
  at least six segments.
- `f64::NAN` and `-1.0` widths are both rejected.
