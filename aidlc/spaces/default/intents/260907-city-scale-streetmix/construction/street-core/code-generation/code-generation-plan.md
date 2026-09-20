# Code Generation Plan — `street-core` (U2)

Upstream inputs: `entities.md`, `rules.md`, `functional-spec.md`
(functional-design), `security-design.md` (nfr-design), `tech-stack-decisions.md`
(nfr-requirements), `unit-of-work.md` (units-generation), `requirements.md`
(requirements-analysis), `team.md`/`project.md` (practices).

## What is built

The `street-core` crate: a new workspace member with zero dependencies
beyond the Rust standard library. It implements `Provenance`, `Dimension`,
`Lane`, `Street`, `StreetNetworkGraph`, and the `StreetSource` port trait
— the imported baseline this project's other client Units will build on.

**Greenfield facts, checked 2026-09-17:** the workspace's existing
`rust-toolchain.toml` (from `osm-extract-proxy`) pins `rustc`/`cargo`
1.97.1; this crate reuses it, adding no new toolchain requirement.

**What is NOT in this Unit's scope**: anything that imports real OSM data
(that's `u4-street-import`'s `StreetImportAdapter`, which will implement
the `StreetSource` port this crate declares), any editing/overlay logic
(`u5-design-editing`), any serialization (`u3-design-payload-spec`), and
any rendering (`u6-client-surfaces`).

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

This crate is entirely "Data model" layer — no repository, API, or
frontend behaviour exists here. The plan below runs Red→Green→Refactor
once per entity/rule group rather than forcing the other four generic
layers.

## Layout

```
crates/street-core/
  Cargo.toml           no dependencies beyond std (nfr-requirements
                        tech-stack-decisions.md); added to the workspace
                        root Cargo.toml's members
  src/lib.rs            module declarations, forbid(unsafe_code) (nfr-design SD-3)
  src/provenance.rs     Provenance enum, Dimension struct (BR1.1, BR1.2)
  src/lane.rs           Lane struct, derived laneKey (BR2.1), bounds-checked
                        access helpers (BR2.2)
  src/street.rs         Street struct, identity (BR3.1), carriageway width
                        calculation (BR4.1), immutability via shared-only
                        constructors (BR5.1)
  src/graph.rs          StreetNetworkGraph struct
  src/source.rs         StreetSource port trait (BR7.1) — no implementation
                        here
```

## Story-to-step traceability

This Unit serves AC4.1.1, AC7.3.1, AC7.3.4 (provenance/lane-identity),
and AC6.2.1/AC6.2.3 (carriageway width) per `functional-design/traceability.json`.

## Steps

### Step 1 — Project structure

- [x] 1.1 Add `crates/street-core` to the workspace root `Cargo.toml`'s
  `members`. Create `crates/street-core/Cargo.toml` with no dependencies
  beyond `[dev-dependencies]` (none needed either — `cargo test` alone
  suffices for a pure data model). (The root `Cargo.toml`'s `members =
  ["crates/*"]` glob already covers a new `crates/street-core` directory
  — no edit to the workspace manifest was needed.)
- [x] 1.2 Create `src/lib.rs` with `#![forbid(unsafe_code)]` and module
  declarations for the five files above (empty stubs that compile).
- [x] 1.3 `cargo build -p street-core` and `cargo clippy -p street-core --all-targets -- -D warnings` pass on the skeleton.

### Step 2 — Bootstrap the test runner

- [x] 2.1 Confirm `cargo test -p street-core` runs (zero tests) and record
  it in `unit-test-instructions.md` as the unit-scoped command.

### Step 3 — Provenance and Dimension: Red

- [x] 3.1 `provenance`: a `Dimension` cannot be constructed without a
  `Provenance` (compile-time — assert via a doctest or a type-level test
  that the constructor requires both arguments); `Provenance::UserSet`
  has no method that transitions it back to `Mapped`; a correction helper
  transitions `Mapped`→`UserSet` and `Inferred`→`UserSet`, and
  `UserSet`→`UserSet` is a no-op (BR1.1, BR1.2). 5-6 tests. (6 tests
  written; the "cannot be constructed without a Provenance" invariant is
  a compile-time fact of the two-required-field struct, not separately
  tested with a dev-dependency such as `trybuild` — see `code-summary.md`.)

### Step 4 — Provenance and Dimension: Green

- [x] 4.1 Implement `Provenance` (derive `Clone, Copy, Debug, PartialEq`)
  and `Dimension { metres: f64, provenance: Provenance }` with a
  `correct(&self, new_metres: f64) -> Dimension` method that always sets
  `UserSet`. (Named `corrected` rather than `correct`, since it returns a
  new value rather than mutating `self` — see `code-summary.md`.)

### Step 5 — Provenance and Dimension: Refactor

- [x] 5.1 Tighten visibility (no public field mutation outside the
  correction method); clippy clean.

### Step 6 — Lane: Red

- [x] 6.1 `lane`: `Lane::key()` returns the same value for two `Lane`s
  constructed with identical `(lane_type, direction, ordinal_from_kerb)`
  and different values for any differing field (BR2.1); a bounds-checked
  lane-list accessor (`.get(i)`-style) returns `None` for an out-of-range
  index and `Some` for a valid one, never panicking (BR2.2). 5-6 tests.
  (7 tests written.)

### Step 7 — Lane: Green

- [x] 7.1 Implement `Lane` and its derived `LaneKey` newtype; implement
  the bounds-checked accessor on the lane-list type used by `Street`.

### Step 8 — Lane: Refactor

- [x] 8.1 Extract `LaneKey` derivation into one pure function shared by
  construction and any future re-derivation; clippy clean. (`LaneKey`
  derivation lives inline in `Lane::new`, the only construction path —
  see `code-summary.md` for why a separate free function was not
  extracted.)

### Step 9 — Street and carriageway width: Red

- [x] 9.1 `street`: two `Street`s built from the same `osm_way_id` but
  different `bounding_node_ids` are distinct and independently keyed
  (BR3.1); `carriageway_width()` sums lane widths strictly between two
  kerb-buffer lanes when both are present, and sums all lanes except
  walkable types and verge buffers when they are not (BR4.1), including a
  case where every contributing width is `Inferred` (still computable,
  BR1.1's provenance-carrying design makes this representable); no
  `&mut` accessor compiles against `Street` (a `trybuild`-style
  compile-fail test, or a doc comment asserting the invariant if
  `trybuild` is judged excess for one crate — decided at Green based on
  what's already in the dependency-free budget). 6-7 tests. (9 tests
  written — 2 extra to cover the carriageway-width provenance-aggregation
  rule made explicit at Green: see `code-summary.md`. Decided the doc
  comment over `trybuild`, per SD-2's zero-dependency budget.)

### Step 10 — Street and carriageway width: Green

- [x] 10.1 Implement `Street { osm_way_id, bounding_node_ids, name,
  lanes: Vec<Lane>, carriageway_width: Dimension }` with a constructor
  that computes `carriageway_width` from the lane list per BR4.1, and no
  `&mut self` method anywhere on the type.

### Step 11 — Street and carriageway width: Refactor

- [x] 11.1 Extract the carriageway-width calculation into its own pure
  function for testability in isolation from `Street` construction;
  clippy clean.

### Step 12 — StreetNetworkGraph and StreetSource: Red

- [x] 12.1 `graph`: a `StreetNetworkGraph` holds a set of `Street`s and
  reports them via a shared-reference iterator; `source`: the
  `StreetSource` trait's method signature returns a `Result<StreetNetworkGraph,
  E>` for a generic associated error type `E`, with a fake in-test
  implementation proving the trait is object-safe / usable without
  osm2streets compiled in anywhere in this crate's dependency tree
  (this is the test that operationalises BR7.1 — a fake `StreetSource`
  compiling and running with zero osm2streets-related code present).
  4-5 tests. (3 `graph` tests + 3 `source` tests = 6.)

### Step 13 — StreetNetworkGraph and StreetSource: Green

- [x] 13.1 Implement `StreetNetworkGraph` and the `StreetSource` trait.

### Step 14 — StreetNetworkGraph and StreetSource: Refactor

- [x] 14.1 Clean up naming/visibility; clippy clean; `cargo llvm-cov -p
  street-core --fail-under-lines 80` (no ignore pattern needed — every
  line here is in-scope per `team.md`'s coverage-floor set, "the
  street/lane domain model crate"). (94.84% line coverage achieved —
  see `code-summary.md`. Also reordered each module so its `#[cfg(test)]
  mod tests` block follows the production code, per a clippy
  `items_after_test_module` lint this toolchain enforces.)

### Step 15 — Documentation and traceability

- [x] 15.1 Module-level doc comments naming the rule each module
  implements. `source-manifest.json` and `traceability.json` under this
  Unit's record directory.

## Quality targets carried in, not negotiable

80% line coverage on this crate (no ignore pattern — it is the coverage
floor's namesake crate); `clippy -D warnings`; `cargo fmt --check`; zero
dependencies beyond std; `#![forbid(unsafe_code)]`; no `&mut` public
method on `Street`, `Lane`, or `StreetNetworkGraph`.
