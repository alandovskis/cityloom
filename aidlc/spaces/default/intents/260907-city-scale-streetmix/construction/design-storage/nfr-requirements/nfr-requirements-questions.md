# NFR Requirements — Questions — `design-storage` (U10)

No open question remains for this Unit: `requirements.md` NFR1/2/3/5/6/7,
`functional-spec.md`, and `rules.md` together determine the applicable
requirements across all six categories this Unit's `service` kind
requires.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifacts?

Produces (all six, per this Unit's `service` kind):
`performance-requirements.md` (4 provisional latency targets for
upload/fetch/remove/expiry-sweep, proposed NFR1.4), `security-requirements.md`
(STRIDE table with 7 threats, 7 requirements, proposed NFR6.4),
`scalability-requirements.md` (3 requirements on storage-growth
bounding, proposed NFR2.3), `reliability-requirements.md` (5
requirements on availability/atomicity/expiry resilience, proposed
NFR3.4), `observability-requirements.md` (4 requirements following
`osm-extract-proxy`'s no-per-request-row discipline, proposed NFR7.4),
`tech-stack-decisions.md` (new crate `cityloom-design-storage`; reuses
`osm-extract-proxy`'s exact axum/tokio pins; adds `sqlx` for PostgreSQL;
depends on `cityloom-api-types` and `cityloom-design-payload`), and
`traceability.json`.

- Looks correct
- Request changes

[Answer]: Looks correct
