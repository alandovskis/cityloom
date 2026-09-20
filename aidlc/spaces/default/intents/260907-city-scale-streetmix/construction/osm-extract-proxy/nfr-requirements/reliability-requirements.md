# Reliability Requirements — `osm-extract-proxy` (U9)

Upstream inputs: `functional-spec.md`, `rules.md` and `entities.md`
(functional-design, this Unit), `requirements.md` (requirements-analysis),
`contract-summary.md` (contract-design), `nfr-requirements-questions.md`
(this stage), `team.md` and `project.md` (practices).

Ids follow `performance-requirements.md`: a third level refines an inception
requirement. Every number is provisional against the B-0 / B-3 measurements.

## The shape of the problem

This service has no persistent state. Everything it holds at run time is
either read-only and shipped with the deploy (`RegionalDataBuild`) or
memory-only and disposable (`CachedExtract`, `RequesterWindow`,
`ServiceCounters`). There is therefore no data to lose, no backup to take
and no recovery point: **recovery is a redeploy**, and the only way the
service can be wrong is to serve from a bad build — which BR9.1 refuses.

That makes availability the one reliability property that matters, and it
is `requirements.md` NFR3.1's 99.5% monthly: **3.6 hours of downtime a
month**, on a single instance with no failover (`requirements.md` A2 records
that this is assumed achievable, not proven). The two things that can spend
that budget are a platform incident, which no design here can prevent, and
a deploy that takes traffic before it is ready, which NFR3.2 and the design
below prevent.

## Service level

| ID | Refines | SLI | SLO | Error budget | Measured how |
|---|---|---|---|---|---|
| NFR3.1.1 | NFR3.1 | **Availability**: the fraction of one-minute intervals in which the readiness signal answered ready (observability NFR3.2.3). | **≥ 99.5% per calendar month.** | 216 minutes a month. | The platform's own health-check history; a month with the budget spent is reported at the gate of the next Bolt that touches this Unit, not silently absorbed. |
| NFR3.1.2 | NFR3.1 | **Success ratio**: `extractsServed` divided by (`extractsServed` + `failuresByReason[internal]` + `failuresByReason[timeout]`), over the counters (BR7.4). Client-caused failures (`invalid_area`, `area_too_large`, `area_not_found`, `rate_limited`) are correct answers, not failures, and are excluded. `upstream_unavailable` is excluded here because it is counted by NFR3.1.1: it can only occur while not-ready. | **≥ 99.9% per calendar month** once the service is Ready. | About 1 in 1,000 served requests may be `internal` or `timeout`. | Read from the periodic counters row (observability NFR3.1.12). |
| NFR3.1.3 | NFR3.1 | **Correctness**: the same canonical box on the same build yields identical bytes on every instance and every run (BR5.4). | **100%** — not a percentage but an invariant. | None. | The golden-fixture suite (`team.md`): every fixture street's served bytes equal the committed bytes on every merge. |

An SLA does not exist: there is no customer contract and no second party
(`team.md`, "There is no second human"). The SLOs above are the project's
own targets and are stricter than anything it promises anyone.

## Deployment safety

| ID | Refines | Requirement | Verified by |
|---|---|---|---|
| NFR3.2.1 | NFR3.2 | **A deploy takes traffic only when Ready.** The readiness signal answers ready only after W2 completes: manifest loaded, every cell verified (BR9.1). The platform routes to the new instance only on a passing check and keeps the previous instance serving until then, so a deploy that ships a bad build never counts against NFR3.1.1 — it fails its check and the previous deployment stays. The health check's path, interval and timeout are Infrastructure Design's; the requirement is that it reads the readiness state and nothing else. | A deploy of an artifact with a corrupted cell (NFR3.2.4's test build) must leave the previous deployment serving — confirmed at environment-provisioning by doing exactly that once. |
| NFR3.2.2 | NFR3.2 | **Ready within the platform's patience.** W2 completes inside NFR1.1.4's 30 seconds at p95 for the first region set; the health-check timeout Infrastructure Design sets must exceed the observed Ready time with margin, or a healthy build will be judged unhealthy. | The Ready lifecycle row's elapsed milliseconds (NFR1.1.4). |
| NFR3.3.1 | NFR3.3 | **No attached volume, no persistent state.** The service holds nothing that survives a restart: cells and the manifest come with the deploy artifact (or are fetched to ephemeral disk at start — `functional-spec.md`'s assumption, Infrastructure Design's choice), and every runtime entity is memory-only (BR6.4, BR7.3, BR7.4). This is what makes NFR3.2.1 a no-downtime rollover rather than a volume hand-off. | Review of the platform configuration at environment-provisioning: no volume attached to this service. |
| NFR3.1.4 | NFR3.1 | **A failed build never reaches production.** W1 publishes nothing on any failure (download, checksum, slicing, a missing region), and an unchanged `buildId` does not redeploy (BR8.5). The service therefore always runs the last good build; the weekly refresh can be late but never wrong. | Build tests (NFR7.2.2); a test that a build identical to the deployed one produces no publish action. |

## Fault tolerance

| ID | Refines | Failure | Behaviour | Verified by |
|---|---|---|---|---|
| NFR3.1.5 | NFR3.1 | **The process crashes** (a bug, an out-of-memory). | The platform restarts it; W2 runs; the cache, the windows and the counters start empty (BR6.4). In-flight requests fail at the client's own timeout (AC3.1.6). Nothing is lost because nothing was held. A crash is a defect and gets a failing regression test before the fix (NFR7.3). | The platform's restart policy, confirmed at environment-provisioning; the counters row after a restart shows `buildId` and zeroed counts. |
| NFR3.1.6 | NFR3.1 | **A phase panics on one request.** | Caught at the handler boundary and answered `503 internal` with one `FailureRecord` (BR10.4, NFR6.4.2); the process keeps serving. | The panic test named under NFR6.4.2. |
| NFR3.1.7 | NFR3.1 | **A cut exceeds the budget** (a slow disk, a slot never freed). | `503 timeout`, retryable (BR10.3, BR10.2); the abandoned cut's work is dropped and its slot released, so one slow request cannot hold a slot past the budget. | The stalled-cut test under NFR1.1.1, extended to assert the slot is free afterwards. |
| NFR3.1.8 | NFR3.1 | **The cache is full.** | Least-recently-used entries are evicted until the new clip fits (BR6.2); a clip larger than the ceiling is served and not stored. Never a failure. | Tests: fill past the ceiling and assert the oldest key is gone and the total is under the ceiling; an oversize clip is served with `200` and absent from the cache. |
| NFR3.1.9 | NFR3.1 | **The manifest names a cell that is missing or corrupt** at start. | Unready; every extract request is `503 upstream_unavailable` (retryable); the readiness signal is not-ready so a rolling deploy leaves the previous instance serving (NFR3.2.1). There is no Ready → Unready transition at run time: the data is read-only and verified once (`functional-spec.md`, "Service readiness"). | NFR3.2.4's tests. |
| NFR3.1.10 | NFR3.1 | **A covered cell has no file** (covered but empty, `entities.md` `Cell` constraint). | Not a failure: the cut reads the touched cells that exist (BR5.1); a clip with no ways is `404 area_not_found` "nothing mapped here" (BR4.2). | A test with a covered, empty cell in the touched set. |
| NFR3.1.11 | NFR3.1 | **The platform is down.** | Nothing this Unit can do; the downtime counts against NFR3.1.1. `requirements.md` A2 already records the single-environment assumption; no failover is designed, because a second environment is a constraint change (`project.md`). | — |

## Recovery objectives

| Objective | Value | Why |
|---|---|---|
| RPO (data loss window) | **Not applicable** | No persistent state exists in this Unit. |
| RTO (time to restore service) | **≤ 10 minutes** for a redeploy of the last good build (provisional: platform build and rollover time plus NFR1.1.4). Rollback uses the platform's deployment history (`team.md`, Deployment). | The only recovery action is a redeploy; it is bounded by the platform's build time, which Environment Provisioning measures. |
| MTTR | Measured operationally; the target is below RTO | There is no on-call and no second person; a failure outside the maintainer's hours is restored when they see the platform's notification. This is the honest reading of `team.md`'s solo-builder constraint, and it is why the SLO is 99.5% and not higher. |

## Graceful degradation

There is one degraded mode and it is deliberate: **Unready**. A service
that cannot prove its data is intact answers every extract request with a
retryable `503 upstream_unavailable` (BR9.1) rather than serving a partial
or wrong map. The client surfaces this through FR6.1's message and FR6.2's
blank cross-section, so no street is a dead end even while the proxy is
down. There is no "serve what we have" mode; a wrong street presented as
real is the failure `raid-log.md` R-2 names as the most damaging to
credibility, and the cheapest guard against it is refusing to serve.

## Cost guard

| ID | Refines | Requirement |
|---|---|---|
| NFR5.1.6 | NFR5.1 | **The account usage limit is the last line of defence for the budget.** Because functional design Q4 rejected a per-service egress ceiling, the only hard stop on spend is the platform's account-level usage limit, which stops services when reached. Environment Provisioning sets it at the budget (`project.md`: growth past ~$5 a month is a constraint change); this Unit's contribution is to stay inside its allocation (NFR5.1.5) so the limit is never what stops it. |

## Assumptions & Open Questions

- 99.5% on one instance with no failover is achievable — `requirements.md`
  A2, carried unchanged; a platform incident has no fallback. [assumption]
- The platform's rolling deploy really does keep the previous instance
  serving until the new one passes its check; confirmed at
  environment-provisioning by deploying a deliberately bad build. [assumption]
- 10 minutes RTO assumes a cached Rust build on the platform; a cold build
  of the workspace may take longer, which is Environment Provisioning's to
  measure. [assumption]
