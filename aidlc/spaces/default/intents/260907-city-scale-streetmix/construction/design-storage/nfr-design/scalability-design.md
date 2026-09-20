# Scalability Design — `design-storage` (U10)

Upstream inputs: `scalability-requirements.md`, `tech-stack-decisions.md`
(nfr-requirements, this Unit), `functional-spec.md`, `rules.md`
(functional-design, this Unit).

Design elements are numbered `SC-n`; `traceability.json` maps each
`NFR2.3.x` from `scalability-requirements.md` to the elements that meet
it.

## SC-1 — Stateless request handling

The only mutable state a running instance holds outside the database is
`security-design.md` SD-2's rate-limiter map, which is per-identifier
bookkeeping, not shared application state — two instances each holding
their own copy under-counts an attacker's true request rate slightly
(a client alternating between instances gets a fresh window on each) but
never over-counts a legitimate client's, so it degrades safely rather
than unsafely under horizontal scaling (NFR2.3.3). No handler holds a
per-request value across calls, and no in-memory cache of `StoredDesign`
rows exists — every read goes to the database, which is the single
source of truth. This satisfies NFR2.3.3's "no cross-request
coordination state" without requiring a distributed rate-limiter store
(e.g. Redis) that this project's cost budget (`project.md` OC-4) does
not otherwise justify.

## SC-2 — Growth bounded by expiry, not by a ceiling

There is no maximum-row-count check in this Unit's own logic (BR2.1's
expiry mechanism is the growth bound, not a hard cap that would start
rejecting uploads once reached). `performance-design.md` PD-3's batched
sweep is what keeps steady-state storage proportional to daily upload
volume rather than growing unboundedly (NFR2.3.1). The stored-row-count
metric (`observability-design.md` OD-2) is how a human notices if actual
growth diverges from the expected bound — a monitoring signal, not an
enforcement mechanism, consistent with `scalability-requirements.md`'s
own "reviewed monthly against the budget" verification method.

## SC-3 — Per-row size bounded at the same boundary that rejects it

The payload size cap (`security-design.md` SD-3) is the same check that
bounds both DoS risk (T3) and per-row storage growth (NFR2.3.2) — one
mechanism serving two requirements, rather than a separate storage-quota
check duplicating the security boundary's own validation.

## SC-4 — Single-instance sizing, not sharded

`tech-stack-decisions.md` already fixes this Unit as one crate merged
into the shared single-Railway-service deployable (`unit-of-work.md`'s
"four Units, one deployable"). No sharding, read-replica, or
multi-region design is proposed here — `scalability-requirements.md`'s
own stated assumption is that Stage 1's realistic volume does not need
it, and introducing one now would be scaling a problem that does not yet
exist, against `team.md`'s "no performance/load testing without a stated
NFR target" practice.

## Assumptions & Open Questions

- **[assumption]** If this Unit's assumption about realistic anonymous-
  upload volume proves wrong (a genuine viral-growth scenario), the
  first scaling lever is a connection-pool and instance-count increase
  within Railway's existing single-service topology — not a
  re-architecture — and that increase is itself routed through
  `project.md`'s "budget growth is a constraint change" rule, per
  `scalability-requirements.md`.

## Traceability

See `traceability.json` in this directory.

_Confirmed._ (nfr-design)
