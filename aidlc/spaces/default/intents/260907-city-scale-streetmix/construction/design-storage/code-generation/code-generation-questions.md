# Code Generation — Plan Approval — `design-storage` (U10)

## Plan Approval

`code-generation-plan.md` (19 steps, TDD Red/Green/Refactor across four
testable layers: data model, repository/data access, business logic
[validation/rate-limiting/failure-mapping and, separately, the expiry
sweep], and API/endpoint) and `unit-test-instructions.md` (24 tests
across five modules, against a real local PostgreSQL database — no
database mocking, per `team.md`) are ready for approval.

This Unit is a new server crate, `cityloom-design-storage` (axum + sqlx +
PostgreSQL), merged into the shared Railway service's router. It is the
first Unit in this workspace whose tests require a real local database
rather than pure in-process logic or committed fixture files.

Approve this exact Code Generation plan?

[Approval Fingerprint]: sha256:30d13f2eefd627a7753bd3407ea388caeed8e2c9bd25875555dc207898ffc0cb

- Approve Plan
- Request Changes

[Answer]: Approve Plan