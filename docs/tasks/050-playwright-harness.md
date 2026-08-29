---
id: 050
title: "Build the Playwright harness and scene_state helper"
depends_on: [049, 028, 042]
features: [TEST-005, TEST-006, TEST-007]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k harness"
model_hint: sonnet
---

# Context
Because the map is WebGL, e2e assertions read `scene_state()` rather than query
the DOM. This helper is what every UI task's tests are written against, so its
ergonomics matter more than usual.

# Scope
- Fixtures launching headless chromium and exposing `page`, serving the built
  app from the task 049 server fixture.
- A helper reading `window.__cityloom.scene_state()` and returning a dict.
- A helper waiting for the first rendered frame before assertions run.
- Assert the harness's expected `scene_state` shape version matches the
  client's, failing loudly on drift.

# Out of scope
- Visual or screenshot regression testing; assertions are on state, not pixels.
- Cross-browser runs; chromium only for v1.
- Recording traces or video by default.

# Acceptance criteria (beyond the acceptance commands)
- The first-frame helper is asserted to actually block until a frame has been
  rendered, using a page that renders late.
- A deliberate shape-version mismatch produces a clear failure naming both
  versions.
