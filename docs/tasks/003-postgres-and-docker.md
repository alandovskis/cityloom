---
id: 003
title: "Stand up Postgres, the migration runner, and the production image"
depends_on: []
features: [OPS-012, OPS-013, OPS-014, OPS-016]
status: todo
acceptance:
  - "just verify"
  - "just db && just migrate && just migrate"
  - "docker build -t cityloom:test . && docker run --rm -d -p 8081:8080 cityloom:test"
model_hint: sonnet
---

# Context
`docker-compose.yml` declares a PostGIS service but nothing connects to it and
no migration runner exists. Persistence tasks (013 onward) and the pytest
database fixture (049) both block on this.

# Scope
- Add a `migrate` binary to `cityloom-server` that applies ordered SQL files
  from `migrations/` and records applied versions in a `_migrations` table.
- Make re-running apply zero migrations and exit 0.
- Add a multi-stage `Dockerfile` producing a slim runtime image.
- Add a readiness wait so `just db` returns only once Postgres accepts
  connections, with a 30 s ceiling.

# Out of scope
- The actual schema. Table definitions are task 013.
- Down-migrations. Forward-only for v1; say so in a comment.
- Any HTTP handler beyond whatever `/healthz` the image needs to answer, which
  task 017 replaces.

# Acceptance criteria (beyond the acceptance commands)
- A migration failing mid-file rolls back and leaves `_migrations` unchanged.
- The migration runner reads `DATABASE_URL` and fails with a clear message when
  it is unset.
