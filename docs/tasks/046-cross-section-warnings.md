---
id: 046
title: "Add the cross-section warnings engine"
depends_on: [044]
features: [XS-010, XS-011, XS-012, XS-013, XS-014, XS-015]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core warnings::"
  - "uv run --frozen pytest -m e2e -k warnings"
model_hint: sonnet
---

# Context
Warnings are the design-critique half of the product: they tell a user their
street does not fit or is unsafe. The rules live in core so they are unit
testable without a browser.

# Scope
- Total width exceeding right-of-way.
- Bike lane directly adjacent to a drive lane with no buffer.
- Sidewalk narrower than a configured minimum.
- No sidewalk on either side.
- Warnings clearing as soon as the condition resolves.
- A warnings table in one place, each with a stable code.

# Out of scope
- Blocking the edit. Warnings advise; they never prevent.
- Jurisdiction-specific design standards or configurable rule sets.
- Warnings about the network (dead ends, connectivity). Not v1.

# Acceptance criteria (beyond the acceptance commands)
- Each rule has a core unit test on a fixture cross-section asserting the code.
- One e2e test asserts a warning appears and then clears after the fix.
- Thresholds are constants in the rules table, not inline literals.
