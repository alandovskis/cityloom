---
id: 017
title: "Build the axum application shell"
depends_on: [003]
features: [API-001, API-002, API-015, API-021, API-022, API-023]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m integration -k server_shell"
model_hint: sonnet
---

# Context
Every later endpoint inherits this shell's error envelope, logging, and
shutdown behaviour, so it is worth establishing before any route with business
logic exists.

# Scope
- axum app with `GET /healthz` returning 200 `ok`, and 503 when the database is
  unreachable.
- A single error type rendering `{"error":{"code":...,"message":...}}` for
  every failure path.
- Request-id middleware echoing `x-request-id` and tagging structured JSON logs.
- Static file serving from `web/`, with `application/wasm` for `.wasm`.
- Graceful SIGTERM shutdown draining in-flight requests, exiting 0.

# Out of scope
- City routes. Task 018.
- Import and job routes. Task 020.
- Auth. There are no accounts in v1; cities are addressed by slug alone.
- Rate limiting.

# Acceptance criteria (beyond the acceptance commands)
- `/healthz` returns 503 with the database stopped, and 200 once it is back,
  without restarting the server.
- A request with a client-supplied `x-request-id` gets the same id echoed back;
  one without gets a generated id.
- SIGTERM during an in-flight slow request lets it complete before exit.
