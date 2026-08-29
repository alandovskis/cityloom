---
id: 009
title: "Implement WGS84 to city-local metre projection"
depends_on: []
features: [GEO-001, GEO-002]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core geo::projection"
model_hint: sonnet
---

# Context
The model works in metres so that widths, offsets, and hit tolerances are
plain arithmetic. OSM arrives in degrees and Postgres stores 4326, so a single
exact-enough projection sits at both boundaries.

# Scope
- A per-city origin (lon/lat) stored on the document.
- Forward and inverse transforms between WGS84 and local metres.
- Accuracy of 0.01 m round-trip over a 20 km extent from the origin.
- Exact identity at the origin.

# Out of scope
- Full geodesy. A local tangent-plane approximation is sufficient for city
  extents; document the error growth and the 20 km validity limit in a comment.
- Choosing the origin per city. Task 024 sets it from the import bbox centre.
- PostGIS-side reprojection. Storage stays 4326.

# Acceptance criteria (beyond the acceptance commands)
- Round-trip error is asserted at the origin, at 1 km, and at 20 km.
- Points north and south of the origin are both covered, so a latitude-sign
  error cannot pass.
