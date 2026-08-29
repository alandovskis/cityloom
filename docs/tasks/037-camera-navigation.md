---
id: 037
title: "Add fit-to-bounds, URL hash persistence, and keyboard navigation"
depends_on: [036, 010]
features: [CAM-004, CAM-005, CAM-007, CAM-008, CAM-009, CAM-010]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k camera_nav"
model_hint: sonnet
---

# Context
The camera in the URL is what makes a shared link land on the same view
(DOC-011). Fit-to-bounds is what a freshly loaded city needs so it does not open
on empty space.

# Scope
- Fit-to-bounds framing the network with configured padding, using the network
  bounds from task 010.
- Leave the camera unchanged when the network is empty.
- Encode camera state in the URL hash, restored on reload within 1e-6.
- Arrow keys panning by a fixed screen step; `+`/`-` zooming about the centre;
  double-click zooming toward the clicked point.

# Out of scope
- The share button. Task 047.
- Deep-linking to a selected edge; v1 encodes the camera only.
- History entries per camera move — replace the hash, do not push.

# Acceptance criteria (beyond the acceptance commands)
- Reload with a hash restores centre and scale within 1e-6, asserted through
  `scene_state()`.
- Fit-to-bounds on an empty city asserts the camera is byte-identical.
