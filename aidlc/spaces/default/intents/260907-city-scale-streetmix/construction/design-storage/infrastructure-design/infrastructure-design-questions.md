# Infrastructure Design — Questions — `design-storage` (U10)

No open question remains for this Unit: the six confirmed `nfr-design`
artifacts, `functional-spec.md`, `components.md`, and `osm-extract-proxy`'s
own established Railway/shared-service precedent together determine the
infrastructure decisions this Unit needs.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifacts?

Produces (all four, per this Unit's `service` kind):
`infrastructure-specification.md` (this Unit's router merges into U9's
existing shared Railway service — no new compute service; one new Railway
resource, a managed PostgreSQL plugin, on Railway's internal network only;
numeric decisions fixed here: `max_connections = 10`, a 6-hour/500-row
expiry sweep as an in-process task, rate-limiter thresholds of
10/minute–100/hour per identifier, a 5 MiB payload cap; a provisional
≤ $1.25/month cost estimate on top of U9's own ≤ $2.50, for a combined
≤ $3.50 of the $5 workspace hard limit), `monitoring-design.md` (this
Unit's signals land in U9's own log/monitor pipeline; no new alert or
monitor — the shared `/readyz` and U9's existing three alerts already
cover this Unit once extended to also ping the database), `cicd-pipeline.md`
(this Unit's own crate joins the existing fast/slow tiers plus a Postgres
CI service container for real-database tests; one genuinely new pipeline
step — `sqlx::migrate!` at process start, gated the same way a failed
build already is), and `traceability.json`.

- Looks correct
- Request changes

[Answer]: Looks correct