# Unit Test Instructions — `street-core` (U2)

Upstream inputs: `code-generation-plan.md` (this stage) and its embedded
Testing Contract; `team.md` Testing Posture.

## Framework and configuration

- Rust's built-in test harness (`cargo test`) on the pinned toolchain
  1.97.1 (`rust-toolchain.toml`, reused from `osm-extract-proxy`). No
  external runner.
- Coverage: `cargo-llvm-cov` (already installed by `osm-extract-proxy`'s
  code generation).
- No mocks, no test containers — every test is in-process against plain
  Rust values, since this crate has no I/O.

## How to run THIS UNIT's tests

| Purpose | Command |
|---|---|
| The runnable command that must succeed (zero tests) before the first Red step | `cargo test -p street-core` |
| One module's tests | `cargo test -p street-core lane::` (any module path prefix) |
| Coverage with the floor, scoped to this Unit | `cargo llvm-cov -p street-core --fail-under-lines 80` |
| The whole gate | `just verify` |

## TDD cycle record

Each Red step of the plan (3.x, 6.x, 9.x, 12.x) runs `cargo test -p
street-core <filter>` before any implementation exists and records the
failing output here, then Green makes it pass, then Refactor keeps it
green. The developer appends one entry per Red step:

```
### Red <plan step> — <date/time UTC>
Command: cargo test -p street-core <filter>
Result: <N> failed (compile error or assertion), <summary line verbatim>
```

### Red 3.1 — 2026-09-17
Command: `cargo test -p street-core provenance::`
Result: compile error, 20 errors — `error[E0433]: cannot find type
`Provenance` in this scope` (and `Dimension`), repeated at every test-body
reference; `Dimension`/`Provenance` did not yet exist. Confirms the Red
state before `Dimension`/`Provenance` were implemented.

### Red 6.1 — 2026-09-17
Command: `cargo test -p street-core lane::`
Result: compile error, 26 errors — `error[E0433]: cannot find type
`LaneList`/`Lane`/`Direction` in this scope`. Confirms the Red state
before `Lane`, `LaneKey`, `LaneList`, `Direction` were implemented.

### Red 9.1 — 2026-09-17
Command: `cargo test -p street-core street::`
Result: compile error, 18 errors — `error[E0433]: cannot find type
`Street` in this scope` (and related `BoundingNodeIds`/`StreetIdentity`
uses). Confirms the Red state before `Street`, `BoundingNodeIds`,
`StreetIdentity` and `compute_carriageway_width` were implemented.

### Red 12.1 (graph) — 2026-09-17
Command: `cargo test -p street-core graph::`
Result: compile error, 3 errors — `error[E0433]: cannot find type
`StreetNetworkGraph` in this scope`. Confirms the Red state before
`StreetNetworkGraph`/`IntersectionId` were implemented.

### Red 12.1 (source) — 2026-09-17
Command: `cargo test -p street-core source::`
Result: compile error, 2 errors — `error[E0405]: cannot find trait
`StreetSource` in this scope`. Confirms the Red state before the
`StreetSource` trait was declared.

## Final result

`cargo test -p street-core`: 28 passed, 0 failed. `cargo clippy -p
street-core --all-targets -- -D warnings`: clean. `cargo fmt --all --
--check`: clean (repo-wide). `cargo llvm-cov -p street-core
--fail-under-lines 80`: 94.84% line coverage (per-module: provenance.rs
100%, source.rs 100%, street.rs 96.61%, lane.rs 93.55%, graph.rs
85.00%), exit code 0 — floor met with no ignore pattern.

## Expected coverage

- **Floor: 80% line coverage**, no ignore pattern — `team.md` names this
  crate directly as part of the coverage-floor set ("the street/lane
  domain model crate").
- Expected volume under the Standard strategy: ~20-25 tests across
  `provenance`, `lane`, `street`, and `graph`/`source`.

## Mocking and stubbing

None needed. The one place a fake matters is `source.rs`'s test: a
minimal in-test `StreetSource` implementation proves the trait is usable
without any osm2streets-related dependency in this crate's tree (BR7.1).

## Test data

Plain Rust struct literals constructed directly in each test — no
fixtures, no external data, since this crate holds no real-world OSM
data (that arrives via `u4-street-import`, out of scope here).

## Definition of green for this Unit

`cargo test -p street-core` green, `cargo clippy -p street-core
--all-targets -- -D warnings` clean, `cargo fmt --all -- --check` clean
for this crate's files, `cargo llvm-cov -p street-core --fail-under-lines
80` passing with no ignore pattern.
