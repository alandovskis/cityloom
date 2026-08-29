---
id: 018
title: "Implement the city CRUD endpoints"
depends_on: [017, 014, 015]
features: [API-003, API-004, API-005, API-006, API-007, API-008, API-009, API-010, API-011]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m integration -k city_crud"
model_hint: sonnet
---

# Context
This is the whole persistence surface the client uses: create, read, replace,
rename, delete, list.

# Scope
- `POST /api/cities` creating from a name, returning 201 and the slug.
- Slug derivation: URL-safe, de-duplicated with a numeric suffix.
- `GET /api/cities/{slug}`, 404 when unknown.
- `PUT /api/cities/{slug}` replacing the document, 409 on a stale version.
- `PATCH /api/cities/{slug}` renaming without touching the network.
- `DELETE /api/cities/{slug}` returning 204.
- `GET /api/cities` listing slug, name, updated_at, newest first.

# Out of scope
- Body size limits and document validation. Task 019.
- Import. Task 020.
- Pagination on the list endpoint; v1 returns all cities.

# Acceptance criteria (beyond the acceptance commands)
- Creating two cities named "Portland" yields distinct slugs, the second
  suffixed.
- `PATCH` leaves the document version's network bytes unchanged.
- `DELETE` then `GET` returns 404.
