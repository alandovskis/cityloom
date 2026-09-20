# Technology Stack Decisions — `osm-extract-proxy` (U9)

Upstream inputs: `functional-spec.md`, `rules.md` and `entities.md`
(functional-design, this Unit), `requirements.md` (requirements-analysis),
`contract-summary.md` (contract-design), `nfr-requirements-questions.md`
(this stage), `team.md` and `project.md` (practices).

Every version below was read from crates.io on 2026-09-12 and is the newest
stable release at that date, per the "always use the latest version"
practice; the exact pins land in `Cargo.toml` and the committed `Cargo.lock`
at code generation. Each decision has a context, a decision, consequences
and the alternatives rejected, in the ADR shape the project's phase
guardrails require; they are numbered `TS-n` to keep them distinct from the
domain-design ADRs in `decisions.md`.

## What was already decided elsewhere

| Decision | Where | Effect here |
|---|---|---|
| Both ends of every HTTP boundary are Rust; request and response types live in one shared crate (`cityloom-api-types`) serialised with `serde` | `contract-summary.md`, Q2 | The server is a Rust binary in the Cargo workspace; Contract 1's types are consumed, not defined, here |
| One Railway service serves the WASM bundle and the API; U9–U12 share it | `unit-of-work.md`, Q7 | TS-1 is inherited by three later Units |
| `rustfmt`, `clippy -D warnings`, `cargo audit`, `cargo-llvm-cov`, TDD, committed `Cargo.lock`, `--locked` builds | `team.md` | Every crate below enters the same gate; nothing here relaxes it |
| Every osm2streets-related dependency is pinned by commit | `project.md` | Not this Unit's tree — the proxy does not depend on osm2streets (`functional-spec.md`: U1's fixtures are cut *by* this Unit, not with it) |
| Which builder produces the Railway image (Nixpacks vs `Dockerfile`) | `team.md`; confirmed at environment-provisioning | Not decided here |
| The client UI framework | `decisions.md` ADR-003 leaves the framework to code generation | Not this Unit's |

## TS-1: axum on tokio for the HTTP server

**Status**: Accepted (Q4). **Applies to**: the whole Railway service, so U10,
U11 and U12 inherit it.

**Context.** The contract fixes a Rust server and no stage had chosen a
framework. The service needs routing for a handful of endpoints, typed
extractors for a query parameter and a header, per-route middleware for
response headers, a way to apply a request budget that maps to this
project's `timeout` reason, and a test harness that can drive a request
without a socket.

**Decision.** `axum` 0.8.9 on `tokio` 1.53 (multi-threaded runtime), with
`tower` 0.5 and `tower-http` 0.7 for middleware. Handlers are plain async
functions returning the typed response; the request budget is a
`tokio::time::timeout` inside the handler so the elapsed case is the
contract's `503 timeout`, not a framework-generic 408; response headers are a
`tower-http` `SetResponseHeader` layer per NFR6.4.3; tests use `tower`'s
`oneshot` against the router with no listener.

**Consequences.** Positive: the tokio project maintains axum, hyper and
tower together, so the runtime, the HTTP implementation and the middleware
move in step; the middleware is the ecosystem's, not the framework's. Neutral:
`tower-http`'s request tracing layer is **not** used on the extract route,
because its default span records the full URI including `bbox` (NFR6.3.1,
T1); any request-scoped tracing is project-owned and never emitted. Negative:
axum's extractor errors are framework-shaped; every one on this route is
mapped to the contract's `ApiError` so BR10.1 holds.

**Alternatives rejected.** *actix-web* — mature and fast, but its middleware
and extractor model is its own rather than tower's, and the throughput
advantage buys nothing at this service's load (NFR2.1.1). *hyper + tower with
no framework* — the fewest dependencies for one endpoint, but the three
later server Units would each re-write routing and error mapping that axum
supplies once.

## TS-2: `osmpbf` for reading PBF

**Status**: Accepted.

**Context.** Two reads exist: the publisher's regional extract at build time
(W1, hundreds of megabytes to a gigabyte) and the touched cells at request
time (W3, a few files of tens to hundreds of kilobytes).

**Decision.** `osmpbf` 0.3.8 for both. At build time its parallel block
decoding (`rayon`) makes a single-pass filter over a province tractable on a
CI runner; at request time a cell is decoded from a `std::io::Read` in
memory.

**Consequences.** Positive: the most used Rust PBF reader, with a memory-mapped
and an indexed mode if the build ever needs random access; it pulls in
`protobuf` 3 and `flate2`, which TS-3 reuses rather than duplicating.
Negative: read-only — see TS-3.

**Alternatives rejected.** *osm-io 0.3* — reads and writes PBF, but its
reader is designed around whole-file pipelines to a database and its writer
is path-bound (TS-3). *osmio 0.16* — reads PBF and writes XML/OPL only; no
PBF writer.

## TS-3: a project-owned PBF encoder

**Status**: Accepted.

**Context.** The proxy writes PBF twice: cells at build time (BR8.3) and
clips at request time (BR5.5), the latter into memory. BR5.4 requires the
same canonical box on the same build to yield byte-identical output on every
instance, and `team.md`'s golden fixtures are cut by this Unit from committed
cells, so fixture bytes and served bytes must come from one code path.
Checked on crates.io today: `osmpbf` is read-only; `osm-io`'s writer is
constructed only from a file path (`Writer::new(path: PathBuf, …)`,
`Writer::from_file_info(path: PathBuf, …)`), so a request-time clip would go
through a temporary file; `osmio` has no PBF writer at all.

**Decision.** One encoder owned by this project, generated with
`protobuf` 3.7 and `protobuf-codegen` from the OSM PBF `.proto` definitions
(`fileformat.proto`, `osmformat.proto`, MIT-licensed, vendored with their
upstream commit recorded in the asset manifest), compressing blobs with
`flate2` 1.1. It writes a header block declaring `OsmSchema-V0.6` and
`DenseNodes`, one primitive block per clip with a string table, dense nodes
and ways in the fixed order BR5.3 requires, and no timestamp, program name
or other varying field. The same encoder writes cells to disk at build time
and clips to a byte buffer at request time.

**Consequences.** Positive: determinism is under this project's control
(BR5.4 is a property of this code, not of a dependency's version); the
request path never touches disk to write; one dependency set shared with
TS-2. Negative: a few hundred lines of encoding to own and test; the OSM
schema's optional fields (relations, metadata, changesets) are simply not
written, which is correct for what osm2streets reads (BR8.2) but means the
encoder is not a general one. The round trip is asserted in tests by decoding
every encoder output with `osmpbf` (TS-2) and comparing elements.

**Alternatives rejected.** *osm-io writer via a temporary file* — a disk
write per cache miss on the request path, and determinism dependent on the
crate's header fields. *prost* instead of `protobuf` — a second protobuf
implementation in a tree that already carries one through `osmpbf`.

## TS-4: `moka` for the extract cache

**Status**: Accepted.

**Context.** BR6.2 fixes a memory-only cache of successful clips with a total
byte ceiling and least-recently-used eviction; BR6.3 fixes single-flight —
concurrent requests for one key share one cut. NFR5.1.1 sets the ceiling at
32 MB.

**Decision.** `moka` 0.12.16, `moka::future::Cache`, built with
`eviction_policy(EvictionPolicy::lru())`, a `weigher` returning
`CachedExtract.sizeBytes`, `max_capacity(32 MiB)`, and `try_get_with` for
the single-flight cut: the crate guarantees that concurrent calls on the
same absent key are coalesced into one evaluation of the loading future. A
loading failure (an empty clip, BR4.2, or a fault) is returned as an error
from the loader so nothing is stored (BR6.2, "failures are never stored").

**Consequences.** Positive: BR6.2 and BR6.3 are two builder options and one
method rather than a hand-written lock-and-notify structure; the byte
weigher makes the ceiling a property of the cache, not of the caller.
Neutral: moka's default policy is TinyLFU, which is *not* the LRU BR6.2
names — the LRU policy is selected explicitly and a test asserts recency
ordering. Negative: a dependency with its own background housekeeping; its
memory overhead per entry is bounded and counts toward NFR5.1.2.

**Alternatives rejected.** *`lru` crate plus a hand-written single-flight
map* — strict LRU but unweighted, so the byte ceiling and the coalescing
would both be project code to test. *No cache* — BR6.x exist because
identical requests are the common case within a session.

## TS-5: BLAKE3 for project digests, MD5 only for the publisher's checksum

**Status**: Accepted.

**Context.** Digests appear in four places: each cell's digest (BR8.4,
verified at every start, BR9.1), the content-derived `buildId` (BR8.4), the
extract key (BR6.1), and the publisher's checksum for a downloaded region
(BR8.1). Verified 2026-09-12 that the publisher ships a `.osm.pbf.md5`
beside each extract.

**Decision.** `blake3` 1.8.7 for every project-owned digest; `md-5` 0.11 to
verify the publisher's file, and for nothing else. The extract key is the
lowercase hex of BLAKE3 over the canonical box's five-decimal string form
and the `buildId`; the `buildId` is BLAKE3 over the regions' source digests
in configured order and the `GridSpec`.

**Consequences.** Positive: verifying a province's cells at start (NFR1.1.4)
is bounded by disk, not hashing; one hash family for everything this project
mints. Neutral: MD5 is used as the publisher provides it — a transfer
integrity check under TLS, not an authenticity claim (T10 in
`security-requirements.md`). Negative: none material.

**Alternatives rejected.** *SHA-256 throughout* — fine, and it is what the
platform's tooling often expects; rejected only because start-up hashing of
hundreds of megabytes is on the readiness path and BLAKE3 is several times
faster on the same hardware. *Skipping the publisher's MD5* — BR8.1
forbids it.

## TS-6: `tracing` with a JSON subscriber to stdout

**Status**: Accepted.

**Context.** Observability is rows on stdout and nothing else
(`observability-requirements.md`); BR7.1–BR7.4 forbid per-request rows.

**Decision.** `tracing` 0.1.44 and `tracing-subscriber` 0.3.23 with its JSON
formatter writing to stdout, **span events disabled** (no enter/exit/close
rows), and every emitted row authored as an explicit event carrying only the
fields the observability file names. Request-scoped spans, if any, exist for
the deadline and the failure mapping and are never emitted.

**Consequences.** Positive: structured rows the platform collects as-is; the
ecosystem's standard instrumentation, so later server Units use the same
subscriber. Negative: the discipline is a configuration, so NFR6.3.1's
log-capture test is what keeps a future `TraceLayer` or `FmtSpan::CLOSE` out.

**Alternatives rejected.** *`log` + `env_logger`* — unstructured lines.
*A metrics endpoint (`/metrics`)* — a public, unauthenticated aggregate
endpoint on a single-instance service is an attack surface for no reader;
the once-a-minute counters row serves the same numbers to the only person
who reads them.

## TS-7: `reqwest` for the build-time download

**Status**: Accepted.

**Context.** W1 downloads each region and its checksum over HTTPS with
certificate verification (NFR7.2.2) and an identifying `User-Agent`. This
runs on the build runner, never in the service.

**Decision.** `reqwest` 0.13.5 with `rustls`, streaming the response body to
disk while hashing (the file is gigabytes; it is never held in memory), with
the project `User-Agent` and a per-region timeout.

**Consequences.** Positive: TLS without a system OpenSSL dependency on the
runner; streaming keeps the build inside a free runner's memory. Neutral: the
crate is a build-tool dependency; it does not enter the service binary.

**Alternatives rejected.** *`curl` from the shell in the build job* — one
more tool to pin, and the hashing-while-streaming would be a second pass.

## TS-8: a project-owned requester limiter

**Status**: Accepted.

**Context.** BR2.1–BR2.3 and BR7.1–BR7.3 fix the limiter's shape: two fixed
windows per requester keyed on a keyed hash of the address with a per-process
key, holding no location, never written anywhere. NFR5.1.3 fixes the
numbers.

**Decision.** A small module of this Unit: `std::collections::HashMap` from
the 64-bit requester hash to two `(windowStartedAt, requestCount)` pairs,
behind a `std::sync::Mutex` (the critical section is a lookup and an
increment), swept of expired windows at least once a minute; the hash is
`std::hash::RandomState::new()` (SipHash-1-3 with a 128-bit key from the
operating system's random source) created once at process start (NFR6.3.2).
No new dependency.

**Consequences.** Positive: the entire per-requester surface is one file
this project owns and NFR6.3.1's test covers; nothing keys on the raw
address anywhere in the tree. Negative: a hand-written window is code to
test — the tests are named under NFR5.1.3 and NFR2.1.3.

**Alternatives rejected.** *`tower_governor` / `governor`* — key on the peer
or forwarded address as given and carry it in their state, which is exactly
the value BR2.3 says exists only for the hashing call; adapting them to a
pre-hashed key is more code than the window itself.

## TS-9: the build pipeline is a Rust binary in the workspace

**Status**: Accepted.

**Context.** W1 is a build step: download, verify, filter, slice, manifest.
It must reproduce the same `buildId` from the same inputs (NFR7.2.1), run
where a schedule can run it, and share the encoder (TS-3) and the model with
the service.

**Decision.** A second binary target in the proxy's crate (or a sibling crate
in the workspace), run as `cargo run --release --bin <build-tool>` by the
scheduled job Infrastructure Design defines. It shares the entity types,
the `GridSpec`, the encoder and the digests with the service, so a cell the
build wrote is a cell the service reads by construction. The two-pass filter
(collect the node ids referenced by kept ways, then emit nodes and ways into
cells) keeps a province inside a free runner's memory by holding node ids in
a compact set rather than the nodes themselves.

**Consequences.** Positive: one toolchain, one test suite, one lockfile; the
manifest is written by the same types that read it. Neutral: where the job
runs and how the cells reach the deploy artifact (in the image, or fetched
to ephemeral disk at start) is Infrastructure Design's; this decision fixes
only that the tool exists and what it shares. Negative: the build binary
adds to the workspace's compile time in CI, mitigated by the two-tier CI
`team.md` already defines.

**Alternatives rejected.** *osmium-tool from the shell* — fast and proven,
but an external binary to pin on the runner, a second implementation of the
cell layout to keep in step with the service, and its output would not be
this project's encoder's bytes.

## Maintainability requirements this stack must meet

| ID | Refines | Requirement |
|---|---|---|
| NFR7.1.1 | NFR7.1 | **Coverage floor extended to this Unit.** `requirements.md` NFR7.1 names client modules. This Unit adds its own: the bbox validation and canonicalisation, the span and coverage computation, the cut and de-duplication, the encoder (TS-3), the limiter (TS-8), the cache-key derivation and the manifest verification join the `cargo-llvm-cov` measured set at the same **80% line-coverage floor**. The axum wiring, the binary entry points and the build tool's download step leave the denominator through the committed ignore pattern, never by lowering the number. Recorded as a practice candidate for `team.md`'s Testing Posture list at this stage's learnings step. |
| NFR7.2.1 | NFR7.2 | **Reproducible builds and data.** `Cargo.lock` committed, `--locked` builds, the toolchain pinned in `rust-toolchain.toml` (version fixed at code generation, recorded in the same committed file U1 keeps for osm2streets); the same region files and `GridSpec` produce the same `buildId` and byte-identical cells (BR8.4, BR5.4). |
| NFR7.3.1 | NFR7.3 | **Every defect gets a failing regression test first**, per `team.md`; for this Unit that includes an encoder output that `osmpbf` cannot read back, a cell layout change, and any log row that fails NFR6.3.1. |

## Dependency summary

| Crate | Version (2026-09-12) | Used for | In the service binary |
|---|---|---|---|
| `axum` | 0.8.9 | HTTP router, extractors, responses (TS-1) | yes |
| `tokio` | 1.53.1 | async runtime, timeouts, semaphore for cutting slots | yes |
| `tower` | 0.5.3 | service trait; `oneshot` in tests | yes |
| `tower-http` | 0.7.1 | response headers (TS-1); **not** its trace layer | yes |
| `serde`, `serde_json` | 1.0.229 / current | Contract 1 types via the shared crate; JSON rows | yes |
| `osmpbf` | 0.3.8 | reading regions and cells (TS-2) | yes |
| `protobuf`, `protobuf-codegen` | 3.7.2 | the project encoder (TS-3) | yes |
| `flate2` | 1.1.10 | PBF blob compression (TS-3) | yes |
| `moka` | 0.12.16 | extract cache with LRU, byte weigher, single-flight (TS-4) | yes |
| `blake3` | 1.8.7 | cell digests, `buildId`, extract key (TS-5) | yes |
| `md-5` | 0.11.0 | publisher checksum (TS-5) | build tool only |
| `tracing`, `tracing-subscriber` | 0.1.44 / 0.3.23 | JSON rows to stdout (TS-6) | yes |
| `reqwest` | 0.13.5 | region download (TS-7) | build tool only |

Every entry above, and the vendored `.proto` files with their MIT licence,
is recorded in the committed asset and dependency manifest `scripts/verify.sh`
checks (`team.md`). None is Streetmix-related; the manifest check stays
green by construction.

## Assumptions & Open Questions

- `cityloom-api-types` (the shared contract crate) exists as a workspace
  member by the time this Unit is generated; it is U9-owned for Contract 1's
  types (`contract-summary.md`, ownership table). [assumption]
- The `protobuf` 3 code generator produces types from the vendored `.proto`
  files at build time without a system `protoc` — the crate's pure-Rust
  parser is used, confirmed at code generation. [assumption]
- A free public-repository CI runner can run the weekly build for one
  province inside its memory and time limits (`scalability-requirements.md`
  NFR2.1.6). [assumption]
