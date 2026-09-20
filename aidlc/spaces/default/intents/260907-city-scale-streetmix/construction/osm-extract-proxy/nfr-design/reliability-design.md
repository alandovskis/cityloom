# Reliability Design — `osm-extract-proxy` (U9)

Upstream inputs: `reliability-requirements.md`, `performance-requirements.md`,
`security-requirements.md` and `tech-stack-decisions.md` (nfr-requirements,
this Unit), `functional-spec.md`, `rules.md` and `entities.md`
(functional-design, this Unit), `contract-summary.md` (contract-design),
`nfr-design-questions.md` (this stage), `team.md` (practices).

Design elements are numbered `RD-n`; `traceability.json` maps each
`NFRx.y.z` from `reliability-requirements.md` to the elements that meet it.

## What there is to be reliable about

`reliability-requirements.md` puts it plainly: no persistent state, so no
backup, no replication, no recovery point; availability is the one property,
and the two ways to lose it are a platform incident and a deploy that takes
traffic before it is ready. This design therefore has no circuit breaker,
no retry policy, no failover and no replication — each of those exists to
survive a dependency, and this Unit has none at request time. What it has
instead is a careful start-up, a single degraded state, a bounded
per-request failure boundary, and a clean shutdown.

## Design elements

### RD-1 — Start-up as a state machine with one exit into Ready

`functional-spec.md`'s readiness machine (Starting → Ready | Unready) is
implemented as a sequence that can fail at any step and, on failure,
leaves an atomic Ready flag down:

```
start
  |- load configuration ............ invalid -> exit non-zero (never Unready: a
  |                                   misconfigured process is a deploy bug)
  |- mint the limiter key (BR2.3)
  |- load the manifest ............. missing/unreadable -> Unready
  |- open the store read-only ...... missing -> Unready
  |- verify every cell's range ..... any digest mismatch -> Unready
  |- build index + coverage bitmap
  |- create cache, windows, counters (all empty, bound to buildId)
  |- start the periodic tasks (counters row, window sweep)
  `- set Ready; emit the Ready row
```

<!-- Text fallback: start-up loads configuration and exits non-zero if it is invalid; mints the limiter key; loads the manifest and opens the store, entering Unready if either is missing; verifies every cell's digest, entering Unready on any mismatch; builds the index and coverage bitmap; creates the empty cache, windows and counters bound to the buildId; starts the periodic tasks; then sets the Ready flag and emits the Ready row. -->

Unready is a running process that answers every extract request with
`503 upstream_unavailable` (BR9.1) and whose readiness endpoint answers
503 (NFR3.2.4, NFR3.1.9). It does not exit and does not retry, because a
bad artifact does not become good by waiting; the fix is a redeploy. The
distinction between "exit non-zero" and "Unready" is deliberate: a
configuration error should fail the deploy loudly at the platform, while a
data error should keep the process alive so the previous deployment keeps
serving under the platform's rollover (RD-2).

There is no Ready → Unready transition: the store is verified once and
opened read-only; nothing at run time can invalidate it (`functional-
spec.md`, "Service readiness"). A read error on a verified range at request
time is treated as a fault of that request (`internal`, RD-4), not as a
state change.

### RD-2 — Readiness-gated rollover

The readiness endpoint (`security-design.md` SD-8) is the only signal the
platform's health check reads (NFR3.2.1, NFR3.2.3). Under the platform's
deploy model — the new instance takes traffic only after its check passes,
the previous instance serving until then — a deploy carrying a bad store
fails its check and the previous deployment stays up. The check's path,
interval and timeout are Infrastructure Design's; this design requires two
properties of them: the timeout exceeds the measured Ready time with margin
(NFR3.2.2, NFR1.1.4), and the check reads the flag only (no build id, no
dependency probing — there is nothing to probe).

### RD-3 — Deadline as the only load-shedding, slot as the only bulkhead

One deadline per request (`performance-design.md` PD-4) bounds every wait:
for a permit, for an in-flight cut, for the cut itself. The four cutting
permits (PD-3) are the bulkhead between the cheap path (hits, failures,
readiness) and the expensive one: however many misses arrive, at most four
cuts run and the async runtime stays responsive. An abandoned cut cancels
itself at the next cell boundary and releases its permit (NFR3.1.7). There
is no separate queue depth, no shed-load policy and no priority scheme
because the deadline already expresses the only policy that matters: a
request either completes inside the user's budget or fails retryably.

### RD-4 — Per-request failure boundary

Every failure in W3 is mapped by one function to a `FailureReason`, a
status and a project-authored detail (BR10.1), which writes exactly one
failure row (`observability-design.md` OD-3) and returns the typed
response. Two edges are designed explicitly:

- **Panics** are caught at the handler boundary by the catch-panic layer
  (`security-design.md` SD-2) and mapped to `internal`, phase `unexpected`
  (BR10.4, NFR3.1.6). The process keeps serving; the panic is a defect and
  gets a regression test before the fix (NFR7.3.1).
- **The single-flight loader's error** is delivered to every waiter on the
  same key; each waiter maps it and answers its own response, and nothing
  is cached (BR6.2, NFR3.1.8 for the full-cache case, NFR3.1.10 for the
  empty case).

### RD-5 — Cache full and cache cold are not failures

The cache evicts least-recently-used entries by weight until a new clip
fits (NFR3.1.8); an oversize clip is served and not retained. A cold cache
— every process start, every deploy (BR6.4) — costs cuts, not
availability; the slots (RD-3) bound the cost of the first minutes after a
deploy.

### RD-6 — Process restart

A crash is left to the platform's restart policy (NFR3.1.5). Nothing is
persisted, so nothing is corrupted by a crash; the new process runs RD-1
from the top and serves a cold cache. The counters restart at zero with the
same `buildId`, which the reader of the counters row sees as a discontinuity
in `uptimeSeconds`.

### RD-7 — Graceful shutdown

On `SIGTERM` (the platform's stop signal on redeploy): stop accepting
connections; give in-flight requests up to the request budget (3 s) to
finish — after which they fail at the client's own timeout, which is
already the AC3.1.6 contract; emit the final counters row (OD-2); exit
zero. The platform's rollover means the new instance is already serving
before the old one receives the signal, so the window in which a request
can be lost is the 3 seconds after switch-over, and only for a request
that was already slower than the p99 target.

### RD-8 — The build never publishes a partial result

The region-build tool writes the store and manifest to a temporary location
and moves them into place only after every region is sliced and every
digest is written; any failure earlier — a download, a checksum, a slicing
error, a missing configured region — leaves the previous build untouched
(NFR3.1.4, BR8.1, BR8.4). An unchanged `buildId` publishes nothing (BR8.5).
How "into place" reaches the deploy is Infrastructure Design's; the
invariant is that the deploy only ever sees a whole build.

### RD-9 — Recovery is redeploy

RPO does not exist; RTO is the platform's build-and-rollover time plus
NFR1.1.4 (`reliability-requirements.md`, ≤ 10 minutes provisional). The
rollback path is the platform's deployment history (`team.md`, Deployment):
redeploying the previous build is the same RD-1 sequence on the previous
artifact. Nothing in this Unit needs a runbook beyond "redeploy the last
good build and read the failure rows".

## Failure mode checklist (from the NFR design guide)

| Question | Answer |
|---|---|
| What happens when this component is unavailable? | Every import falls to FR6.1's message and FR6.2's blank cross-section in the client; nothing else in the product depends on it at run time. |
| What happens when response time doubles? | Hits stay well inside 30 ms; misses at 600 ms p95 still complete inside the 3 s budget; NFR1.1.2 is missed and the buckets show it. |
| What happens when throughput exceeds capacity? | Misses wait for permits and time out retryably; hits are unaffected. |
| What happens when a dependency returns corrupted data? | The only dependency is the store, verified at start; a corrupted download never becomes a store (RD-8). |
| Is there a graceful degradation path? | One: Unready, which is refusing to serve — deliberately the only degraded mode (`reliability-requirements.md`, "Graceful degradation"). |
| Blast radius of a failure? | The single instance, which is the whole import path; degraded in the client, never wrong. |
| Recovery procedure? | Automatic restart for a crash; redeploy for a bad build; nothing manual beyond reading the rows. |

## Rejected alternatives

- **Retrying a failed cut internally.** A cut fails for deterministic
  reasons (empty, oversize, fault); retrying repeats them. Retries are the
  client's (AC7.2.1) for the retryable set only.
- **A "serve stale" mode when the store fails verification.** A wrong street
  served as real is the failure `raid-log.md` R-2 names; refusing is the
  reliable answer.
- **Health-checking dependencies.** There are none; a deep check would
  probe nothing and could only add a false-negative path.

## Assumptions & Open Questions

- The platform delivers `SIGTERM` and allows at least the 3-second drain
  before killing the process; confirmed at environment-provisioning.
  [assumption]
- The platform's restart policy restarts a crashed process without a manual
  step; confirmed at environment-provisioning. [assumption]
