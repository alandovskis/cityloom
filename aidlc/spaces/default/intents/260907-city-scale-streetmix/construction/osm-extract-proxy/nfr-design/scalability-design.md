# Scalability Design — `osm-extract-proxy` (U9)

Upstream inputs: `scalability-requirements.md`, `performance-requirements.md`
and `tech-stack-decisions.md` (nfr-requirements, this Unit),
`functional-spec.md` and `entities.md` (functional-design, this Unit),
`contract-summary.md` (contract-design), `nfr-design-questions.md` (this
stage).

Design elements are numbered `SC-n`; `traceability.json` maps each
`NFRx.y.z` from `scalability-requirements.md` to the elements that meet it.

## Three axes, three mechanisms

`scalability-requirements.md` names three things that grow — requests,
data, instances — and this design gives each its own mechanism so growth on
one axis never forces a change on another.

| Axis | Grows by | Mechanism | Ceiling at Stage 1 |
|---|---|---|---|
| Requests | more people, more clicks | per-requester windows (SC-1), cutting slots (SC-2), the cache (SC-3) | ~5 requests/s aggregate, ~4 misses/s sustained (NFR2.1.1) |
| Data | more regions | configuration + a weekly build + a bigger store (SC-4, SC-5) | one province; the store and index scale linearly |
| Instances | a second replica | nothing to add: every runtime structure is per-process (SC-6) | one instance |

## Design elements

### SC-1 — Per-requester windows as the first admission control

The limiter of `security-design.md` SD-4 is also the load shape: it caps
any one requester at 30 per minute and 300 per hour (NFR5.1.3), which
bounds both egress (each served clip is paid bytes) and the rate at which
one client can drive misses into the slots. Memory is 64 bytes per live
requester and the map is swept every minute (NFR2.1.3), so ten thousand
requesters in an hour cost under a megabyte.

### SC-2 — Cutting slots as the only queue

Four permits on one `tokio::sync::Semaphore` (NFR1.1.6, NFR2.1.2). There is
no request queue with its own depth and no load-shedding policy to tune: a
miss that cannot get a permit waits, inside its own 3-second deadline, and
fails as `timeout` if the wait outlasts it (NFR1.1.1). Hits never take a
permit, so a flood of misses cannot starve the common path. The permit
count is a configuration value so B-3's measurements can move it without a
code change; the design assumption is one shared vCPU and roughly 100 ms of
CPU per cut.

### SC-3 — The cache as the request-axis multiplier

Within a session, repeated requests for the same street are hits, and hits
cost a lookup (`performance-design.md` PD-5). The cache's 32 MB holds
several hundred streets at the expected clip size, which is more than one
person looks at in a session; its purpose is latency and CPU, not egress
(`functional-spec.md`, "Caching does not reduce egress") — every response
is paid bytes regardless, which is why SC-1 exists.

### SC-4 — Data growth is a build, not a change

Adding a region is one configuration entry and one run of the region-build
tool (`logical-components.md` LC-3). The store, the manifest, the index
and the coverage bitmap all scale linearly with populated cells (NFR2.1.4,
NFR2.1.5):

| Structure | Per cell | One province (~200,000 cells) | Where |
|---|---|---|---|
| Store entry (compressed PBF) | tens to hundreds of KB, dense; ~1 KB, rural | ≤ 500 MB (NFR2.1.6) | disk |
| Manifest row (JSON) | ~100 bytes | 15–20 MB | disk, parsed at start |
| Index entry | 24 bytes | ~5 MB | memory |
| Coverage bitmap | 1 bit per grid cell in the regions' rectangle | < 1 MB | memory |

The bitmap is sized by the regions' bounding rectangle, not by the province
count; two adjacent provinces share one rectangle. A second province
roughly doubles the store, the manifest and the index, and start-up time
with them (NFR1.1.4's target is for the first region set).

### SC-5 — The build's own scale

The region-build tool must hold a province's kept node ids while slicing
(TS-9, two-pass filter): tens of millions of ids. Design: a compact
integer set (a sorted, delta-compressed id list or a bitmap over id ranges,
chosen at code generation) rather than a hash set of `i64`, so the pass
fits a free CI runner's memory with room to spare (NFR2.1.6, ≤ 30 minutes).
The store is written in one sequential pass in cell-id order, which is also
the order the service verifies it in (PD-7).

### SC-6 — Horizontal scaling: possible, not planned, and it changes one number

Every runtime structure — the cache, the windows, the counters, the
histograms, the Ready flag — is per-process and needs no coordination
(NFR2.1.3 rows, `entities.md` "Held in memory while the service runs"). A
second replica would therefore work unchanged, with three consequences
`scalability-requirements.md` already records: the per-requester allowance
doubles unless the configured limits are divided by the replica count
(the only option consistent with BR7.3, which forbids a shared requester
store); memory and CPU cost double, which is a constraint change under
`project.md`; and the counters become per-replica and must be summed by
the reader. None of this is built now.

### SC-7 — Cell packing keeps the artifact count flat

Q1's packed store means the number of files in the deploy artifact is two
regardless of region count (NFR2.1.7): the store and the manifest. The
count that made per-cell files unattractive — hundreds of thousands per
province — never reaches the image, the fetch or the start-up directory
walk.

## Capacity thresholds and what happens at them

| Threshold | Signal | What happens | Designed response |
|---|---|---|---|
| A requester exceeds 30/min or 300/h | `requestsLimited` rises | `429` with `Retry-After` | Nothing; working as designed (SC-1) |
| Misses exceed ~4/s sustained | miss latency buckets shift right; `timeout` failures appear | requests wait for permits and some time out | Raise the permit count if CPU allows, or the service size; both are configuration (SC-2) |
| Cache hit ratio falls below ~50% in a session | `cacheHits` / (`cacheHits` + `cacheMisses`) | more cuts per user | Raise the ceiling (a configuration value; costs memory at ~$10/GB-month) |
| Process memory approaches 192 MB | platform metrics | nothing automatic | Investigate: cache ceiling, manifest size, a leak; a service-size change is a constraint change |
| A second province is configured | `cellCount` doubles in the Ready row | start-up time grows | Re-check NFR1.1.4 against the measured Ready row before the next region |

## Rejected alternatives

- **A shared cache (the database service, or a cache service).** Rejected
  at functional design Q2; it would also reintroduce a dependency at request
  time and the breaker/retry machinery this Unit is glad not to need.
- **Per-endpoint or global rate limiting with a token bucket.** A global
  bucket punishes every user for one abuser; the per-requester windows are
  the fair shape, and the slots already bound total CPU.
- **Sharding the store by tile (Q1 option C).** Bounded file count was the
  only benefit, and a single store bounds it better.

## Assumptions & Open Questions

- The ~100 ms CPU per cut behind the four-permit figure is unmeasured;
  B-3's miss-latency buckets are the first real number, and the permit
  count is configuration so it can follow. [assumption]
- The compact id set for the build fits a 7 GB runner for the largest
  Canadian province (British Columbia at 1.2 GB of source); if it does not,
  the build slices per region file in turn rather than all at once — a
  code-generation choice with no design consequence. [assumption]
