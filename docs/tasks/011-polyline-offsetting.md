---
id: 011
title: "Implement polyline offsetting and street ribbon generation"
depends_on: [010, 005]
features: [GEO-006, GEO-007, GEO-008, GEO-018]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core geo::offset"
model_hint: sonnet
---

# Context
A street is drawn as one quad strip per cross-section segment, each offset from
the centreline by its cumulative left-kerb distance. This is the bridge between
the cross-section model and the renderer.

# Scope
- Offset a polyline by a signed width, negative offsetting to the other side.
- Miter convex corners without self-intersection for interior angles above 30
  degrees; document the fallback below that.
- `ribbon(edge, cross_section)` emitting one quad strip per segment in
  left-to-right order.

# Out of scope
- GPU buffers. Task 031 consumes these as plain vertex data.
- Intersection polygons at nodes. Task 012.
- Bevel or round joins. Miter with a limit is enough for v1.

# Acceptance criteria (beyond the acceptance commands)
- Offset distance is asserted at sampled points along straight runs to 1e-6 m.
- A hairpin fixture (interior angle under 30 degrees) is covered and asserted
  not to produce NaN or a self-crossing strip.
- Adjacent segment strips abut with no gap above 1e-4 m (the GL-006 contract,
  proven here in pure Rust rather than against pixels).
