---
id: 028
title: "Expose scene_state() as the Playwright assertion seam"
depends_on: [027]
features: [WASM-005, WASM-006, WASM-007]
status: todo
acceptance:
  - "just verify"
  - "wasm-pack test --headless --chrome crates/cityloom-client scene_state"
model_hint: sonnet
---

# Context
Rendering is WebGL, so there is no DOM to assert against. `scene_state()` is
the contract that makes every later UI feature testable; it is a product
surface, not a debug aid, and must stay in release builds.

# Scope
- `scene_state()` returning camera, mode, selection, and per-visible-edge ids
  with screen-space bounds.
- Install it on `window.__cityloom` (the `GLOBAL_NAME` constant already in the
  crate).
- `select_edge(id)` reflected in the returned selection.
- Version the returned shape so harness and client can disagree loudly.

# Out of scope
- Real visibility computation. Until culling exists (task 033), "visible" may
  mean "all edges"; the shape is what matters here.
- `render_stats()`. Task 033.
- The Python helper that reads it. Task 050.

# Acceptance criteria (beyond the acceptance commands)
- `window.__cityloom.scene_state()` is reachable from a page context, asserted
  in a browser test rather than a Rust unit test.
- The hook is present in a `--release` build, proven by asserting against the
  release bundle.
