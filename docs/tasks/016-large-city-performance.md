---
id: 016
title: "Meet the 50k-edge save and load budget"
depends_on: [014]
features: [PERSIST-015]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m integration -k perf_large_city"
model_hint: sonnet
---

# Context
"City scale" is the product claim, so the persistence layer has to survive a
real city's edge count. Measure before optimising; the naive per-row insert
from task 014 is the thing under test.

# Scope
- A generator producing a synthetic 50,000-edge city.
- A timed save and load asserting under 10 s each against a local database.
- Whatever it takes to get there: batched or COPY-based inserts, indexes added
  from measured plans, fewer round trips.
- Record the measured numbers in the test as a comment.

# Out of scope
- Client-side rendering performance. Task 033.
- Optimising below the budget. Meet 10 s and stop.
- Distributed or sharded storage.

# Acceptance criteria (beyond the acceptance commands)
- The perf test is marked `integration` so it does not slow the default gate.
- The loaded 50k city compares equal to the generated one, so a fast path that
  loses data cannot pass.
