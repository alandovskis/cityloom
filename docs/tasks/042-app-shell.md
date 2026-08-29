---
id: 042
title: "Build the Tailwind pipeline and application shell"
depends_on: [002]
features: [UI-001, UI-002, UI-012, UI-013, UI-014]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k app_shell"
model_hint: sonnet
---

# Context
The chrome the map lives inside. Tailwind is compiled to one served stylesheet
rather than loaded from a CDN, so the app works offline and in tests.

# Scope
- Tailwind build wired into `just build`, emitting `web/app.css`.
- Full-viewport canvas with a left tool rail and a right inspector.
- No horizontal page scroll at 1280x800.
- Every interactive control reachable in tab order.
- An accessible label on the canvas.

# Out of scope
- Tool rail behaviour and inspector content. Task 043.
- Any visual design system beyond a plain, legible default.
- Mobile or touch layout; v1 targets desktop.

# Acceptance criteria (beyond the acceptance commands)
- The served HTML is asserted to contain no CDN script or stylesheet tag.
- A tab-order test walks every control and asserts none is skipped.
