# Performance Requirements — `osm-extract-proxy` (U9)

Upstream inputs: `functional-spec.md`, `rules.md` and `entities.md`
(functional-design, this Unit), `requirements.md` (requirements-analysis),
`contract-summary.md` (contract-design), `nfr-requirements-questions.md`
(this stage), `team.md` (practices).

Every requirement below refines one inception requirement and carries a
third-level id (`NFR1.1.1` refines `NFR1.1`), because `requirements.md`
already uses the two-level form for its own sub-requirements. Every number
is **provisional**: it is the value the proxy is built and tested against,
and B-0 / B-3 measure the real figure. A measured figure replaces a
provisional one through the normal change path — an amendment recorded
against this file — never by silent absorption (`project.md`, budget rule).

## Budget allocation

`requirements.md` NFR1.1 gives a full osm2streets import of one street
10 seconds end to end. That budget crosses four Units; this stage fixes only
this Unit's share and records the split so the others can take theirs.

| Leg | Owner | Share of NFR1.1 | Fixed where |
|---|---|---|---|
| Browser to proxy, request | network | inside the 7 s remainder | — |
| **Proxy: validate, cover, cut, encode, respond** | **U9 (this Unit)** | **3 s hard budget (Q2)** | NFR1.1.1 below |
| Proxy to browser, response bytes | network | inside the 7 s remainder | — |
| osm2streets conversion | U4 `street-import` | its own NFR stage; AC3.1.2 | U4 `nfr-requirements` |
| Render the cross-section | U6 `client-surfaces` | its own NFR stage; AC3.1.3 | U6 `nfr-requirements` |

The proxy's share is deliberately small: with cells pre-cut on local disk
(functional-spec W3), its own work is reading at most twelve files, merging,
de-duplicating and encoding. The 3-second hard budget exists so that a stuck
request can never consume the user's whole 10 seconds (BR10.3, AC3.1.6); the
p95 targets are what the proxy is expected to do, not what it is allowed.

## Requirements

| ID | Refines | Requirement | Target | Measured how |
|---|---|---|---|---|
| NFR1.1.1 | NFR1.1 | **Request budget.** A request that has not begun its response within the budget is abandoned and answered `503 timeout` (BR10.3, BR10.2 retryable). The budget covers every phase of W3, including waiting for an in-flight cut (BR6.3) and waiting for a cutting slot (NFR2.1.2). | Hard budget **3,000 ms**, measured from the request being received by the handler to the first response byte being written. | Applied in the handler with a single deadline, so the mapped reason is `timeout`, not a framework-generic status. A unit test stalls the cutting phase past the deadline and asserts `503 timeout` within 3,000 ms + 50 ms. |
| NFR1.1.2 | NFR1.1 | **Cache-miss latency.** A request whose key is not cached (W3 steps 9–13: cut, encode, store, respond) completes inside the target, for a box touching at most the span bound (NFR1.1.5). | **p95 ≤ 300 ms, p99 ≤ 1,000 ms** at the proxy, at up to four concurrent cuts, on the Railway service size Infrastructure Design selects. | Production: aggregate latency buckets (see "Latency measurement" below). CI: a benchmark over committed cells runs 200 misses on the CI runner and fails if p95 exceeds 300 ms — a regression guard on the code path, not evidence about production hardware. |
| NFR1.1.3 | NFR1.1 | **Cache-hit latency.** A request whose key is cached (W3 step 8 to 13) completes inside the target. | **p95 ≤ 30 ms, p99 ≤ 100 ms** at the proxy. | Same two mechanisms; the CI benchmark runs 1,000 hits. |
| NFR1.1.4 | NFR1.1 | **Time to Ready.** From process start, the service loads the manifest, verifies every listed cell's digest (W2, BR9.1) and enters Ready. Until then every extract request is `503 upstream_unavailable`, and the readiness signal (observability NFR3.2.3) is not-ready. | **≤ 30 s at p95** for the first configured region set (one Canadian province, on the order of 150,000–200,000 populated cells at the 0.01° grid). | One log row at the Ready transition carrying elapsed milliseconds since process start and the cell count verified (a lifecycle row, not a per-request row; permitted by BR7.4). |
| NFR1.1.5 | NFR1.1 | **Bounded work per miss.** The cells read for one request are exactly the touched cells that exist (BR5.1), and the touched-cell count is bounded by the span bound (BR3.3). This is what makes NFR1.1.2 achievable on a shared vCPU. | **Cell size 0.01°** (`GridSpec.cellSizeDegrees`, an exact divisor of 1); **span bound 12 cells**. At the grid, one request reads at most 12 cell files, on the order of 1–2 MB of PBF in a dense city cell set. | Unit tests on the span computation: a box straddling a cell corner touches 4 cells; a 13-cell rectangle is `400 area_too_large`; a box exactly on a cell edge touches only the cells it overlaps by area. |
| NFR1.1.6 | NFR1.1 | **Concurrent cuts bounded.** At most four clips are being cut at any moment; a fifth miss waits for a slot inside its own budget (NFR1.1.1), so overload becomes `timeout` rather than an unbounded queue or an out-of-memory. Hits are not subject to the slot. | **4 concurrent cuts** (provisional; sized to one shared vCPU with ~100 ms of CPU per cut). | A test starts eight misses against a stalled cutting phase and asserts that no more than four are cutting at once and the rest either complete after a slot frees or time out. |
| NFR5.2.1 | NFR5.2 | **Per-request server compute is bounded and cached.** NFR5.2 says per-user computation runs in the browser; `decisions.md` ADR-001 (divergence 7) records that the proxy is the exception and that NFR5.2 needs restating. The exception is bounded: a hit costs no cutting, a miss costs one bounded cut (NFR1.1.5), identical concurrent misses share one cut (BR6.3). | Per miss: **≤ 100 ms CPU typical** on the selected service size (provisional). Average CPU for the service: see `scalability-requirements.md` NFR5.1.4. | The CI benchmark records CPU time per miss alongside wall time; production CPU is read from the platform's usage metrics, which carry no per-request data. |

## Resource constraints

| ID | Refines | Constraint | Value |
|---|---|---|---|
| NFR5.1.1 | NFR5.1 | Cache ceiling (BR6.2), by total `CachedExtract.sizeBytes`. | **32 MB**. A clip larger than the ceiling is served but never stored, so the ceiling is never exceeded by a single entry. |
| NFR5.1.2 | NFR5.1 | Whole-process resident memory: cache, loaded manifest, coverage index, requester windows, per-request working memory, runtime. Within it, the per-request working memory during a cut (decoded cells plus the assembled clip) is bounded separately so four concurrent cuts cannot push the process past the target. | **≤ 128 MB typical, ≤ 192 MB at p99** over a month; **≤ 8 MB per cut** at the span bound, so ≤ 32 MB in flight with four cuts. Railway meters memory in use at about $10 per GB-month (checked 2026-09-11, `nfr-requirements-questions.md`), so 128 MB is about $1.30 a month of the $5. |

## Canonicalisation precision

BR3.2 requires a fixed precision so equal areas produce equal keys; the value
is fixed here.

- **5 decimal places** in WGS84 degrees, about 1.1 m at the equator and less
  elsewhere — finer than any bounding box a street's geometry produces, so
  two requests for the same street from the same client build are identical
  after rounding, and coarse enough that floating-point formatting noise
  cannot produce two keys for one box.
- **Rounded outward**: minimums are rounded down and maximums rounded up to
  the precision. Canonicalisation therefore never shrinks a box, so a way
  whose extent touched the requested box still touches the canonical one
  (BR5.2).
- The canonical box is what the span (BR3.3), the coverage check (BR4.1),
  the extract key (BR6.1) and the cache (BR6.2) see; the raw box is used for
  nothing after step 4 of W3.

## Latency measurement

BR7.4 permits aggregate measurement only and forbids a per-request row
anywhere, and BR7.2 fixes a failure row's fields. So a successful request
emits nothing, and p95 cannot be computed from a request log because there is
none. The production measurement for NFR1.1.2 and NFR1.1.3 is therefore
**aggregate latency buckets held in memory and exported with the counters**
(observability NFR3.1.12 and NFR3.1.14): for each of hit and miss, a fixed
set of duration buckets (≤ 10, 25, 50, 100, 250, 500, 1,000, 3,000 ms and
over) holding counts. Percentiles are read from the buckets; nothing is
written per request.

`entities.md` fixes `ServiceCounters` without latency fields. Adding the
buckets is an aggregate-only change consistent with BR7.4 and is recorded
under "Amendments required" rather than made silently.

## Amendments required

Recorded rather than edited, because the artifacts are approved — the
treatment `functional-spec.md` and `contract-summary.md` already use.

| Artifact | What must change | Why |
|---|---|---|
| `entities.md` `ServiceCounters` | Gains `hitLatencyBuckets` and `missLatencyBuckets` (fixed bucket bounds, a count per bucket), aggregate only | NFR1.1.2 and NFR1.1.3 have no production measurement otherwise; BR7.4 forbids the per-request alternative |
| `requirements.md` NFR5.2 | Restated to name the proxy as the bounded, cached exception | Already recorded as `decisions.md` ADR-001 divergence 7 and still outstanding; NFR5.2.1 above is the bound that restatement needs |

## Assumptions & Open Questions

- A dense city cell at 0.01° holds on the order of 100 KB of filtered PBF,
  so twelve cells decode in well under 100 ms. Unmeasured; B-3's first cut
  over real cells replaces it. [assumption]
- The Railway service size (shared vCPU, memory) is selected at
  infrastructure-design; the p95 targets assume one shared vCPU. [assumption]
- A single street's canonical box at 5 decimal places is stable across
  requests because the client derives it from the same node coordinates each
  time; if the client ever adds a margin computed in metres, the rounding
  must be applied after the margin, or keys will drift. [assumption]
- The 30-second Ready target scales with the region set; a second province
  roughly doubles it. Whether cells ship inside the image or are fetched to
  ephemeral disk at start (functional-spec assumption) changes the figure and
  is Infrastructure Design's. [assumption]
