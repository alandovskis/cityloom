# Reliability Design — `design-storage` (U10)

Upstream inputs: `reliability-requirements.md`, `tech-stack-decisions.md`
(nfr-requirements, this Unit), `functional-spec.md`, `rules.md`
(functional-design, this Unit), `osm-extract-proxy`'s own reliability
design precedent (this workspace's one other server Unit).

Design elements are numbered `RD-n`; `traceability.json` maps each
`NFR3.x.y` from `reliability-requirements.md` to the elements that meet
it.

## RD-1 — Atomic writes, no partial row (NFR3.4.1, BR6.1)

The upload path is a single `INSERT` statement (`performance-design.md`
PD-1) — there is no multi-statement write for a single upload to be
"partial" between, so atomicity is Postgres's own single-statement
guarantee rather than an application-managed transaction spanning
multiple writes. A failure at any point before the `INSERT` commits
(payload validation failure, a connection drop mid-request) leaves
zero rows; a failure is never observable as a half-populated row,
because no code path constructs one. `security-design.md` SD-6's error
mapping ensures a failed `INSERT` surfaces as a typed `StorageFailure`,
never a partially-applied write silently treated as success.

## RD-2 — Health check exercises the database (NFR3.2.1)

The readiness endpoint (shared router-merge point with `osm-extract-
proxy`'s own readiness route, `logical-components.md` LC-3) executes a
lightweight query (`SELECT 1`) against the pool before reporting ready;
a failed or timed-out ping reports not-ready while the process keeps
running, matching `observability-design.md` OD-4's stated distinction.
This is what makes a Railway deploy's health check a genuine
"can this instance actually serve design-storage requests" signal
rather than only "did the process start."

## RD-3 — Expiry sweep degrades safely if skipped (NFR3.4.2)

The sweep (`performance-design.md` PD-3) is idempotent and stateless
across runs — it always queries "what is expired now," never "what
changed since the last run" — so a skipped cycle (a deploy landing
mid-interval, a transient database outage) has exactly one consequence:
the next cycle finds a larger backlog and clears it in more batches. No
design ever disappears early (retention is never understated) and the
sweep interval bounds how late a design can be (`infrastructure-design`
fixes the interval at ≤ 24 hours per `reliability-requirements.md`'s
target).

## RD-4 — Zero-downtime redeploy (NFR3.1.1, NFR3.3.1)

This Unit's data lives in the shared Railway Postgres service, never an
attached volume on the application instance (`tech-stack-decisions.md`,
`components.md`'s stated external dependency) — a redeploy of the
application instance never touches the data. A schema migration that
adds a column (e.g. Stage 2's eventual use of `owner_account_id`, already
reserved nullable in `entities.md`) is additive and backward-compatible
with the previous version still serving traffic during the rollout
window RD-2's health check governs.

## Assumptions & Open Questions

- **[assumption]** The exact sweep interval, and whether it runs as a
  scheduled background task inside the same process or a separate
  Railway cron-style job, is `infrastructure-design`'s to fix — this
  design only requires whichever mechanism is chosen to be idempotent
  and interval-bounded (RD-3).

## Traceability

See `traceability.json` in this directory.

_Confirmed._ (nfr-design)
