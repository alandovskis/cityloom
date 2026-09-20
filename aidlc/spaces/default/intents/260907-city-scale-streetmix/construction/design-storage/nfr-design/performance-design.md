# Performance Design — `design-storage` (U10)

Upstream inputs: `performance-requirements.md`, `tech-stack-decisions.md`
(nfr-requirements, this Unit), `functional-spec.md`, `rules.md`
(functional-design, this Unit), `osm-extract-proxy`'s own performance
design precedent (this workspace's one other server Unit).

Design elements are numbered `PD-n`; `traceability.json` maps each
`NFR1.4.x` from `performance-requirements.md` to the elements that meet
it.

## PD-1 — One round trip per operation, no N+1

Each of the three endpoints (`upload`, `fetch`, `remove`) issues exactly
one `sqlx` query against `stored_designs`: an `INSERT` for upload (with
`ON CONFLICT (anonymous_design_id) DO NOTHING` reporting `0` rows
affected as the `409` case from `functional-spec.md`'s upload workflow
step 5), a `SELECT` for fetch, a `DELETE` for remove. No handler makes a
second query to check existence before acting — the single statement's
own result (rows affected, or a found/not-found row) is the answer,
avoiding the check-then-act round trip that would double NFR1.4.1–
NFR1.4.3's latency for no benefit.

## PD-2 — Connection pooling sized for the shared service

`sqlx::PgPool` is created once at process start (the composition root,
`logical-components.md` LC-3) with a bounded pool (`max_connections`
fixed at `infrastructure-design`, sized against the single Railway
Postgres instance's own connection ceiling — this Unit does not invent
its own number in isolation from U9's proxy, since both share the one
Railway deployment). A held connection is returned to the pool
immediately after each query; no handler holds a connection across an
`.await` point unrelated to that query (Rust's `sqlx` API makes this the
natural shape — the pool handle's lifetime is scoped to the query call).

## PD-3 — The expiry sweep runs in bounded batches, never one long transaction

NFR1.4.4 requires the sweep to hold no lock for more than 100 ms. The
sweep is a single `DELETE ... WHERE anonymous_design_id IS NOT NULL AND
expires_at < now() LIMIT <batch_size>` repeated until it deletes zero
rows, each iteration its own short transaction — never one `DELETE`
with no `LIMIT` that could hold a table-level lock for however long a
large expired set takes. `batch_size` is fixed at
`infrastructure-design` against a realistic anonymous-upload volume
(`scalability-requirements.md` NFR2.3.1's "uploads per day × 30" bound).

## PD-4 — Where the clock starts

Consistent with the `osm-extract-proxy` review's R-01 finding (measure
from the point the requirement's own text says, not a convenient
internal checkpoint), each of NFR1.4.1–NFR1.4.3's budgets is measured
from the request being received by the handler to the first response
byte being written — including whatever authorization check (BR3.1) or
validation (BR6.1) runs first. No phase is excluded from the timed
window.

## PD-5 — Benchmarked, not modelled

A CI benchmark (criterion or equivalent, run against a local Postgres
instance in the test harness — never the production database) drives
each endpoint with a representative `DesignPayload` (a multi-street
corridor design, per `performance-requirements.md`'s stated assumption)
and asserts the p95/p99 targets. Production aggregates are exported as
latency-bucket histograms alongside the operation counters
(`observability-design.md` OD-2) — never a per-request log row, matching
`osm-extract-proxy`'s own established discipline and this Unit's own
BR7.1 (no personal signal on the identifier, which a per-request latency
row keyed on it would risk becoming).

## Assumptions & Open Questions

- **[assumption]** `max_connections` and the expiry sweep's `batch_size`
  and interval are `infrastructure-design`'s to fix numerically; this
  stage only fixes the *shape* (bounded pool, bounded batch) that makes
  a later numeric choice safe rather than a source of contention.

## Traceability

See `traceability.json` in this directory.

_Confirmed._
