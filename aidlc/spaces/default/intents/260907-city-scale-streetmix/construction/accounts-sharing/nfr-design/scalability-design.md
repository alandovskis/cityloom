# Scalability Design — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._

Upstream inputs: `scalability-requirements.md` (nfr-requirements, this
Unit), `design-storage/scalability-design.md` (sibling precedent).

Design elements are numbered `SC-n`.

## SC-1 — Stateless request handling

Resolves NFR2.4.1. No `HashMap`-backed or other in-process store holds
per-session or per-account state between requests (unlike
`design-storage`'s intentionally in-process, short-lived rate-limiter
map — a different kind of state this Unit does not need, since it has no
anonymous-identifier-guessing surface of its own). Every request is
independently servable by any process instance; horizontal scaling past
the current single instance would need no sticky-session mechanism.

## SC-2 — Indexes for the two highest-frequency lookups

Resolves NFR2.4.2. `Session.token_hash` and `Account.email` each carry a
unique index, created by the initial migration. Verified by an `EXPLAIN`
check confirming an index scan (not sequential) on both lookup paths.

## SC-3 — Shared connection pool

Resolves the open assumption in `scalability-requirements.md`: this Unit
does not construct its own `PgPool`. Per the composition-root pattern
`design-storage/logical-components.md` LC-3 already established, whichever
binary owns process start constructs one shared `PgPool` and passes it
into this Unit's router constructor — pool sizing is that binary's
`infrastructure-design`/`environment-provisioning` decision, not
duplicated here.

## Traceability

See `traceability.json` in this directory.

_Confirmed (consolidated summary confirmed 2026-09-20)._
