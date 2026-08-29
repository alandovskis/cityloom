---
id: 030
title: "Create the WebGL2 context, shader pipeline, and frame loop"
depends_on: [027]
features: [GL-001, GL-002, GL-003, GL-010, GL-011, GL-019]
status: todo
acceptance:
  - "just verify"
  - "wasm-pack test --headless --chrome crates/cityloom-client render::context"
model_hint: sonnet
---

# Context
The renderer's foundation: context, programs, resize handling, and teardown.
Getting DPR and viewport right here prevents a class of blurry-canvas bugs that
are painful to attribute later.

# Scope
- Acquire a WebGL2 context; on failure render a visible fallback message rather
  than a blank canvas.
- Compile and link shader programs at init with no GL error.
- A frame loop leaving `gl.getError()` at 0 after a full frame.
- Drawing buffer sized to CSS size times device pixel ratio.
- Resize updating viewport and projection without aspect distortion.
- Teardown deleting every buffer, texture, and program created.

# Out of scope
- Drawing streets. Task 031.
- Culling and stats. Task 033.
- Labels. Task 035.

# Acceptance criteria (beyond the acceptance commands)
- A forced context-creation failure is asserted to produce the fallback message
  in the DOM.
- Teardown is asserted by counting created-versus-deleted GL objects, not by
  eyeballing.
- Resize at DPR 1 and DPR 2 both assert correct drawing buffer dimensions.
