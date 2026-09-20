# Tech Stack Decisions — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._


Upstream inputs: `functional-spec.md`, `rules.md` (functional-design,
this Unit); `components.md` `AccountService`/`SharingService`;
`contract-summary.md` Contract 4/5; `design-storage/tech-stack-decisions.md`
(this workspace's established server-crate stack).

## Decisions

- **Language/crate**: Rust, a new workspace crate (`cityloom-accounts-sharing`),
  a server binary — not compiled to WASM. Reuses the workspace's existing
  `rust-toolchain.toml`.
- **HTTP framework — `axum`, same pinned version as `osm-extract-proxy`/
  `design-storage` (`=0.8.9`)**: this workspace already has the "one
  router per server Unit, merged at the composition root" convention
  established; a third HTTP stack would break it for no benefit.
- **Async runtime — `tokio`, same pinned version (`=1.53.1`)**.
- **Database access — `sqlx` (postgres, `runtime-tokio`, `tls-rustls`
  features), not an ORM.** Same reasoning as `design-storage`: a small,
  hand-written query set (account lookup, session CRUD, grant/link CRUD,
  the erasure-cascade transaction) doesn't need an ORM's abstraction
  distance, and compile-time-checked queries match this project's
  fail-at-compile-time discipline (`team.md` Code Style).
- **Password hashing — `argon2` (the `argon2` crate, RFC 9106 defaults).**
  Chosen over `bcrypt`/`scrypt`: it's the current OWASP-recommended
  default, has an actively maintained pure-Rust implementation with no
  system library dependency (keeping the Railway build image simple, the
  same reasoning that picked `rustls` over an OpenSSL backend elsewhere
  in this workspace), and its tunable memory cost is a deliberate
  defense against GPU-accelerated cracking that this Unit's asset
  (account access to a person's designs) warrants even at this project's
  small scale.
- **Session tokens — a random 256-bit value, hashed (SHA-256) before
  storage in `Session.token_hash`.** Not `argon2` for the session token
  itself: the token is already high-entropy and short-lived (unlike a
  human-chosen password), so a fast cryptographic hash is sufficient and
  avoids paying `argon2`'s deliberate slowness on every authenticated
  request's session lookup — a different threat model from password
  storage, so a different, appropriately-weighted mechanism.
- **`cityloom-api-types`**: extended with this Unit's own request/response
  types (sign-up/sign-in, save-with-name, grant/link operations, the
  Contract 4/5 error shape), reusing the same shared-types crate
  `osm-extract-proxy` and `design-storage` already established.
- **Error handling — `thiserror`**, same as the other server Units.
- **Tracing/logging — `tracing` + `tracing-subscriber`**, same pinned
  versions, for the structured no-per-request-row logging discipline
  `observability-requirements.md` requires.

## What was already decided elsewhere

| Decision | Owner | Why not this Unit's to (re)decide |
|---|---|---|
| Single Railway service, one deployable for U9–U12 | `unit-of-work.md`, `team.md` Deployment | This Unit's binary is one router merged into the shared entry point at `environment-provisioning` |
| Session cookie flags (`HttpOnly`, `Secure`, `SameSite`) | `contract-summary.md` Contract 4 | Fixed at contract design, not renegotiable here |
| The access-gate flag's existence and default-off state | `project.md` Mandated | This Unit implements the gate; it does not decide whether one exists |

## Rationale

Reusing the established framework/runtime pins keeps the eventual
single-binary composition a router merge, not a rewrite. The one new
dependency this Unit introduces beyond `design-storage`'s stack —
`argon2` — is scoped tightly to the one thing `design-storage` never
needed: hashing a human-chosen credential.

## Assumptions & Open Questions

- **[assumption]** Whether `cityloom-accounts-sharing` ships as its own
  binary later merged at the process/reverse-proxy level, or as a
  library crate imported by one of the other server Units' binaries, is
  an `infrastructure-design`/`environment-provisioning` decision — this
  stage fixes the crate's own internal stack, not its deployment
  topology (same deferral `design-storage` made for the same reason).

## Traceability

See `traceability.json` in this directory.
