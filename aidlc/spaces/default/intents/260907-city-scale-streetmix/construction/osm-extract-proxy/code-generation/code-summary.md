# Code Summary — `osm-extract-proxy` (U9)

## What was built

The first application code in the repository: a Cargo workspace with two
crates —

- **`cityloom-api-types`** — Contract 1's wire types (`FailureReason`,
  `ApiError`, `ReloadRequired`), owned by this Unit, `serde`-only.
- **`osm-extract-proxy`** — the proxy service and the `region-build` tool,
  sharing one library (`src/lib.rs`) across 17 modules plus the two
  binaries, following the plan's three-ring layering (pure /
  `geo`,`grid`,`manifest`,`osm`,`encode`,`cut`,`extract_key`,`failure`;
  I/O / `store`,`cache`,`limiter`,`counters`,`emit`,`readiness`,`config`,`build`;
  outer / `http`, the binaries).

Plus the Cargo workspace configuration (`Cargo.toml`, `rust-toolchain.toml`,
`rustfmt.toml`), the gate (`justfile`, `scripts/verify.sh`,
`scripts/state-summary.sh`, `scripts/next-tasks.sh`), the dependency and
asset manifest (`docs/dependencies.md`), the deploy shape (`Dockerfile`,
`railway.json`, `docs/deploy.md`), the weekly data-build workflow
(`.github/workflows/data-build.yml`), the configured region list
(`data/regions.toml`) and the data pointer (`data/current-build.toml`), and
the root `README.md`.

Entry into this session found Step 1 (workspace skeleton) and about a third
of Step 3 (`geo.rs`, `grid.rs` fully implemented; `osm.rs`, `extract_key.rs`,
`manifest.rs`, `cityloom-api-types::contract1` in the Red state — tests
written, no implementation) already done from a prior session. Every
module from Step 4 onward (`osm`, `extract_key`, `manifest`, `contract1`,
`failure`, `counters`, `cache`, `limiter`, `store`, `encode`, `decode`,
`cut`, `readiness`, `config`, `emit`, `http`, `build`, both binaries, and
every infrastructure/doc file) was written in this session.

## Key implementation decisions

- **PBF encode/decode is hand-rolled**, not delegated to a higher-level OSM
  writer crate (per `tech-stack-decisions.md` TS-3): `encode.rs` builds
  `DenseNodes` and `Way` protobuf messages directly (via the vendored,
  code-generated `proto_gen` module) with a first-seen-order string table,
  zlib-compresses each fileblock, and never writes a timestamp or
  `writingprogram` field, which is what makes BR5.4's byte-identity
  guarantee hold. `decode.rs` reads it back with the `osmpbf` crate — a
  genuine cross-implementation round trip, not the same code checking
  itself.
- **Raw coordinate convention.** `osm::Node`/`Way` store coordinates as the
  OSM PBF format's own raw integers at the default granularity of 100 (one
  raw unit = 100 nanodegrees = `osm::RAW_GRANULARITY`), matching what
  `osmpbf`'s `decimicro_lat()`/`decimicro_lon()` return directly. This was
  discovered as a real defect during Green: the first `way_extent`
  implementation multiplied by 1 instead of 100 before calling
  `geo::Extent::of_points` (which expects true nanodegrees), and the
  pre-written `way_extent_comes_from_its_nodes` test caught it immediately
  (a value 100x too small). Documented as the fix, not silently absorbed.
- **The cache-oversize and loader-error rules are enforced explicitly**,
  not left to `moka`'s own eviction heuristics: `Cache::try_get_with`
  inspects the result's size after the loader returns and calls
  `invalidate` immediately if it exceeds the ceiling, because `moka`'s LRU
  policy alone does not guarantee an over-ceiling single entry is evicted
  before the next read.
- **The HTTP and region-build integration suites live inside their modules**
  (`#[cfg(test)] mod tests` in `src/http.rs` and `src/build/mod.rs`), not
  under `tests/` as the plan's Layout literally states. Both need private
  items — `AppState`'s stall/peak-concurrency test hooks, `build`'s
  fixture-source helper — that a true integration test (a separate crate
  for visibility purposes) cannot reach without making them `pub`, which
  would leak test-only surface into the public API. They still exercise the
  router "for real" via `tower::ServiceExt::oneshot`, matching the plan's
  intent; `unit-test-instructions.md`'s command table was corrected to
  match (`http::`, `build::` filters). `tests/bench.rs` is a true
  integration test and stays under `tests/`, needing only the public API.
- **Region download uses `reqwest::blocking`**, not the `stream` feature
  the plan's dependency list mentions, and the `RegionSource` trait
  abstracts it entirely: production (`ReqwestSource`) hits the network,
  tests use an in-memory `FixtureSource`, so the checksum/filter/slice
  logic is exercised without a real download anywhere in the suite.
- **`now_rfc3339`** (in `build/mod.rs`) implements Howard Hinnant's public-
  domain `civil_from_days` algorithm rather than adding a `chrono`
  dependency not in `tech-stack-decisions.md`'s list; tested against four
  known instants including a leap-year boundary check via 2020.

## Test coverage summary

`cargo llvm-cov -p osm-extract-proxy -p cityloom-api-types
--fail-under-lines 80 --ignore-filename-regex '(src/bin/|/out/|proto_gen)'`,
last run before the code-generation guard closed further edits this
session:

```
TOTAL   5899 regions, 181 missed (96.93%) | 412 functions, 20 missed (95.15%) | 3741 lines, 114 missed (96.95%)
```

Every module individually clears the 80% line floor; the lowest is
`decode.rs` at 85.82% (the `Element::Relation(_) => {}` arm and a couple of
defensive `Err` paths that no committed fixture currently exercises).
`cargo test --workspace`: **115 tests**, 0 failed, 0 ignored except
`tests/bench.rs`'s one `#[ignore]`d benchmark, which was run explicitly in
`--release` and passed (miss p95 ≈ 83 µs, hit p95 ≈ 10 µs against the
300 ms / 30 ms budgets — comfortably inside on this synthetic, single-
process fixture; the numbers are a regression guard on the code path, not
evidence about Railway's shared-vCPU hardware, exactly as
`performance-requirements.md` states). `cargo clippy --workspace
--all-targets -- -D warnings`: clean. `cargo fmt --all -- --check`: clean.
The end-to-end pipeline was also smoke-tested manually outside the test
suite: `region-build --synthetic` produced a real `store.bin`/`manifest.json`,
and the running service served a real, byte-identical-on-repeat PBF extract
for it, rejected a wrong `x-client-build` with 426, and cached correctly.

## Deviations and gaps — surfaced, not absorbed

Per the instruction to record a gap rather than silently weaken a target,
in descending order of how much they matter:

1. **The production latency-measurement mechanism doesn't match
   `observability-requirements.md` NFR3.1.14's exact shape.** The
   requirement specifies **two** histograms (hit, miss) with bucket bounds
   `10, 25, 50, 100, 250, 500, 1000, 3000` ms. `counters.rs` implements
   **one** combined histogram with bounds `1, 5, 10, 30, 50, 100, 300,
   1000, ∞` ms that does not distinguish hit from miss. The CI
   benchmark (`tests/bench.rs`) still independently verifies the p95
   targets (NFR1.1.2, NFR1.1.3) end to end, so the *behaviour* the
   histogram exists to make visible in production is proven at test time;
   what's missing is the *production-visible* per-hit/per-miss breakdown.
   Traced as `NFR3.1.14`: PARTIAL, and `NFR1.1.2`/`NFR1.1.3`: PARTIAL
   (the "production measurement" half only).
2. **The per-reason log level doesn't match
   `security-requirements.md`/`observability-requirements.md`'s table.**
   The spec: `internal`→ERROR; `timeout`, `upstream_unavailable`→WARN;
   `invalid_area`, `area_too_large`, `area_not_found`, `rate_limited`→INFO.
   `emit.rs::record_failure` implements a simpler `status ≥ 500 → ERROR,
   else WARN` rule, which gets `internal` right but puts `timeout` and
   `upstream_unavailable` at ERROR instead of WARN, and all four
   client-caused reasons at WARN instead of INFO. **This was found while
   writing this file's traceability pass, with a fix half-drafted**
   (a `LogLevel` field added to `FailureMapping`) **when the
   code-generation guard began refusing further edits to
   `crates/osm-extract-proxy/src/failure.rs` for this unit**, reporting
   that the plan's checkboxes (marked complete earlier in this same
   session) close the window for further Step 4 writes without a fresh
   recorded approval. The half-drafted field was not committed inconsistent;
   `failure.rs` on disk is the last-known-green version with the simpler
   rule, and `just verify` was green with it. Fixing this exactly is a
   same-day follow-up: add `LogLevel` to `FailureMapping`, update
   `emit::record_failure` to branch on it instead of `status`, and update
   the two existing level assertions in `emit.rs`'s tests.
3. **`NFR2.1.1`'s specific mixed-load (1 miss : 4 hits at 5 rps, 60 s) and
   20-simultaneous-miss spike benchmarks were not written.** `tests/bench.rs`
   covers single-type load (200 misses, then 1,000 hits) and
   `http.rs::eight_misses_against_four_slots_never_exceed_four_concurrent`
   covers the slot ceiling under concurrency, but neither is the exact
   scenario the requirement names.
4. **`NFR6.4.4`'s listener hardening (an explicit header-read timeout and
   maximum header size) is not configured.** `POST` rejection is real and
   tested; the explicit numeric limits are not set in
   `src/bin/osm-extract-proxy.rs` — `axum::serve`/`hyper`'s un-overridden
   defaults apply. "Values are fixed at code generation from the HTTP
   stack's documented options," per the requirement, was not done.
5. **Build rows (`NFR3.1.16`, one structured line per region and per build
   run) were not added to `build/mod.rs`.** `region-build`'s binary prints
   plain `println!` lines (`buildId=`, `cellCount=`, `builtAt=`) for the
   GitHub Actions workflow to parse, not a `tracing`-emitted structured
   row. The manifest itself carries every field the row would report
   (`RegionRecord`, `Manifest.totalBytes`, etc.), so the data exists; it
   is not yet observable as a log line the way NFR3.1.15's lifecycle rows
   are.
6. **`NFR1.1.4`'s 30-second Ready target and `NFR2.1.6`'s 30-minute build
   target are not independently timed against real, province-sized data** —
   this Unit's test suite uses small synthetic fixtures throughout
   (`team.md`'s own practice: never a live fetch at test time), so neither
   ceiling has a fixture large enough to threaten it. Both are B-3
   measurements against real data, as `performance-requirements.md`
   and `scalability-requirements.md` already say.
7. **`NFR3.2.5`'s "a limited requester still gets the readiness answer" is
   true by construction** (`readyz_handler` never touches the limiter) but
   has no dedicated regression test driving a requester past its limit and
   then asserting `/readyz` still answers — a one-line gap, not a design gap.

None of the seven above required weakening a coverage floor, a clippy
setting, or a written test's assertion to reach green; `just verify`'s
80% line-coverage gate, `-D warnings`, and every already-written test
assertion are exactly as strict as the plan states. The gaps are entirely
in unimplemented refinements (levels, dual histograms, two specific load
scenarios, two explicit numeric listener settings, one log line, and one
missing regression test) layered on top of a system whose primary
contract (Contract 1, all 32 business rules, and every measurable acceptance
criterion this Unit owns) is implemented and green.

## Files this stage created or modified

See `source-manifest.json` (application code and infrastructure) and
`traceability.json` (requirement-to-file mapping) in this same directory.

## What the next stage needs

- `ci-pipeline`: the fast/slow CI tiers this Unit's `justfile` and
  `scripts/verify.sh` already support locally (`just verify` is the fast
  tier verbatim); `cargo audit` and the release-mode `tests/bench.rs -- --ignored`
  run belong in the slow tier per `team.md`.
- `environment-provisioning`: the handoff list in `docs/deploy.md`
  (Railway service creation, deployment/service variables, spending limit,
  uptime monitor) — nothing in this stage created any cloud resource.
- A follow-up Bolt (or the next touch of this Unit) to close gaps 1–2 and
  4 above, all small, contained changes to `failure.rs`/`emit.rs`/
  `bin/osm-extract-proxy.rs` respectively.
