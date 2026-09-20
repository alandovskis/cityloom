# NFR Design — Questions — `design-storage` (U10)

No open question remains for this Unit: the six confirmed
`nfr-requirements` artifacts, `functional-spec.md`, and `rules.md`
together determine the design elements needed to meet every requirement
this Unit's `service` kind produces.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifacts?

Produces (all six, per this Unit's `service` kind):
`performance-design.md` (5 elements: single round-trip queries, bounded
connection pool, batched expiry sweep, PD-4 clock-start discipline
matching `osm-extract-proxy`'s R-01 review lesson, CI benchmark),
`security-design.md` (9 elements as an 8-layer defence-in-depth chain:
ordered checks, identifier-keyed rate limiting, fail-closed input
validation reusing U3's version check, object-level authorization on
every read, typed sqlx data access, typed error mapping with no leaked
driver error, no identifying field stored, secrets handling, supply
chain), `scalability-design.md` (4 elements: stateless handling,
expiry-bounded growth, shared size-cap enforcement, single-instance
sizing), `reliability-design.md` (4 elements: single-statement atomic
writes, database-exercising health check, idempotent/interval-bounded
sweep, zero-downtime redeploy via the shared external database),
`observability-design.md` (4 elements: closed-field failure row,
aggregate counters, one row per sweep cycle, readiness distinguishing
database from process), `logical-components.md` (crate-internal module
layout, dependencies on `cityloom-api-types`/`cityloom-design-payload`,
a single composition-root constructor per `team.md`'s port-and-root
corollary, explicit non-ownership boundary), and `traceability.json`.

- Looks correct
- Request changes

[Answer]: Looks correct
