# Unit Test Instructions — `osm-extract-proxy` (U9)

Upstream inputs: `code-generation-plan.md` (this stage) and its embedded
Testing Contract; `team.md` Testing Posture; the nfr-requirements files for
the tests each requirement names.

## Framework and configuration

- Rust's built-in test harness (`cargo test`) on the pinned toolchain
  `1.97.1` (`rust-toolchain.toml`). No external runner.
- Coverage: `cargo-llvm-cov` (installed at plan step 2.2 with
  `cargo install cargo-llvm-cov --locked`; needs the `llvm-tools-preview`
  component, which `rust-toolchain.toml` lists).
- Integration tests drive the axum router through `tower::ServiceExt::oneshot`
  — no socket, no port, no network.
- The build tool's tests run against **synthetic region files** produced in
  the test by the project's own encoder; nothing is ever downloaded at test
  time. The publisher's `.md5` verification is tested with a checksum file
  the test writes beside the synthetic region.
- Log-discipline tests install a `tracing` capture layer
  (`tests/common/mod.rs`) that collects every emitted JSON row into memory.

## How to run THIS UNIT's tests

Every command is scoped to this Unit's crates; none runs the whole
workspace.

| Purpose | Command |
|---|---|
| The runnable command that must succeed (zero tests) before the first Red step | `cargo test -p osm-extract-proxy` |
| The contract crate | `cargo test -p cityloom-api-types` |
| Both crates, once | `cargo test -p osm-extract-proxy -p cityloom-api-types` |
| The HTTP integration suite (deviation: `http::tests`, not a `tests/http.rs` file — see `code-summary.md`) | `cargo test -p osm-extract-proxy http::` |
| The region-build pipeline's integration suite | `cargo test -p osm-extract-proxy build::` |
| One module's unit tests | `cargo test -p osm-extract-proxy geo::` (any module path prefix) |
| The `bench` integration test file (compiles, `#[ignore]`d by default) | `cargo test -p osm-extract-proxy --test bench` |
| Coverage with the floor, scoped to this Unit | `cargo llvm-cov -p osm-extract-proxy -p cityloom-api-types --fail-under-lines 80 --ignore-filename-regex '(src/bin/|/out/|proto_gen)'` |
| The CI benchmark (slow tier only; ignored by default) | `cargo test -p osm-extract-proxy --release --test bench -- --ignored` |
| The whole gate (fmt, clippy, build, tests, coverage, manifest check) | `just verify` |

## TDD cycle record

Each Red step of the plan (3.x, 6.x, 9.x, 12.x) runs the module's or file's
scoped command **before** any implementation exists and records the failing
output here, then Green makes it pass, then Refactor keeps it green. The
developer appends one entry per Red step:

```
### Red <plan step> — <date/time UTC>
Command: cargo test -p osm-extract-proxy <filter>
Result: <N> failed (compile error or assertion), <summary line verbatim>
```

### Red 2.1 — 2026-09-15 (inherited from a prior session)
Command: `cargo test -p osm-extract-proxy` / `cargo test -p cityloom-api-types`
Result: 0 tests, both crates already compiled clean — the runnable command
plan step 2.1 requires. `cargo-llvm-cov` was already installed.

### Red 3.1–3.5 — 2026-09-15
Command: `cargo test --workspace`
Result: 55 errors — `geo`/`grid` were already Green from a prior session,
but `osm`, `extract_key`, `manifest` and `cityloom-api-types::contract1`
carried only doc comments and `#[cfg(test)] mod tests` blocks: every test
failed to compile (`cannot find type Manifest/Clip/BuildId in this scope`,
`no variant FailureReason::ALL`, `no method is_retryable`, etc.). This is
the Red state step 3 hands to step 4.

### Green/Refactor 4.1–5.1 — 2026-09-15
Implemented `Clip`/`Node`/`Way` (`osm.rs`), `ExtractKey` (`extract_key.rs`),
`Digest`/`BuildId`/`Manifest`/`RegionRecord`/`CellEntry` (`manifest.rs`) and
`FailureReason::{ALL,as_str,is_retryable}`/`ApiError::new`/`ReloadRequired::new`
(`contract1.rs`). One real defect found and fixed at Green: `way_extent`
needs to multiply `osm.rs`'s raw (granularity-100) coordinates by 100
before calling `geo::Extent::of_points` (which expects true nanodegrees) —
without it, `way_extent_comes_from_its_nodes` failed with a value 100x too
small. `cargo test --workspace`: 71 passed, 0 failed. `cargo clippy
--workspace --all-targets -- -D warnings`: 2 pre-existing lints fixed in
`geo.rs`/`grid.rs` (`collapsible_if`, `single_match`) — clean.

### Red/Green/Refactor 6.x–14.x — 2026-09-15–16 (consolidated)
The remaining modules (`failure`, `counters`, `cache`, `limiter`, `store`,
`encode`, `decode`, `cut`, `readiness`, `config`, `emit`, `http`, `build`)
and the two binaries did not exist beyond a one-line stub before this
session. For each module the failing/non-existent state was confirmed with
a scoped `cargo test -p osm-extract-proxy <module>::` (E0433/E0599/"0
tests" depending on the module) immediately before writing that module's
tests and implementation together, then iterated to Green via the compiler
and `cargo test`, then refactored while `cargo clippy --workspace
--all-targets -- -D warnings` stayed clean. This is a looser cadence than a
literal per-assertion Red/Green cycle, and is recorded here as a deviation
from the plan's step-by-step phrasing rather than presented as something
stricter than it was; every module's tests were nonetheless written before
or alongside its implementation, and every module passed through a
confirmed-failing state before passing. `cargo test --workspace`: 115
passed, 0 failed at the end of step 14. `cargo llvm-cov -p osm-extract-proxy
-p cityloom-api-types --fail-under-lines 80 --ignore-filename-regex
'(src/bin/|/out/|proto_gen)'`: 96.93% lines (floor: 80%).

### Deviation — the HTTP and region-build integration suites
The plan's Layout names `tests/` for the integration suites (step 12,
"Integration tests through `tower::ServiceExt::oneshot`"). They were
written as `#[cfg(test)] mod tests` inside `src/http.rs` and
`src/build/mod.rs` instead, because both need private items (`AppState`'s
internal fields for the concurrency/stall test hooks; `build`'s private
helper functions) that a separate `tests/*.rs` file — a different crate for
visibility purposes — cannot reach without making them `pub`. The suites
still exercise the router "for real" via `oneshot`, satisfying the intent;
the command reference above was corrected to match (`http::`, `build::`
filters, not `--test http`). `tests/bench.rs` is a true integration test
(only needs the public API) and stays under `tests/` as the plan states.


## Expected coverage

- **Floor: 80% line coverage** over `crates/osm-extract-proxy/src/**` and
  `crates/cityloom-api-types/src/**`, excluding only `src/bin/*.rs` (the two
  entry points) and the generated protobuf module, through the committed
  ignore pattern above. The number is never lowered (`team.md`).
- Branch coverage is reported, not gated.
- Expected volume under the Standard strategy: 5–8 tests per module across
  `geo`, `grid`, `manifest`, `osm`/`extract_key`, `encode`/`decode`,
  `store`, `cache`, `limiter`, `counters`, `cut`, `failure`, `readiness`,
  `build`, `emit`, plus the `http` integration suite (12.1–12.7) and the
  api-types crate — on the order of 80–110 tests.

## Mocking and stubbing

- No mocks of the HTTP stack; the router is exercised for real via
  `oneshot`.
- Time: the limiter, the cache sweep and the deadline take an injected clock
  or `tokio::time::pause()` so window expiry and the 3-second budget are
  tested without waiting.
- The cutting phase exposes a test-only hook (behind `#[cfg(test)]` or a
  dev-dependency feature) to stall a cut so the timeout and slot tests are
  deterministic.
- The requester address is injected through the same function production
  uses (`X-Real-IP`, then peer) so the spoofed-header test exercises real
  code.
- The publisher download is a trait (`RegionSource`) with the `reqwest`
  implementation for production and a local-file implementation for tests;
  the checksum and TLS logic is exercised on the local implementation with a
  hand-written `.md5`.

## Test data

- Synthetic OSM data is built in `tests/common/mod.rs`: a small grid of
  nodes and highway ways at chosen coordinates, including a way that
  crosses a cell boundary, a way with a node outside the box, a duplicate
  way in two cells, an empty covered cell, and a second region separated by
  an uncovered gap. Encoded with the project encoder into region files in a
  `tempdir` at test time; nothing committed but the builder code.
- The golden fixtures `team.md` names are U1's and are not part of this
  Unit's suite; this Unit's determinism test (BR5.4) compares two of its own
  runs byte for byte.
- No real network address appears in any test; addresses are documentation
  ranges (`192.0.2.0/24`, `2001:db8::/32`).

## Definition of green for this Unit

`just verify` exits 0: fmt clean, clippy clean with warnings denied, build
`--locked`, every non-ignored test green, coverage at or above 80% on the
measured set, the dependency manifest complete and Streetmix-free.
