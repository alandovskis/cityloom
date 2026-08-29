---
id: 049
title: "Build the pytest database and server fixtures"
depends_on: [017, 013]
features: [TEST-002, TEST-003, TEST-004, TEST-008, TEST-009]
status: todo
acceptance:
  - "just verify"
  - "just db && uv run --frozen pytest -m integration -k harness"
model_hint: sonnet
---

# Context
Every integration test from task 014 onward depends on these fixtures. They are
listed late only because they need a server and a schema to point at; schedule
them as early as those exist.

# Scope
- A `pg_database` fixture provisioning an isolated database per test and
  dropping it afterwards.
- A `server` fixture starting the binary on an ephemeral port and waiting for
  `/healthz` before yielding.
- Teardown that kills the process even when the test fails or errors.
- Checked-in OSM fixtures; no test reaching the network.
- The suite passing with outbound network access blocked.

# Out of scope
- Playwright. Task 050.
- Parallel test execution; correctness first.
- Seeding realistic city data beyond what individual tests build.

# Acceptance criteria (beyond the acceptance commands)
- A deliberately failing test still leaves no orphaned server process or
  leftover database, asserted by a follow-up check.
- Two tests running in sequence are asserted not to see each other's rows.
