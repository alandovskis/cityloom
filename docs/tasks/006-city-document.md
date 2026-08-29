---
id: 006
title: "Assemble the City document with a versioned schema"
depends_on: [004, 005]
features: [CORE-030, CORE-031, CORE-032, CORE-033, CORE-034]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core city::"
model_hint: sonnet
---

# Context
`City` is the unit of persistence, of the API payload, and of what the wasm
client holds. Cross-sections live in a library shared by reference so that
editing one street's section can update every street that shares it — the
behaviour XS-016 promises.

# Scope
- `City` bundling network, named cross-section library, display name, slug, and
  `schema_version`.
- Reference-shared cross-sections: N edges may point at one library entry.
- `detach_cross_section(edge)` producing an independent copy.
- A typed `UnsupportedSchemaVersion` error when loading a future version.
- An empty city that is valid and round-trips.

# Out of scope
- Persisting to Postgres. Task 014.
- The undo stack. Task 041.
- Migration between schema versions. v1 only needs to refuse the future
  cleanly; leave a comment saying so.

# Acceptance criteria (beyond the acceptance commands)
- Editing a shared section is observable through all referencing edges.
- After `detach`, editing the detached copy leaves every other referencing edge
  byte-identical.
- Loading `schema_version = SCHEMA_VERSION + 1` returns the typed error and
  does not panic.
