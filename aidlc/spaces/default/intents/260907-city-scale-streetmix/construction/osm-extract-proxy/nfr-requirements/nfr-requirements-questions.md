# NFR Requirements — Questions — `osm-extract-proxy` (U9)

Upstream inputs: `functional-spec.md` and `rules.md` (functional-design, this
Unit), `requirements.md` (requirements-analysis), `contract-summary.md`
(contract-design), `team.md` (practices).

**What this stage decides for this Unit:** the measurable targets behind the
proxy's rules — the numbers functional design deliberately left *owed* to
this stage (contract-design Q6's precedent) — plus its security, reliability
and observability requirements and the technology choices that follow.

Most of it is already fixed and is not re-asked:

- **Availability**: 99.5% monthly for the service (NFR3.1), health-checked
  deploys (NFR3.2), no attached volume (NFR3.3).
- **Import budget**: 10 seconds end to end for a full import (NFR1.1) — the
  proxy gets a slice of that, not all of it.
- **Cost**: ~$5/month all in (NFR5.1). Checked against Railway's published
  pricing on 2026-09-11: memory is about $10 per GB-month, CPU about $20 per
  vCPU-month, egress $0.05 per GB, and the Hobby plan is $5/month including $5
  of usage credit. Every number below is a claim on that same $5.
- **Privacy**: no address, no bounding box, no extract key ever logged
  (functional design BR7.x, `decisions.md` ADR-004).
- **Toolchain and gates**: Rust, `clippy -D warnings`, `cargo fmt --check`,
  `cargo audit`, 80% line coverage on the measured set, TDD (`team.md`).
- **Security headers and no-DAST assertions**: `team.md` names the response
  headers and the object-level authorisation checks asserted in the normal
  test suite; the proxy inherits the header set.

Three numbers change what the design costs and what a user can do, and one
technology choice is inherited by every later server Unit; those four are
below. The grid cell size, the cell-span bound, the canonical precision and the
supporting crates (PBF reading and writing, hashing, the cache) are technical
parameters set by this stage with rationale and confirmed at the summary
rather than asked.

---

## Q1. How many requests may one requester make? (the numbers behind BR2.1)

Each served extract is paid egress whether it came from the cache or not. A
one-street clip in the compact binary format is expected to be tens of
kilobytes — unmeasured until B-3. A person selecting streets by hand makes a
request every few seconds at most; corridor work (US6.3, US6.4) selects several
streets in a row.

The limit is two-tiered — a short window for bursts and a longer one for
sustained use — because a single-window limit is either too tight for a burst
of clicks or too loose over an hour.

A. **30 per minute and 300 per hour, per requester.** Room for a burst of
   selections and a corridor of a dozen streets; a client running flat out
   is bounded to about 7,000 clips a day — roughly 1 GB and $0.05 a day at
   150 KB per clip, or $1.60 a month for one relentless client.

B. **60 per minute and 1,000 per hour.** Looser; one relentless client could
   cost about $5 a month on its own at the same clip size — the whole budget.

C. **10 per minute and 100 per hour.** Tight; a corridor of a dozen streets
   selected quickly would hit the minute limit and see a retry prompt.

X. Other (please specify)

[Answer]: A

---

## Q2. How much of the 10-second import budget does the proxy get? (the numbers behind BR10.3)

NFR1.1 gives a full import 10 seconds end to end: network to the proxy, the
proxy's own work, network back, the adapter's conversion, rendering. AC3.1.6
needs a server-side budget after which the proxy abandons the request and
answers `timeout`. With cells pre-cut and on local disk, the proxy's own work
is reading a few files, merging, encoding — well under a second is the
expectation, but nothing has been measured yet.

Every option is **provisional**: it is the value the proxy is built against
and B-0/B-3 measure it; a measured figure replaces it through the normal
change path rather than being absorbed.

A. **Hard budget 3 seconds; targets p95 ≤ 300 ms on a cache miss, p95 ≤ 30 ms
   on a hit, measured at the proxy.** Leaves about 7 seconds for the two
   network legs, the adapter and the screen. Generous for local cutting;
   tight enough that a stuck request cannot eat the user's whole budget.

B. **Hard budget 5 seconds; same p95 targets.** More slack for a slow disk or
   a large box; leaves 5 seconds for everything else.

C. **Hard budget 1 second; p95 ≤ 200 ms miss, ≤ 20 ms hit.** Strict; a cold
   start or a big box may fail the budget before it fails the user.

X. Other (please specify)

[Answer]: A

---

## Q3. How much memory, and therefore how much of the $5, may the proxy hold? (the numbers behind BR6.2)

Railway meters memory at about $10 per GB-month, on what the process actually
uses. The cache ceiling (BR6.2) is the one number in this Unit that turns
directly into a monthly charge, and the rest of the process — the loaded
manifest, the coverage index, per-request working memory — sits under the
same ceiling. The database service and the client bundle's serving share the
same $5.

A. **Cache ceiling 32 MB; whole-process target 128 MB typical.** About $1.30
   a month of the $5 for this service. At tens of kilobytes a clip the cache
   holds several hundred streets — more than one person looks at in a
   session.

B. **Cache ceiling 64 MB; whole-process target 256 MB typical.** About $2.60
   a month — half the budget for this one service — for a cache that mostly
   holds streets nobody will ask for again before the next deploy empties it.

C. **Cache ceiling 16 MB; whole-process target 96 MB typical.** About $1 a
   month; the cache holds a hundred-odd streets.

X. Other (please specify)

[Answer]: A

---

## Q4. Which Rust HTTP stack does the server run on?

`contract-summary.md` fixes that the server is Rust and shares its request and
response types with the client through one crate; it names no HTTP framework,
and neither does any earlier stage. U9 is the first of the four server Units
to reach this stage, and U9 through U12 are one Railway service
(`unit-of-work.md`), so the stack chosen here is inherited by `design-storage`,
`accounts-sharing` and `data-rights` rather than re-chosen per Unit. It is a
decision for the deployable, recorded in this Unit because this Unit arrives
first.

What it changes: the crate the request handlers, the middleware (request
budget, requester limiting, security headers), the readiness endpoint and the
test harness are written against, and the async runtime underneath them.

A. **axum on tokio.** Maintained by the tokio project on top of hyper and
   tower, so timeouts, body limits and header middleware are tower's and are
   shared with the rest of that ecosystem; handlers are plain async functions
   with typed extractors; serde JSON is built in. The most common choice for
   new Rust services today.

B. **actix-web.** Longer established, its own runtime layered on tokio, with
   its own middleware and extractor model rather than tower's; consistently
   near the top of throughput benchmarks, which this service does not need.

C. **hyper and tower with no framework.** The fewest dependencies for one
   endpoint and a readiness check; routing, extraction and error mapping are
   hand-written, and the three later server Units would repeat that work.

X. Other (please specify)

[Answer]: A

---

## Consolidated Summary Confirmation

**Your answers**

- **Per-requester limit** (Q1): 30 requests per minute and 300 per hour, per
  requester — one relentless client bounded to about 7,000 clips a day, about
  $1.60 a month at 150 KB per clip.
- **Request budget** (Q2): a hard budget of 3 seconds at the proxy, after which
  the request is abandoned and answered `timeout`; targets p95 ≤ 300 ms on a
  cache miss and p95 ≤ 30 ms on a hit, measured at the proxy. Provisional
  until B-0/B-3 measure it.
- **Memory** (Q3): cache ceiling 32 MB; whole-process target about 128 MB
  typical — about $1.30 a month of the $5.
- **Server HTTP stack** (Q4): axum on tokio, inherited by the three later
  server Units that share the one Railway service.

**Things I will set with rationale rather than ask — confirm them here**

- **Grid cell size 0.01°** (1/100 of a degree, exact edges): about 1.1 km
  north–south and 0.7–0.8 km east–west across Canada's cities. A typical
  one-street box touches 1–4 cells. **Cell-span bound 12 cells**, so a box is
  at most about 10 km² whatever its shape and about 9–13 km on its long
  side; beyond that, `area_too_large`. The bound caps request cost (cells
  read) and clip size (bytes served) at once. Consequence for Infrastructure
  Design, flagged not decided: a province at this size is on the order of
  150,000–200,000 populated cells, which argues for packing cells into one
  indexed file in the deploy artifact rather than one file per cell.
- **Canonical precision 5 decimal places** (about 1.1 m), rounded outward
  (minimums down, maximums up), so canonicalisation never shrinks a box.
- **Two fixed windows per requester** (minute and hour), each a start time
  and a count; the requester hash is SipHash keyed with a random 128-bit key
  minted at process start (the standard library's `RandomState`, no new
  dependency); a window is discarded when it elapses.
- **At most 4 clips being cut at once**; further misses wait inside their own
  3-second budget, so overload surfaces as `timeout`, never as an unbounded
  queue.
- **Ready within 30 seconds of process start** (p95) for the first region
  set, verifying every cell's digest; the readiness endpoint reads the
  Ready/Unready state and nothing else. Its shape and path are Infrastructure
  Design's.
- **Digests**: BLAKE3 for cell digests, the `buildId` and the extract key.
  MD5 only where the publisher's checksum demands it — verified today that
  Geofabrik publishes a `.osm.pbf.md5` beside every extract (BR8.1).
- **Crates** (versions as published on crates.io today): `axum` 0.8, `tokio`
  1, `tower-http` 0.7 (response headers; the request budget is applied in the
  handler so it maps to `timeout`, not a generic 408); `osmpbf` 0.3 to read
  the regional file at build time and the cells at request time — it is
  read-only, and `osm-io`'s writer takes a file path while `osmio` has no PBF
  writer at all, so **the PBF encoder is project-owned**: one encoder,
  generated with `protobuf` 3 (the implementation `osmpbf` already pulls in)
  from the MIT-licensed OSM `.proto` definitions, writes both the cells at
  build time and the clips at request time, so fixture bytes and served
  bytes come from the same code and BR5.4's byte-identity is under this
  project's control; `moka` 0.12 for the cache with its LRU policy, a byte
  weigher and `try_get_with` for single-flight; `blake3` 1.8; `md-5` 0.11;
  `tracing` + `tracing-subscriber` for JSON logs to stdout; `reqwest` 0.13
  for the build-time download over TLS. No third-party rate limiter: the
  ones available key on the raw address, which BR2.3 and BR7.1 forbid
  holding.
- **Security posture**: the endpoint is public and unauthenticated by design;
  input is validated per BR3.1 before anything is read; the requester's
  address is read from the platform-designated header (Infrastructure Design
  names it), hashed immediately and never retained; request spans carry
  method, route, status, reason, phase, duration and hit/miss — never the
  URI or query (the framework's default request span would log the `bbox`),
  never headers, never the peer address; `team.md`'s response headers are
  set; the build job needs no secret; the download is TLS with certificate
  verification plus the publisher's checksum.
- **Observability**: the counters are written as one JSON log line every
  60 seconds and at shutdown; each `FailureRecord` is one JSON log line
  (ERROR for `internal`, WARN for `timeout` and `upstream_unavailable`, INFO
  for the 4xx reasons); no metrics endpoint, no distributed tracing, no
  dashboard — Railway's own deploy and crash notifications and its account
  usage limit are the only alerts in product Stage 1.
- **Scalability and reliability**: one instance; all runtime state is
  per-process, so a second replica would work but would double the
  per-requester allowance; 99.5% monthly (NFR3.1) is 3.6 hours of downtime a
  month; a failed build never deploys; there is nothing to back up and no
  recovery point — recovery is a redeploy.

**Things I will record as open rather than answer**

- Every number above is provisional against B-0/B-3 measurements: clip size,
  egress per street, cut latency, startup time, cell counts and artifact size.
- How Railway presents the client address, where the cells live in the
  deploy artifact, and the health check's shape — Infrastructure Design.
- How the counters and failure log are exported — observability-setup.
- Whether `bbox` remains the request shape — unchanged from `contract-summary.md`.

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
