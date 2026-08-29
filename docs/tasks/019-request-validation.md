---
id: 019
title: "Enforce request limits and document validation at the API edge"
depends_on: [018, 007]
features: [API-012, API-013, API-014]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m integration -k validation"
model_hint: sonnet
---

# Context
A 50k-edge city is a large PUT body, and a malformed network must never reach
Postgres. The core validator from task 007 becomes the API's gatekeeper.

# Scope
- Reject bodies over 32 MB with 413.
- Reject malformed JSON with 400 and a machine-readable code.
- Run `Network::validate()` on every write; reject failures with 422 listing
  the violations in the error envelope.

# Out of scope
- Repairing invalid documents.
- Streaming or chunked upload. A 32 MB ceiling is sufficient for v1.
- Compression negotiation.

# Acceptance criteria (beyond the acceptance commands)
- The 422 body lists every violation, matching what `validate()` returned.
- A 413 is returned without buffering the whole body into memory.
