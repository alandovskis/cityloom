# NFR Design — Questions — `osm-extract-proxy` (U9)

Upstream inputs: `performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`,
`observability-requirements.md` and `tech-stack-decisions.md`
(nfr-requirements, this Unit), `functional-spec.md` (functional-design, this
Unit), `contract-summary.md` (contract-design), `components.md`
(domain-design), `team.md` (practices).

**What this stage decides for this Unit:** how the numbers and obligations
fixed at `nfr-requirements` are met — the read path for a cut, the cache and
single-flight design, the limiter, the request budget and slot mechanism,
the readiness and start-up design, the log and counters design, the security
controls as a layered design, and the logical components Infrastructure
Design will place.

Almost all of it follows from decisions already taken and is not re-asked:

- The numbers: 3-second budget, p95 targets, 0.01° cells with a 12-cell span
  bound, 5-decimal outward canonicalisation, 30/minute and 300/hour per
  requester in two fixed windows, four cutting slots, 32 MB cache with LRU,
  128 MB process target, Ready within 30 seconds (`performance-requirements.md`,
  `scalability-requirements.md`).
- The stack: axum on tokio, `osmpbf` to read, a project-owned PBF encoder,
  `moka` with LRU and single-flight, BLAKE3 digests, `tracing` JSON rows
  (`tech-stack-decisions.md` TS-1 to TS-9).
- The privacy and logging design: no per-request row, five-field failure
  rows, a per-process keyed hash, nothing identifying ever written
  (`security-requirements.md` NFR6.3.x, `observability-requirements.md`).
- There is no downstream call at request time, so there is no circuit
  breaker, no retry policy and no bulkhead to design toward a dependency;
  retries are the client's (AC7.2.1) and the only isolation is the cutting
  slot.

One thing is genuinely open and shapes the read path, the start-up check and
what Infrastructure Design has to place. It is below. Everything else this
stage sets with rationale is confirmed at the summary rather than asked.

---

## Q1. How are the cells laid out on disk?

`entities.md` and `functional-spec.md` speak of cell *files* and leave the
layout open; `scalability-requirements.md` NFR2.1.7 records the count that
makes the choice matter — on the order of 150,000–200,000 populated cells
for one province at 0.01° — and hands *where they ship* (inside the image,
or fetched to ephemeral disk at start) to Infrastructure Design. This
question is the layout itself, which is this stage's because it fixes how a
cut reads (NFR1.1.2), how start-up verifies (NFR1.1.4, BR9.1) and how a
region is added (NFR2.1.4). Whatever is chosen, the manifest (BR8.4) stays
the source of truth for what exists and its digest.

A. **One packed store per build.** A single file holding every populated
   cell's bytes back to back; the manifest carries each cell's offset,
   length and digest. The service opens one file and reads a cell by
   offset (`pread`, or a memory map); start-up verifies by hashing each
   cell's byte range in one sequential pass over the file; the deploy
   artifact is two files, whatever Infrastructure Design does with them. A
   new region means a new store — which is already how builds work (a build
   is immutable, BR8.5 deploys whole builds). Simplest read path, fastest
   start-up, fewest files; the build must write the store in one go and
   cannot be inspected with `ls`.

B. **One file per cell** under a directory tree (`cells/<column>/<row>`).
   The simplest build and the easiest to inspect by hand; each cut opens up
   to twelve files; start-up opens and hashes every one of the
   150,000–200,000 files, which is seconds of directory work before any
   hashing; a container image layer with that many small files is slow to
   build, push and pull, and an ephemeral-disk fetch at start would need to
   be a tarball of them anyway.

C. **One pack per 1° by 1° tile** (about 10,000 cells each; a province is a
   few hundred packs), each with its own index inside the manifest. Bounded
   file count, a cut opens at most four packs, and a single tile could be
   rebuilt alone — a partial update BR8.5's whole-build deploys never need.
   More moving parts than A for a benefit nothing currently asks for.

X. Other (please specify)

[Answer]: A

---

## Consolidated Summary Confirmation

**Your answer**

- **Cell layout** (Q1): one packed store per build — a single file holding
  every populated cell's bytes back to back, the manifest carrying each
  cell's offset, length and digest. One open at start, reads by offset,
  start-up verification as one sequential pass, a deploy artifact of two
  files.

**Things I will design with rationale rather than ask — confirm them here**

- **Read path for a cut.** Touched cell ids are computed in integer
  arithmetic from the canonical box (coordinates scaled to 1e-5 degree
  units, so 0.01° is exactly 1,000 units and cell edges are exact); each
  touched cell present in the index is read from the store by offset with a
  positional read (no memory map, no `unsafe`), decoded with `osmpbf` from
  the bytes, and a way is kept when the extent of its own nodes (all present
  in the cell, BR8.3) intersects the box; elements are de-duplicated by id
  across cells, sorted by type then id (BR5.3) and encoded once. Cutting
  runs on the blocking thread pool behind a four-permit semaphore (the
  slots) and checks its deadline between cells, so an abandoned cut stops
  within one cell's decode time and frees its slot (NFR3.1.7). The index is
  a sorted table of (cell id, offset, length) — about 24 bytes per cell,
  under 5 MB for a province — and the coverage index is a bitmap over the
  grid rectangle of the configured regions.
- **Hit path.** A `moka` lookup on the extract key returns the shared bytes
  with no slot and no decode; misses go through `try_get_with`, which is the
  single-flight (BR6.3); a cut that yields no ways or fails returns an error
  from the loader so nothing is stored (BR6.2).
- **Request budget.** One `tokio::time::timeout` of 3 seconds around the
  whole handler body after the build-stamp check; the deadline is passed
  into the cut; elapsing maps to `503 timeout`.
- **Limiter.** A mutex-guarded map from the 64-bit requester hash to four
  integers (minute start and count, hour start and count); both windows are
  checked and incremented in one critical section; a background task sweeps
  expired entries every 60 seconds; `Retry-After` is the seconds until the
  earlier window clears.
- **Start-up and readiness.** Load configuration, load the manifest (JSON,
  parsed straight into compact structs), open the store, verify every cell's
  byte range against its BLAKE3 digest in one sequential pass, build the
  index and the coverage bitmap, mint the limiter key, then flip an atomic
  Ready flag the readiness endpoint reads (200 when Ready, 503 otherwise).
  Any failure leaves the flag down and logs the Unready lifecycle row. On
  SIGTERM: stop accepting, wait up to the 3-second budget for in-flight
  requests, emit the final counters row, exit.
- **No breaker, no retry policy, no bulkhead toward a dependency** — there
  is no dependency at request time. The one isolation mechanism is the
  cutting slot; the one degradation is Unready. A panic in a request is
  caught at the handler boundary and answered `internal` (BR10.4).
- **Security as layers.** Edge (TLS termination, platform-asserted address)
  → router (GET only, header size and read-timeout limits, no CORS, the
  `x-client-build` gate first) → limiter on the hashed address → typed
  parsing of `bbox` into an integer box → integer-only cell addressing into
  a read-only store → typed responses with the `team.md` header set → a log
  subscriber that emits only the catalogued events. Configuration from
  environment variables, none of them secret. The build tool: TLS with
  certificate verification, the publisher's `.md5`, provenance in the
  manifest.
- **Observability as a small catalogue.** One counters struct of atomics
  (the `ServiceCounters` fields plus two nine-bucket latency histograms and
  uptime), emitted as one JSON row by a 60-second interval task and at
  shutdown; failure rows emitted from the single failure-mapping function
  with exactly five fields; lifecycle rows at Ready, Unready and shutdown;
  the `tracing` JSON subscriber configured with span events off. No
  request-scoped fields are ever attached to an emitted event.
- **Logical components** for Infrastructure Design to place: the proxy
  process (handler, limiter, canonicaliser, store reader, cutter, encoder,
  cache, counters, readiness, log emitter); the cell store artifact (store
  file plus manifest, read-only); the region-build tool (downloader,
  verifier, filter, slicer, store and manifest writer); and three external
  parties — the platform edge, the platform log sink, and the publisher.
  Failure domains: the process (restart), the artifact (rebuild and
  redeploy), the build job (publishes nothing), the platform. Blast radius
  of the process: every street import, degraded to FR6.2's blank
  cross-section by the client. The one shared resource is the Railway
  service itself, which the later server Units share; this Unit's memory
  and CPU figures are its contribution to that process, not the whole.

**Things I will record as open rather than answer**

- Where the two-file artifact ships and how the weekly build's output
  reaches it — Infrastructure Design, unchanged.
- The manifest's exact field names and the store's on-disk framing are
  code-generation's, within the shape above.
- Whether a way's extent should be precomputed at build time and stored
  beside the way to skip the node pass at request time — an optimisation to
  take only if B-3's measured miss latency needs it.

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
