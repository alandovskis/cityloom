---
id: 020
title: "Add the job queue and OSM import endpoints"
depends_on: [018]
features: [API-016, API-017, API-018, API-019, API-020, OSM-022]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m integration -k import_jobs"
model_hint: sonnet
---

# Context
Importing a bbox takes long enough that a synchronous request would time out,
so import is a job the UI polls. This task builds the machinery; task 024
supplies the work it runs.

# Scope
- `POST /api/cities/{slug}/import/osm` accepting a bbox, returning 202 and a
  job id.
- Reject inverted or oversized bboxes with 400.
- `GET /api/jobs/{id}` reporting pending, running, succeeded, or failed.
- Monotonically increasing progress fraction reaching 1.0.
- Failed jobs expose their error message; succeeded import jobs expose their
  reconciliation report.

# Out of scope
- The import itself. Stub the worker; task 024 replaces the stub.
- Durable job storage across restarts. In-process is enough for v1; note the
  limitation.
- Cancellation.

# Acceptance criteria (beyond the acceptance commands)
- Progress values sampled during a run are non-decreasing.
- A job whose worker panics reports `failed` with a message, and does not wedge
  the queue.
