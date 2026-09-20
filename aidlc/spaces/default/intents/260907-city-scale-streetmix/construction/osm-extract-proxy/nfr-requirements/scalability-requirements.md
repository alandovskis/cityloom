# Scalability Requirements — `osm-extract-proxy` (U9)

Upstream inputs: `functional-spec.md`, `rules.md` and `entities.md`
(functional-design, this Unit), `requirements.md` (requirements-analysis),
`contract-summary.md` (contract-design), `nfr-requirements-questions.md`
(this stage), `team.md` (practices).

Ids follow `performance-requirements.md`: a third level refines an inception
requirement. Every number is provisional against the B-0 / B-3 measurements.

## What scales, and what does not

The proxy has three dimensions that can grow, and they grow differently:

- **Requests**: how many people are looking at streets at once. Bounded per
  requester by Q1 (NFR5.1.3) and per instance by the cutting slots
  (NFR1.1.6); a single instance carries product Stage 1.
- **Data**: how many regions the deployment carries (`RegionalDataBuild`).
  Grows by configuration and a weekly build (W1), not by code; each region
  adds cells, artifact size and start-up verification time.
- **Instances**: all runtime state is per-process (`CachedExtract`,
  `RequesterWindow`, `ServiceCounters` — `entities.md`, "Held in memory
  while the service runs"), so a second replica needs no coordination. It is
  not planned for Stage 1 and it changes the meaning of the per-requester
  limit (below).

Egress does **not** scale down with the cache — a hit is served bytes just
as a miss is (`functional-spec.md`, "Caching does not reduce egress"). The
only things that bound egress are the per-requester limit, the span bound,
and PBF encoding (BR5.5).

## Load handling

| ID | Refines | Requirement | Target | Measured how |
|---|---|---|---|---|
| NFR5.1.3 | NFR5.1 | **Per-requester limit (the numbers behind BR2.1).** Two fixed windows per `requesterHash`: a minute window and an hour window, each a start time and a count. A request that has reached either limit is `429 rate_limited` (retryable, BR10.2); hits count exactly as misses (BR2.2). A window is discarded when it elapses (BR7.3). | **30 per minute and 300 per hour** per requester. Bound per relentless client: 7,200 clips a day; at 150 KB per clip about 1.08 GB a day and, at $0.05 per GB, about **$1.60 a month** for one client running flat out. | Unit tests: the 31st request in a minute is `429`; the 301st in an hour is `429` even when the minute window is clear; a hit counts; a new minute window resets the minute count but not the hour count; a window absent for longer than its length is gone. |
| NFR2.1.1 | NFR2.1 | **Concurrency.** The service sustains the expected Stage 1 concurrency with the latency targets of `performance-requirements.md` holding. | **20 concurrent users** — about 5 requests per second aggregate, of which most are hits after the first minutes of a session. Ceiling before `timeout` appears: about **4 misses per second sustained** (four slots at ~100 ms of CPU, NFR1.1.6) and about **50 hits per second** (provisional). | The CI benchmark runs a mixed load (1 miss : 4 hits) at 5 requests per second for 60 s over committed cells and asserts the p95 targets; the spike test runs 20 simultaneous misses and asserts that none is dropped and every one is either served or `timeout`. |
| NFR2.1.2 | NFR2.1 | **Cutting slots as the load-shedding mechanism.** The concurrency bound is a slot, not a queue: at most four cuts run at once; a request waiting for a slot spends its own budget (NFR1.1.1) and fails as `timeout` if the slot never comes. No other load shedding exists and no failure reason other than `timeout` is produced by overload, so the client's retry offer (AC7.2.1) applies. | **4 slots** (provisional). | Covered by NFR1.1.6's test. |
| NFR2.1.3 | NFR2.1 | **Requester-window memory bounded.** The number of live `RequesterWindow` entries is bounded by distinct requesters in the last hour; each entry is a 64-bit hash, two start times and two counts. | **≤ 64 bytes per requester; 10,000 distinct requesters in an hour ≤ 640 KB**, inside NFR5.1.2. Expired windows are swept at least every minute so the map does not grow between requests from a departed requester. | A test inserts 10,000 windows, advances the clock past the hour, triggers the sweep and asserts the map is empty. |

## Data growth

| ID | Refines | Requirement | Target | Measured how |
|---|---|---|---|---|
| NFR2.1.4 | NFR2.1 | **Coverage grows by configuration.** Adding a region is a configuration change plus one build (W1); no code changes. Coverage is a set of cell ids (`CoverageIndex`) and the proxy answers `404 area_not_found` ("not an area this deployment covers", BR4.1) outside it. | First deployment: **one Canadian province**, which one being a configuration and fixture choice (`functional-spec.md` assumptions). | A test builds from two small committed regions and asserts a box in each is covered and a box between them is not. |
| NFR2.1.5 | NFR2.1 | **Cell count per region is known and bounded at build time.** At 0.01° a province is on the order of 150,000–200,000 populated cells (`RegionalDataBuild.cellCount`, recorded in the manifest, BR8.4). The manifest entry per cell (id, digest, counts, size) is about 60 bytes, so a province's manifest is on the order of 10 MB in memory — inside NFR5.1.2 — and the coverage index as a bitmap over the region's grid rectangle is under 1 MB. | Manifest ≤ **16 MB in memory** per province; coverage index ≤ **1 MB**. | The build writes `cellCount` and `totalBytes` to the manifest; a test on the small committed regions asserts the in-memory manifest size is proportional to `cellCount` at ≤ 80 bytes per cell. |
| NFR2.1.6 | NFR2.1 | **Deploy artifact size and build time.** The cells for the first region set fit the deploy and the weekly build finishes inside one scheduled run. | Cells for one province **≤ 500 MB** after filtering (BR8.2), so the deploy artifact carrying them is **≤ 1 GB**; the weekly build (download, verify, filter, slice, manifest) completes in **≤ 30 minutes** on the runner Infrastructure Design selects. | `RegionalDataBuild.totalBytes` and the build's own elapsed time, written to the manifest and the build log. |
| NFR2.1.7 | NFR2.1 | **Cell packing is Infrastructure Design's, and the count above is why it matters.** Whether the 150,000–200,000 cells are one file each or one packed, indexed file in the artifact is where cells live in the deploy artifact — `functional-spec.md` names that Infrastructure Design's. This stage records the count that makes one-file-per-cell unattractive (image layer size, W2 opening every file at start) and requires that whichever layout is chosen keeps NFR1.1.4's 30-second Ready target. | — | Decided at infrastructure-design; NFR1.1.4 measures the result. |

## Cost as a scaling constraint

| ID | Refines | Requirement | Target |
|---|---|---|---|
| NFR5.1.4 | NFR5.1 | **Average CPU.** Cutting is the only CPU-heavy work; at the expected Stage 1 load it is idle most of the time. | **≤ 0.05 vCPU averaged over a month** (about $1 at Railway's ~$20 per vCPU-month), provisional. |
| NFR5.1.5 | NFR5.1 | **Service cost allocation.** Memory (NFR5.1.2), CPU (NFR5.1.4) and egress together stay inside this Unit's share of the ~$5 budget, leaving room for the database service and for serving the client bundle from the same Railway service (`unit-of-work.md`, "Four Units, one deployable"). | **≤ $2.50 a month for this Unit all-in**, provisional. The egress term is measured at B-3 from `ServiceCounters.bytesServed` and `lastExtractBytes` (W5) and recorded; any measured figure that breaks this allocation is a constraint change under `project.md`, not something to absorb. |

## Scaling out, if it is ever needed

Not planned for product Stage 1; recorded so the consequence is known.

- **What works unchanged.** Every instance loads the same
  `RegionalDataBuild`, cuts deterministically (BR5.4), and holds its own
  cache and windows. No shared state, no coordination, no sticky sessions.
- **What changes meaning.** The per-requester limit is per instance, so two
  replicas allow twice NFR5.1.3 per requester. If replicas are ever added,
  the limit is divided by the replica count or moved to a shared store — the
  latter would be the first per-requester state held outside the process,
  which BR7.3 forbids, so division is the only option consistent with the
  rules as written.
- **What it costs.** A second instance doubles NFR5.1.2 and NFR5.1.4 and is
  therefore a constraint change under `project.md`.

## Assumptions & Open Questions

- 20 concurrent users is a stated assumption, not an observed figure: there
  is no user base and no analytics (`requirements.md` A7 makes the same
  point for viewport widths). [assumption]
- 150 KB per clip is the working estimate behind every egress figure; B-3
  records the real one. [assumption]
- A filtered province is ≤ 500 MB of cells. Ontario's full extract is
  943 MB (`functional-design-questions.md`, checked 2026-09-11); the
  highway-only share after BR8.2's filter, plus duplication of ways across
  the cells they touch (BR8.3), is unmeasured. [assumption]
- The build runner (where W1 runs weekly) is unchosen; the 30-minute target
  assumes a hosted CI runner with at least 7 GB of memory and 14 GB of disk,
  which is what a free public-repository runner provides. [assumption]
