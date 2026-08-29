---
id: 033
title: "Add viewport culling, render stats, and hit the 10k-edge frame budget"
depends_on: [032]
features: [GL-014, GL-015, GL-016]
status: todo
acceptance:
  - "just verify"
  - "wasm-pack test --headless --chrome crates/cityloom-client render::stats"
  - "uv run --frozen pytest -m e2e -k render_perf"
model_hint: sonnet
---

# Context
Where "city scale" is either met or not. Culling is also what makes
`scene_state().visible_edges` mean something real, which tasks 038 and 050
depend on.

# Scope
- Frustum-cull edges outside the viewport and exclude them from draw calls.
- `render_stats()` reporting draw calls, triangle count, and culled edge count.
- Reach 60 fps with 10,000 visible edges on the reference machine; add a
  spatial index if measurement demands one.
- Make `scene_state().visible_edges` reflect real culling.

# Out of scope
- Level-of-detail simplification by zoom. Post-v1 unless the budget forces it.
- GPU instancing, unless measurement demands it.
- Label culling. Task 035.

# Acceptance criteria (beyond the acceptance commands)
- An edge just outside the viewport is asserted culled; one just inside is not.
- The fps assertion records the measured value in the failure message, so a
  regression reports how far off it was.
