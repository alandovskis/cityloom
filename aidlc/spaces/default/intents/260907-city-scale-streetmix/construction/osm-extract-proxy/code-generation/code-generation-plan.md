# Code Generation Plan — `osm-extract-proxy` (U9)

Upstream inputs: `functional-spec.md`, `rules.md` and `entities.md`
(functional-design), `performance-requirements.md`,
`security-requirements.md`, `scalability-requirements.md`,
`reliability-requirements.md`, `observability-requirements.md` and
`tech-stack-decisions.md` (nfr-requirements), `performance-design.md`,
`security-design.md`, `scalability-design.md`, `reliability-design.md`,
`observability-design.md` and `logical-components.md` (nfr-design),
`infrastructure-specification.md`, `monitoring-design.md` and
`cicd-pipeline.md` (infrastructure-design), `contract-summary.md`
(contract-design), `components.md` (domain-design), `unit-of-work.md`
(units-generation), `bolt-plan.md` B-3 (delivery-planning),
`requirements.md` (requirements-analysis), `stories.md` US3.1
(user-stories), `team.md` and `project.md` (practices).

## What is built

The first application code in the repository: the Cargo workspace, the
shared contract crate for Contract 1 (`cityloom-api-types`, U9-owned per
`contract-summary.md`), and the proxy crate `osm-extract-proxy` with two
binaries — the service (W2, W3) and the region-build tool (W1) — plus the
`Dockerfile`, the Railway configuration file and the weekly data-build
workflow that `infrastructure-specification.md` and `cicd-pipeline.md`
hand to this stage. The gate `team.md` names (`justfile` → `scripts/verify.sh`)
is wired to real commands so the TDD cycle below has a runnable command
from step 2 onward.

**Greenfield facts, checked on this machine on 2026-09-14:** `rustc 1.97.1`
and `cargo 1.97.1` are installed and are the toolchain to pin; `just` and
`cargo-audit` are present; `cargo-llvm-cov` is not (installed in step 2).
Every crate version in `tech-stack-decisions.md` is re-read from crates.io
at step 1 and the newest stable at that date is pinned — the design's
versions are the floor, not a ceiling.

**What is NOT in this Unit's scope** (and so not in this plan): the CI
workflow with the two tiers (`ci-pipeline`, stage 3.7, once for the whole
workspace); the osm2streets adapter, fixtures and the WASM client (U1, U2,
U4, U6); any Railway resource creation (`environment-provisioning`); the
uptime-monitor account (same). Anything here that a later stage needs is
named in `code-summary.md` at the end.

## Layout

```
Cargo.toml                       workspace: members = crates/*
rust-toolchain.toml              channel = "1.97.1"
justfile                         build / format / verify (+ test, cov)
scripts/verify.sh                the one gate: fmt, clippy -D warnings,
                                 build --locked, test, llvm-cov floor,
                                 dependency-manifest + no-Streetmix check
scripts/state-summary.sh         session-start summary (CLAUDE.md)
scripts/next-tasks.sh            READY tasks under docs/tasks/ (CLAUDE.md)
docs/tasks/                      task files (CLAUDE.md convention)
docs/dependencies.md             the committed dependency and asset manifest
Dockerfile                       three stages (infrastructure-specification ID-2)
railway.json                     builder, healthcheck, restart, replicas (ID-5)
data/current-build.toml          pointer: release tag, buildId, asset sha256s
.github/workflows/data-build.yml weekly region build (cicd-pipeline CP-4)
crates/cityloom-api-types/       Contract 1 wire types (serde), U9-owned
crates/osm-extract-proxy/
  Cargo.toml                     lib + bin osm-extract-proxy + bin region-build
  build.rs                       protobuf-codegen over proto/ (pure Rust, no protoc)
  proto/fileformat.proto, osmformat.proto, LICENSE, UPSTREAM  (vendored, MIT)
  src/lib.rs
  src/geo.rs        BoundingBox parse/validate (BR3.1), canonical integer box (BR3.2, PD-1)
  src/grid.rs       GridSpec, CellId, touched-cell span (BR3.3), CoverageIndex bitmap
  src/manifest.rs   Manifest, Region, CellEntry, buildId derivation (BR8.4), JSON
  src/store.rs      packed store: writer (build) and reader (index, pread, verify) (PD-2, PD-7)
  src/osm.rs        in-memory OSM elements (Node, Way) shared by build and cut
  src/encode.rs     the project PBF encoder (TS-3): deterministic blocks
  src/decode.rs     osmpbf-backed reader into src/osm.rs types (TS-2)
  src/cut.rs        clip assembly: touched cells, extent test, dedup, sort (BR5.x, PD-3)
  src/extract_key.rs  BLAKE3 key over canonical box + buildId (BR6.1)
  src/cache.rs      moka LRU by bytes, single-flight loader (BR6.2, BR6.3, PD-5)
  src/limiter.rs    keyed hash, two windows, sweep, Retry-After (BR2.x, SD-4, TS-8)
  src/failure.rs    FailureReason -> status/detail/phase mapping (BR10.x), record_failure
  src/counters.rs   atomics + latency buckets, periodic row (OD-2, PD-6)
  src/emit.rs       the one emitter module: subscriber config, fixed events (OD-1..OD-5)
  src/readiness.rs  Starting/Ready/Unready, start-up sequence (RD-1)
  src/config.rs     typed env configuration, fail-fast (SD-9, ID-7)
  src/http.rs       axum router, extract + readyz handlers, headers, timeout, catch-panic (SD-2, SD-6, PD-4)
  src/build/        region-build pipeline: download+md5 (TS-7), filter (BR8.2), slice (BR8.3), write (RD-8)
  src/bin/osm-extract-proxy.rs   main: config, start-up, serve, SIGTERM drain (RD-7)
  src/bin/region-build.rs        main: W1
  tests/            integration tests (router via tower::oneshot, log capture, build round-trip)
  benches/ or tests/bench.rs     the CI benchmark, #[ignore]d, run in the slow tier
```

Layering per `team.md`: the proxy is a server-side crate outside the
client's three-ring rule, but the same discipline applies inside it — `geo`,
`grid`, `manifest`, `osm`, `encode`, `cut`, `extract_key`, `failure` are
pure modules with no HTTP, no I/O and no logging dependency; `store`,
`cache`, `limiter`, `counters`, `emit`, `readiness`, `config`, `build` are
the I/O ring; `http` and the binaries are the outer ring. Cargo features
are not used to split them; module visibility is.

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "for every unit of work, write the failing test — or the",
  "scope": "feature",
  "test_strategy": "standard",
  "project_type": "greenfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Ordering**: for every unit of work, write the failing test — or the\n  executable form of the stated acceptance criterion — before writing the\n  implementation that satisfies it, including at the osm2streets adapter\n  boundary, where the expected output is committed as a fixture (see below)\n  before the adapter code that must reproduce it is written against that\n  fixture. This is the strictest of the options put to the interview and was\n  chosen deliberately over the mixed test-first-for-the-model /\n  spike-then-characterise-for-the-boundary approach the quality review\n  proposed; see `evidence.md` for that tension and why the human's explicit\n  choice stands.\n- **Coverage floor**: the org default's 80% line-coverage floor for `feature`\n  scope applies, measured with **`cargo-llvm-cov`** over a **stated set** of\n  Rust crates/modules in the Cargo workspace: the street/lane domain model\n  crate, the provenance model, the osm2streets adapter module, the\n  persistence layer, and the editing state machine. Because the client\n  itself compiles to WASM, there is no separate \"WASM glue\" category to\n  exclude — the whole client is WASM, and the denominator is simply the set\n  of crates/modules named above. The view/rendering layer (whichever DOM\n  framework Domain Design selects — see the open constraint below) and\n  visual-identity assets leave the denominator through an explicit,\n  committed `cargo-llvm-cov` ignore pattern — never by lowering the number.\n  Branch coverage on the adapter and the lane/provenance model is\n  **reported**, not gated, until a real number exists to set a floor against\n  at `nfr-requirements`.\n- **Every defect gets a failing regression test that reproduces it before the\n  fix lands.** Worth more here than on a team: it is what stops the same\n  osm2streets edge case returning in three months once the context that found\n  it the first time is gone.\n- **Merge gate — all of the following pass in CI before squash-merge to\n  `main`**, run through the `justfile`/`scripts/verify.sh` gate above:\n  1. `cargo build --workspace` and `cargo check --workspace` clean.\n  2. `cargo clippy --workspace --all-targets -- -D warnings` clean (clippy\n     denies warnings; nothing merges with an unresolved lint).\n  3. `cargo fmt --all -- --check` clean.\n  4. Unit and component tests green (`cargo test --workspace`).\n  5. Coverage floor met on the measured set above (`cargo-llvm-cov`).\n  6. The osm2streets adapter's golden-fixture suite green (see below).\n  7. Keyboard-path and accessibility tests green, for any Bolt touching the\n     editing surface.\n- **Two CI tiers, so the gate stays fast**: a fast tier on every push runs\n  1–6; a slow tier on PRs to `main` and nightly runs the browser-mode\n  keyboard/accessibility tests (item 7) plus the full release-mode WASM\n  build of the workspace — which now includes compiling the pinned\n  osm2streets crate dependency from source as an ordinary part of the Cargo\n  build, not a separate binding step. The build is cached by Cargo's own\n  incremental/target-directory caching keyed on `Cargo.lock`, never rebuilt\n  from scratch per push.\n- **The osm2streets adapter is characterised, not trusted.** Capture real\n  osm2streets output for a small, committed set of real OSM extracts (a\n  well-tagged street, a thinly-tagged street, a one-way, a street with a\n  separately mapped cycleway, one intersection) as committed fixture files,\n  and test everything above the adapter against those fixtures — never\n  against a live OSM fetch at test time, which would make the suite\n  non-deterministic and\n  impolite to a free public API. This suite's job is to tell the builder the\n  day the dependency's behaviour moves underneath them, not to prove\n  osm2streets correct.\n- **Provenance is tested, not just rendered.** For any Bolt touching import or\n  the editor, the test floor includes at least one test asserting that a\n  thinly-tagged fixture yields an `inferred` value, and one asserting the UI\n  renders it distinguishably from a `mapped` one. This is what makes Q3's\n  success criterion checkable rather than aspirational.\n- **Accessibility testing is two distinct things, not one axe-core check.**\n  Automated tooling (axe-core, driven from Playwright) is the floor for\n  contrast, names, roles and landmarks in the surrounding UI — it cannot see\n  inside a `<canvas>` element at all, so it is not evidence about the editing\n  surface itself if that surface turns out to be canvas-rendered. The\n  per-action check is a keyboard-only interaction test for every editing\n  action (select a lane, change its type, change its width, extend along the\n  corridor, undo), asserting both the resulting model state and the announced\n  accessible name/state. A defined manual screen-reader walkthrough of the\n  full edit path happens before each stage's release. **WCAG 2.1 AA is not\n  fully CI-verifiable**; a green pipeline is not read as conformance.\n- **Open constraint on Domain Design — more load-bearing now, not less, under\n  Rust/WASM (Q5 defers the editing-surface choice; it does not remove this\n  consequence):** the affirmation-gate decision to build the client in Rust\n  compiled to WebAssembly does not force a raster-canvas editing surface.\n  Rust/WASM UI frameworks split on exactly this axis: Leptos renders real DOM\n  nodes through fine-grained reactivity with no virtual DOM, and Yew and\n  Dioxus also render real DOM nodes, through virtual-DOM diffing — none of\n  the three requires drawing to `<canvas>`. So Q5's choice remains live and\n  is now a framework-selection question for Domain Design as much as a\n  rendering-technique one. If Domain Design nonetheless selects a raster\n  canvas for the interactive lane elements — in any of these frameworks, or\n  by hand-rolling WASM-driven canvas drawing — automated accessibility\n  verification of the editing surface itself is not available: axe-core\n  cannot inspect canvas contents, and WCAG 2.1 AA conformance there can only\n  be checked by the manual screen-reader pass, every release, indefinitely.\n  A DOM-rendering choice (Leptos, Yew, or Dioxus used in its normal mode)\n  makes each lane a real, focusable, announceable element and keeps the\n  automated floor meaningful; a canvas choice does not, regardless of which\n  language drew the canvas. This is not a decision made here; it is the cost\n  Domain Design is choosing against if it picks canvas.\n- No snapshot tests of rendered geometry — pure maintenance cost for one\n  builder, and they fail on every legitimate visual change, training the\n  habit of regenerating snapshots without reading them.\n- No performance/load testing without a stated NFR target — none is fixed yet\n  (`constraint-register.md`), and load-testing a single-instance $5/month app\n  against no target is waste. One cheap budget guard instead: assert a\n  ceiling on the built client bundle and the WASM artifact size in CI, which\n  protects the hosting budget (OC-4) and phone usability at once.\n- The Test Strategy Standard's \"E2E skipped unless NFR requirements exist\"\n  does **not** apply to the keyboard/accessibility browser tests above — WCAG\n  2.1 AA is itself a committed non-functional requirement, and a real browser\n  is its only validation method.\n- **Not yet resolved, carried to Requirements Analysis rather than decided\n  here**: whether the \"week to under a day\" workflow-time claim\n  (`initiative-brief.md`) becomes an acceptance criterion. `raid-log.md` R-4\n  records it as unvalidated, with no baseline and no user timed yet. It stays\n  a hypothesis until a real baseline exists; a later stage should not be able\n  to invent a threshold nobody measured."
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Meet an 80% line-coverage floor.",
      "Run the selected tests in CI before merge."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:ed4bdcf58363e72548317cd6aa9374f8d2b8448e61759898d0eacfa80c588f09",
  "contract_sha256": "sha256:154405be1006cdb8702ac3d4735e1c9938381c308660d7eb2fc281364e7e3f86"
}
```

The contract's `plan_profile.steps` is the ordering baseline below. The
"Frontend behavior" layer is genuinely inapplicable — this Unit has no user
interface (`security-requirements.md`, NFR4 row of the nfr-requirements
traceability) — and is omitted; every other layer runs Red → Green →
Refactor in order. Red steps record the failing command's output in
`unit-test-instructions.md`'s log before Green begins.

## Steps

Every step names the design element or rule it implements and the story
criterion it traces to. This Unit's story is US3.1's fetch half: AC3.1.1
(the bytes the import needs), AC3.1.4 / AC3.1.5 (deterministic bytes the
fixtures can be cut from), AC3.1.6 (the budget), and the failure names of
AC7.1.1 / AC7.1.2 / AC7.2.1 / AC7.2.2 via Contract 1. AC3.1.2, AC3.1.3 and
AC3.1.7 are N/A for this Unit as `functional-design/traceability.json`
records.

### Step 1 — Project structure and production configuration skeleton

- [x] 1.1 Create the Cargo workspace: root `Cargo.toml` (`members = ["crates/*"]`, `resolver = "3"`, workspace `[workspace.package]` with edition 2024, licence, repository), `rust-toolchain.toml` pinning `1.97.1` with `rustfmt`, `clippy`, `llvm-tools-preview` components, a root `rustfmt.toml` (defaults, edition 2024) and `clippy.toml` if any lint needs a threshold. Extend `.gitignore` with `target/`, `.env`, `.env.*`.
- [x] 1.2 Re-read every crate in `tech-stack-decisions.md`'s dependency table on crates.io and pin the newest stable of each in the crate manifests: `axum`, `tokio`, `tower`, `tower-http` (features `set-header`, `catch-panic`), `serde`, `serde_json`, `osmpbf`, `protobuf`, `protobuf-codegen`, `flate2`, `moka` (`future`), `blake3`, `md-5`, `tracing`, `tracing-subscriber` (`json`), `reqwest` (`rustls-tls`, `stream`), plus `thiserror` for typed errors, `toml` for the pointer file's shape if the build tool writes it, `tempfile` and `tokio-test`/`tower` `util` as dev-dependencies. Record each with origin and licence in `docs/dependencies.md` (the asset and dependency manifest `team.md` names). No Streetmix package anywhere.
- [x] 1.3 Create `crates/cityloom-api-types`: `FailureReason` (the eight values of `entities.md`, with the `functional-spec.md` amendments: `invalid_area` added, `rate_limited` replacing `upstream_rate_limited`, `malformed_extract` reserved), `ApiError { reason, detail }`, `ReloadRequired { reason: "reload_required", current_build }`, `is_retryable()` per BR10.2, all `serde` with the wire spelling of Contract 1. This crate is the source of truth Contract 1's OpenAPI block is generated from later; it has no dependency but `serde`.
- [x] 1.4 Create `crates/osm-extract-proxy` with the module skeleton in "Layout" (empty `pub mod` declarations, `lib.rs`, two `src/bin/*.rs` stubs that compile), `build.rs` running `protobuf-codegen` over `proto/` with the pure-Rust parser (no `protoc`), and the vendored `fileformat.proto` / `osmformat.proto` from the OSM PBF upstream with their MIT `LICENSE` and an `UPSTREAM` file recording the repository and commit (TS-3; `docs/dependencies.md` gains the row).
- [x] 1.5 Wire the gate: `justfile` targets `build` (`cargo build --workspace --locked`), `format` (`cargo fmt --all`), `test` (`cargo test --workspace`), `cov` (`cargo llvm-cov` with the floor), `verify` (runs `scripts/verify.sh`); rewrite `scripts/verify.sh` to run, in order, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace --locked`, `cargo test --workspace`, the coverage floor (step 2.2), and the manifest check — fail if `Cargo.lock` or `docs/dependencies.md` mentions `streetmix` (case-insensitive) or if any `[[package]]` in `Cargo.lock` is absent from `docs/dependencies.md`'s direct-dependency table (transitive crates are listed by the lockfile itself; the manifest lists direct ones with licence and origin). Add `scripts/state-summary.sh` (prints the active workflow's current stage from `aidlc-state.md`, the last handoff note, and READY tasks) and `scripts/next-tasks.sh` (lists `docs/tasks/*.md` whose status line is `READY`), create `docs/tasks/README.md` describing the task-file shape from `docs/templates/task-template.md`. This closes `team.md`'s "`CLAUDE.md` is brought in line with reality" item.
- [x] 1.6 `cargo build --workspace --locked` and `cargo clippy --workspace --all-targets -- -D warnings` pass on the skeleton; commit `Cargo.lock`.

### Step 2 — Bootstrap the test runner and record the exact unit-scoped command

- [x] 2.1 Confirm `cargo test -p osm-extract-proxy` and `cargo test -p cityloom-api-types` run (zero tests) and record both in `unit-test-instructions.md` as the unit-scoped commands; `cargo test -p osm-extract-proxy --test <name>` for a single integration test file.
- [x] 2.2 Install `cargo-llvm-cov` (`cargo install cargo-llvm-cov --locked`) and record the coverage command scoped to this Unit: `cargo llvm-cov -p osm-extract-proxy -p cityloom-api-types --fail-under-lines 80 --ignore-filename-regex '(src/bin/|/out/|proto_gen)'` — the ignore pattern removes only the two `main.rs` entry points and the generated protobuf code, per `team.md` (never by lowering the number). Put the same invocation in `justfile` `cov` and `scripts/verify.sh`.
- [x] 2.3 Add the test-support module `tests/common/mod.rs`: a synthetic-OSM builder (nodes and highway ways placed on a chosen grid), an in-memory region encoder using `src/encode.rs` once it exists (so the build tool's tests never fetch anything), a `tempdir` helper, and a `tracing` capture layer that collects emitted JSON rows into a `Vec<String>` for the log-discipline tests.

### Step 3 — Data model: Red

Tests written first, run, and their failing output recorded. Targets (5–8 tests per module, Standard strategy):

- [x] 3.1 `geo`: parse of `bbox` — four decimals, strict min < max, range checks, `NaN`/`inf`/`-0`/exponent/extra-number/empty cases → `invalid_area` (BR3.1, NFR6.4.1); canonicalisation to 1e-5-degree integer units rounded outward (BR3.2, PD-1, `performance-requirements.md` "Canonicalisation precision"); a property test over random strings never panics.
- [x] 3.2 `grid`: 0.01° cells as exactly 1,000 units; column/row from an integer box; a box straddling a corner touches 4 cells, a box on an edge touches only what it overlaps by area, a 13-cell rectangle exceeds the span bound of 12 (NFR1.1.5, BR3.3); `CoverageIndex` bitmap over the regions' rectangle answers covered/uncovered and a box touching one uncovered cell is uncovered (BR4.1).
- [x] 3.3 `manifest`: JSON round-trip of `Manifest { buildId, builtAt, gridSpec, regions[], filterProfile, cellCount, totalBytes, coverage, cells[{cellId, offset, length, digest, wayCount, nodeCount}] }`; `buildId` = BLAKE3 over region source digests in configured order plus the `GridSpec` (BR8.4, TS-5) is stable across runs and changes when any input changes.
- [x] 3.4 `osm` + `extract_key`: element ordering (nodes then ways, by id, BR5.3); the key is the lowercase hex BLAKE3 of the five-decimal box string, a newline, the `buildId` (BR6.1) and differs across builds (BR6.4).
- [x] 3.5 `cityloom-api-types`: serde spelling of every `FailureReason`, `ApiError`, `ReloadRequired`; `is_retryable` exactly matches BR10.2.

### Step 4 — Data model: Green

- [x] 4.1 Implement `geo`, `grid`, `manifest`, `osm`, `extract_key` and the api-types crate with the minimum that turns 3.x green; no HTTP, no I/O in these modules.

### Step 5 — Data model: Refactor

- [x] 5.1 Tighten types (newtypes for `CellId`, `Units`, `BuildId`, `ExtractKey`; `.get(i)`-style bounded accessors per `team.md`), remove duplication; suite green, clippy clean.

### Step 6 — Repository / data access: Red

- [x] 6.1 `encode` + `decode`: a clip encoded by the project encoder decodes with `osmpbf` to the same elements (TS-3 round-trip); the header declares `OsmSchema-V0.6` and `DenseNodes`; the same input yields byte-identical output on two runs (BR5.4); no timestamp or program-name field is written.
- [x] 6.2 `store`: the writer packs cells back to back in cell-id order and the reader's index (sorted, 24 bytes per entry, binary search) finds them; a positional read returns exactly the cell bytes; `verify_all` passes on a good store, fails on one flipped byte, on a missing cell and on a truncated file (NFR3.2.4, PD-2, PD-7); an absent entry is "covered but empty" (NFR3.1.10).
- [x] 6.3 `cache`: LRU by byte weight under a 32 MiB ceiling — fill past the ceiling and the oldest key is gone and the total stays under; an oversize clip is returned and not retained (NFR3.1.8, NFR5.1.1); `try_get_with` coalesces concurrent misses for one key into one loader run (BR6.3); a loader error is stored nowhere (BR6.2).
- [x] 6.4 `limiter`: 31st request in a minute → limited; 301st in an hour → limited with the minute clear; hits count; a new minute resets the minute count not the hour count; the sweep removes windows older than an hour; 10,000 entries ≤ 640 KB (NFR5.1.3, NFR2.1.3); `Retry-After` between 1 and 60 on a minute-limited request (NFR5.1.7); two service instances hash the same address differently (NFR6.3.2).
- [x] 6.5 `counters`: increments are visible in the next snapshot; latency goes into the right one of the nine buckets; `uptimeSeconds` grows (OD-2, PD-6).

### Step 7 — Repository / data access: Green

- [x] 7.1 Implement `encode` (protobuf-codegen types, zlib blobs, fixed field order), `decode` (`osmpbf`), `store` (writer and reader, `read_exact_at`, BLAKE3 verification pass), `cache` (`moka::future::Cache` with `EvictionPolicy::lru()` and a byte weigher), `limiter` (`Mutex<HashMap<u64, Windows>>`, `RandomState` key), `counters` (atomics).

### Step 8 — Repository / data access: Refactor

- [x] 8.1 Extract the store framing (magic, version) into one place shared by writer and reader; make the sweep interval and ceilings constructor parameters; green, clippy clean.

### Step 9 — Business logic: Red

- [x] 9.1 `cut`: a way with its extent intersecting the box is kept with every node it references, even nodes outside the box (BR5.2); a way in two touched cells appears once (BR5.3); output is sorted; a covered box with no ways yields the "nothing mapped" outcome and nothing to cache (BR4.2); the cut checks its deadline between cells and stops early (NFR3.1.7); per-cut memory stays under 8 MB at the span bound on the synthetic data (NFR5.1.2, asserted on allocated buffer sizes).
- [x] 9.2 `failure`: every `FailureReason` maps to exactly its status, retryability and project-authored detail from `functional-spec.md`'s table; a record carries exactly `occurredAt`, `reason`, `status`, `phase`, `detail` (BR7.2, BR10.1).
- [x] 9.3 `readiness`: the start-up sequence enters Ready on a good build and Unready on a missing manifest, an unreadable manifest, a missing store, and a digest mismatch, each with the right `check` value and failing-cell count (RD-1, OD-4); invalid configuration exits non-zero rather than Unready.
- [x] 9.4 `build`: from two synthetic region files, the pipeline verifies the `.md5` (a corrupted download and a mismatched checksum both fail the build with nothing written, NFR7.2.2, BR8.1), keeps highway ways and their nodes and records the profile (BR8.2), writes a way into every cell its geometry touches (BR8.3), writes the store and manifest to a temporary location and moves them into place only at the end (RD-8), and yields the same `buildId` and byte-identical cells on a second run (NFR7.2.1); a box in each region is covered and a box between them is not (NFR2.1.4); the manifest's in-memory size is ≤ 80 bytes per cell (NFR2.1.5).
- [x] 9.5 `emit`: `record_failure` emits one JSON row with the five fields and the level per reason (OD-3); `emit_ready`/`emit_unready`/`emit_shutdown` carry the OD-4 fields; the subscriber emits no span events.

### Step 10 — Business logic: Green

- [x] 10.1 Implement `cut`, `failure`, `readiness`, `build/` (download with `reqwest` + `rustls` streaming to disk while hashing MD5 and BLAKE3, with the project `User-Agent`; filter; two-pass slice with a compact node-id set; store and manifest writer), and `emit`.

### Step 11 — Business logic: Refactor

- [x] 11.1 One `FailureMapping` table used by `failure`, `emit` and `http`; the deadline threaded as a single `Instant`; green, clippy clean.

### Step 12 — API / endpoint: Red

Integration tests through `tower::ServiceExt::oneshot` against the router, no socket:

- [x] 12.1 `GET /api/extract` with a missing or different `x-client-build` → 426 `ReloadRequired` before anything else (BR1.1); a wrong `bbox` → 400 `invalid_area`; a 13-cell box → 400 `area_too_large`; an uncovered box → 404 "not an area this deployment covers"; a covered empty box → 404 "nothing mapped here"; a good box → 200 `application/octet-stream` with `x-extract-key` a lowercase hex digest; the second identical request is a hit with identical bytes (BR5.4) and the same key.
- [x] 12.2 A stalled cutting phase (test hook) → `503 timeout` within 3,050 ms and the slot is free afterwards (NFR1.1.1, NFR3.1.7); eight misses against four slots never exceed four concurrent cuts (NFR1.1.6); a handler panic → `503 internal` with the typed body and exactly one failure row (NFR6.4.2, BR10.4).
- [x] 12.3 Limiter through HTTP: the 31st request → 429 with `Retry-After` 1–60; a spoofed `X-Forwarded-For` does not change the hash the platform's `X-Real-IP` produces (NFR6.3.3, ID-4); with no `X-Real-IP` the peer address is used.
- [x] 12.4 Headers: every response carries `Strict-Transport-Security`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Content-Security-Policy: default-src 'none'`, `Cache-Control: private, no-store` and no `Set-Cookie` (NFR6.4.3, NFR6.3.4); an `OPTIONS` preflight with `Origin` and `Access-Control-Request-Headers: x-client-build` gets no `Access-Control-Allow-*` header (NFR5.1.8); `POST /api/extract` is rejected by the router (NFR6.4.4).
- [x] 12.5 `GET /readyz` → 200 `ready` when Ready, 503 `not-ready` otherwise, with the header set, no build detail, and unaffected by a limited requester (NFR3.2.5); an extract request while Unready → 503 `upstream_unavailable` (NFR3.2.4).
- [x] 12.6 **The log-discipline test** (NFR6.3.1, the enforcement for the whole L7 layer): drive one hit, one miss and every failure reason with the capture layer installed; assert the captured output contains none of the address used, any coordinate of the box, the returned key, or any touched cell id; and that a successful request emitted no row at all (BR7.4).
- [x] 12.7 The counters row after the sequence above shows the expected `extractsServed`, `bytesServed`, `cacheHits`, `cacheMisses`, `requestsLimited`, `failuresByReason` and `lastExtractBytes` (OD-2, W5).

### Step 13 — API / endpoint: Green

- [x] 13.1 Implement `http` (router with `GET /api/extract` and `GET /readyz` only; `SetResponseHeader` layer; `CatchPanicLayer` mapped to the typed `internal` body; the build-stamp check before the `tokio::time::timeout(3 s)` around the rest; the address read from `X-Real-IP` with peer fallback), `config` (typed `CITYLOOM_*` variables with the provisional defaults; `RAILWAY_GIT_COMMIT_SHA` or `CITYLOOM_BUILD_ID` as the build stamp validated against `[A-Za-z0-9._-]{1,64}`; fail-fast), and the two binaries (`osm-extract-proxy`: configure the subscriber first, run RD-1, bind `PORT`, spawn the counters and sweep tasks, handle `SIGTERM`/`SIGINT` with a 3-second drain and the final counters row; `region-build`: W1 from configuration, exit code by outcome, prints the `buildId` and the pointer's fields).

### Step 14 — API / endpoint: Refactor

- [x] 14.1 One `AppState` with the store, index, coverage, cache, limiter, counters and Ready flag behind `Arc`; handler split into the W3 phases as named functions so each phase's failure carries its `phase`; green, clippy clean, `cargo llvm-cov` at or above 80% on the measured set.

### Step 15 — Environment and build configuration

- [x] 15.1 `Dockerfile` (ID-2): stage 1 `rust:1.97.1-bookworm` builder, `cargo build --release --locked --bin osm-extract-proxy`; stage 2 a `debian:bookworm-slim` data stage that reads `data/current-build.toml`, downloads the two assets from the GitHub Release it names with `curl`, verifies each SHA-256, and fails the build on any mismatch; stage 3 `debian:bookworm-slim` (or `gcr.io/distroless/cc-debian12:nonroot`, whichever the developer confirms runs the binary) as a non-root user with `/data/store.bin`, `/data/manifest.json` and the binary; `EXPOSE`d port from `PORT`. Layer order: the data stage's inputs are only the pointer file.
- [x] 15.2 `railway.json` (ID-5): `build.builder = DOCKERFILE`, `deploy.healthcheckPath = /readyz`, `deploy.healthcheckTimeout = 120`, `deploy.restartPolicyType = ON_FAILURE`, `deploy.restartPolicyMaxRetries = 10`, `deploy.numReplicas = 1`; the deployment variables `RAILWAY_DEPLOYMENT_DRAINING_SECONDS=5` and the `CITYLOOM_*` defaults are documented in `docs/deploy.md` as the variables environment-provisioning sets (they are not secrets; the file records their names and values).
- [x] 15.3 `data/current-build.toml`: the pointer's shape (`release_tag`, `build_id`, `store_sha256`, `manifest_sha256`, `published_at`) with a documented placeholder value that makes the `Dockerfile`'s data stage fail loudly until the first data build has run — never a silently empty store.
- [x] 15.4 `.github/workflows/data-build.yml` (CP-4): `schedule` Monday 06:00 UTC + `workflow_dispatch`; `concurrency: data-build`; `permissions: contents: write`; steps: checkout, toolchain from `rust-toolchain.toml`, `cargo run --release --locked --bin region-build -- --config data/regions.toml --out target/data`, compare the printed `buildId` with the pointer (exit `unchanged`), delete any incomplete same-tag release, create the release `data-<buildId>` with both assets and their SHA-256s, download-and-hash both stored assets, then commit the pointer with `GITHUB_TOKEN`. `data/regions.toml` names the first configured region (one Canadian province, a configuration choice per `functional-spec.md`; the developer picks the smallest province whose data exercises the path — Prince Edward Island at 10.8 MB — so the first real build is cheap, and records that the choice is configuration).
- [x] 15.5 `docs/deploy.md`: the runbook `reliability-design.md` RD-9 says is one line — redeploy the last good build — plus the environment-provisioning handoff list copied from `infrastructure-specification.md` so the next stage has it beside the code.
- [x] 15.6 The CI benchmark as `tests/bench.rs` marked `#[ignore]`: 200 misses and 1,000 hits over the synthetic build, asserting p95 ≤ 300 ms and ≤ 30 ms and recording CPU time per miss; the exact command (`cargo test -p osm-extract-proxy --release --test bench -- --ignored`) is what `ci-pipeline` puts in the slow tier.

### Step 16 — Documentation and traceability

- [x] 16.1 Crate-level and module-level doc comments naming the rule or design element each module implements; `README.md` at the workspace root with the layout above, the gate, and how to run the service locally against a synthetic build (`cargo run --bin region-build -- --synthetic` writing a tiny store to `target/data`, then `cargo run --bin osm-extract-proxy`).
- [x] 16.2 `docs/dependencies.md` complete (every direct crate with version, origin, licence; the vendored `.proto` files; no Streetmix), `scripts/verify.sh` green end to end.
- [x] 16.3 `<record>/construction/osm-extract-proxy/code-generation/source-manifest.json` listing every path created (workspace root files, `crates/`, `scripts/`, `docs/`, `data/`, `.github/workflows/data-build.yml`, `Dockerfile`, `railway.json`), and `traceability.json` mapping AC3.1.1, AC3.1.4, AC3.1.5, AC3.1.6, every `BRx.y` of `rules.md`, and every `NFRx.y.z` of the nfr-requirements files to the implementation or test file that delivers it.
- [x] 16.4 Update `docs/state/handoff.md` (≤ 30 lines) and `docs/state/features.md` with the proxy's status, per `CLAUDE.md`.

## Story-to-step traceability

| Story criterion / rule group | Steps |
|---|---|
| AC3.1.1 (bytes the import needs: complete ways with nodes) | 9.1, 10.1, 12.1, 13.1 |
| AC3.1.4, AC3.1.5 (deterministic bytes the fixtures are cut from) | 6.1, 7.1, 9.1, 12.1 |
| AC3.1.6 (the budget elapsing is a failure, not a spinner) | 12.2, 13.1 |
| AC7.1.1, AC7.1.2 (one named reason per failure) | 9.2, 12.1, 12.2 |
| AC7.2.1, AC7.2.2 (retryable set) | 3.5, 9.2 |
| BR1 build stamp | 12.1, 13.1 |
| BR2 requester limiting | 6.4, 7.1, 12.3 |
| BR3 validation, canonicalisation, span | 3.1, 3.2, 4.1 |
| BR4 coverage | 3.2, 9.1, 12.1 |
| BR5 clipping and determinism | 6.1, 9.1, 10.1 |
| BR6 caching and key | 3.4, 6.3, 7.1 |
| BR7 privacy and logging | 9.5, 12.6, 12.7 |
| BR8 regional build | 9.4, 10.1, 15.4 |
| BR9 readiness | 9.3, 12.5 |
| BR10 failure mapping and timing | 9.2, 12.2 |
| NFR6.4.3 headers, NFR5.1.8 no CORS, NFR6.4.4 listener | 12.4, 13.1 |
| Infrastructure ID-2, ID-5, ID-7, CP-4 | 15.1–15.5 |
| `team.md` gate and `CLAUDE.md` alignment | 1.5, 2.2, 16.2 |

## Quality targets carried in, not negotiable

3,000 ms request budget; p95 ≤ 300 ms miss / ≤ 30 ms hit (asserted in the
ignored benchmark, reported at B-3); 0.01° cells, span bound 12; 5-decimal
outward canonicalisation; 30/min and 300/hour; 4 slots; 32 MiB cache;
≤ 8 MB per cut; Ready ≤ 30 s (measured on the synthetic build, reported);
80% line coverage on the measured set with the stated ignore pattern only;
`clippy -D warnings`; `cargo fmt --check`; `--locked` builds. None is
lowered to make a step pass; a gap is surfaced in `code-summary.md`.
