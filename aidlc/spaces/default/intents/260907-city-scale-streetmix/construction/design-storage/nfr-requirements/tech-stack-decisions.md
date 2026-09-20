# Tech Stack Decisions — `design-storage` (U10)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design,
this Unit); `components.md` `DesignRepository`; `contract-summary.md`
Contract 2; `osm-extract-proxy`'s own established stack (this
workspace's one other server Unit).

## Decisions

- **Language/crate**: Rust, a new workspace crate (`cityloom-design-storage`),
  a server binary — not compiled to WASM. Reuses the workspace's existing
  `rust-toolchain.toml`.
- **HTTP framework — `axum`, same pinned version as `osm-extract-proxy`
  (`=0.8.9`)**: this project already has one axum service in the
  workspace; using the same framework and version for a second server
  Unit avoids maintaining two HTTP stacks for what `unit-of-work.md`
  itself frames as "four work boundaries inside one deployment boundary"
  — the eventual composition into a single Railway service (confirmed at
  `environment-provisioning`) is far simpler if every server Unit's
  router is an axum `Router` that can be `.merge()`d into one.
- **Async runtime — `tokio`, same pinned version (`=1.53.1`)**: same
  reasoning — one runtime, one process.
- **Database access — `sqlx` (postgres, `runtime-tokio`, `tls-rustls`
  features), not an ORM.** `components.md` names PostgreSQL directly as
  `DesignRepository`'s external dependency ("a separate service from the
  application so the application holds no attached volume," NFR3.3).
  `sqlx` gives compile-time-checked queries (matching this project's
  "fail at compile time, not review" discipline, `team.md` Code Style)
  without an ORM's abstraction distance from the actual SQL — the query
  set here (insert, select-by-id, delete-by-id, delete-expired) is small
  enough that an ORM buys nothing and a hand-written schema stays
  legible. `rustls`, not the OpenSSL TLS backend, for the same
  reasoning `osm-extract-proxy`'s `reqwest` dependency already
  established in this workspace (pure-Rust TLS, no system OpenSSL
  dependency to manage on the Railway build image).
- **`cityloom-api-types`**: extended with this Unit's own request/response
  types (`UploadRequest`, `UploadReceipt`, this Unit's `ApiError`/reason
  enum per Contract 2) — reusing the same shared-types crate
  `osm-extract-proxy` (U9) already established, so the two server Units'
  wire contracts live in one place rather than two independently-typed
  copies.
- **`cityloom-design-payload`**: a normal dependency — this Unit validates
  `payloadVersion` (Contract 3's fail-closed check) before storing a
  payload, reusing U3's `DesignPayload::from_json` rather than
  re-implementing the version check.
- **Error handling**: `thiserror`, same as `osm-extract-proxy`, for
  `StorageFailure`'s `std::error::Error` impl.
- **Tracing/logging — `tracing` + `tracing-subscriber`**, same pinned
  versions as `osm-extract-proxy`, for the structured, no-per-request-row
  logging discipline `observability-requirements.md` requires.

## What was already decided elsewhere

| Decision | Owner | Why not this Unit's to (re)decide |
|---|---|---|
| Single Railway service, one deployable for U9–U12 | `unit-of-work.md`, `team.md` Deployment | This Unit's binary is one router merged into the shared entry point at `environment-provisioning`, not an independently deployed service |
| The anonymous identifier is client-minted, never server-derived | `decisions.md` ADR-006 | This Unit only stores and compares it |
| The `DesignPayload` wire shape | `design-payload-spec` (U3), Contract 3 | This Unit stores it opaquely and only checks `payloadVersion` |

## Rationale

Reusing `osm-extract-proxy`'s exact framework/runtime pins keeps the
eventual single-binary composition (`environment-provisioning`) a
router merge rather than a rewrite, and keeps this workspace's Rust
server surface to one HTTP stack rather than two.

## Assumptions & Open Questions

- **[assumption]** Whether `cityloom-design-storage` ships as a library
  crate whose router is imported by `osm-extract-proxy`'s binary, or as
  its own binary later merged at the process/reverse-proxy level, is an
  `infrastructure-design`/`environment-provisioning` decision — this
  stage fixes the crate's own internal stack, not its deployment
  topology.

## Traceability

See `traceability.json` in this directory.
