# Code Generation Plan — `design-payload-spec` (U3)

Upstream inputs: `entities.md`, `rules.md`, `functional-spec.md`
(functional-design, this Unit); `security-design.md` (nfr-design, this
Unit); `tech-stack-decisions.md` (nfr-requirements, this Unit);
`contract-summary.md` Contract 3 (inception/contract-design);
`unit-of-work.md` U3 (inception/units-generation).

This Unit is a `spec`-kind Unit: a standalone, dependency-light Rust
crate (`cityloom-design-payload`) that defines the wire/storage shape
for a design payload. It has no API/endpoint layer, no repository/data
access layer of its own (it IS the data model that other Units' data
access layers use), and no frontend. Only the "Data model / database
behavior" and "Business logic" testable layers from the Testing
Contract apply — the fail-closed version check (SD-1) is business
logic living in a `TryFrom` wrapper, everything else is plain
`serde`-derived data shape.

## Story/AC traceability

| Plan step | Story / AC | Rule |
|---|---|---|
| Steps 3-5 | AC8.1.1, AC8.1.2 (round-trip fidelity) | BR2.1 |
| Steps 3-5 | AC8.3.1-8.3.5 (payload shape) | BR1.1, BR3.1, BR5.1 |
| Steps 6-8 | AC9.2.1 (export reuse) | BR3.1 |
| Steps 6-8 | AC11.2.1-11.2.3 (provenance identifiable on the wire) | BR3.1 |
| Steps 3-5, 6-8 | NFR6.4.4 (fail-closed version check) | BR1.1 |
| Steps 3-5, 6-8 | NFR6.4.5 (Correction structural provenance) | BR4.1 |
| Steps 3-5, 6-8 | NFR6.3.1 (no PII-shaped fields) | — |

## Steps

- [x] Step 1: Project structure and production configuration skeleton.
      Create `crates/cityloom-design-payload/` as a new workspace member:
      `Cargo.toml` (name `cityloom-design-payload`, `serde` with the
      `derive` feature as the only dependency beyond std, `serde_json`
      as a dev-dependency for round-trip tests), `src/lib.rs`. Add the
      crate to the workspace root `Cargo.toml`'s `members` list.
- [x] Step 2: Bootstrap the minimal test runner/configuration and record
      the exact unit-scoped command. This workspace already has a
      working `cargo test` runner (established by `osm-extract-proxy`
      and `street-core`); the unit-scoped command for this crate is
      `cargo test -p cityloom-design-payload`. Record it in
      `unit-test-instructions.md`.
- [x] Step 3: Data model / database behavior — Red: write the failing
      tests for the six entity types (`DesignPayload`, `StreetKey`,
      `Dimension`, `LaneEdit`, `Correction`, `ImportFingerprint`)
      deriving `Serialize`/`Deserialize` and round-tripping through
      `serde_json` with every field populated, including a
      thinly-tagged `Inferred` `Dimension` and a `Correction` (BR2.1,
      BR3.1, AC8.1.1, AC8.1.2, AC11.2.3). Record the failing command
      output (types do not exist yet).
- [x] Step 4: Data model / database behavior — Green: implement the six
      struct/enum types in `src/lib.rs` (or split into `src/types.rs`
      if that reads better) exactly per `entities.md`'s schema —
      `Provenance` enum (`Mapped`, `Inferred`, `UserSet`), `Dimension`,
      `StreetKey`, `LaneEdit`, `Correction`, `ImportFingerprint`,
      `DesignPayload` — with `#[derive(Serialize, Deserialize, ...)]`
      and `#[serde(rename_all = "camelCase")]` per Contract 3's wire
      naming (`payloadVersion`, `boundingNodeIds`, etc). `Correction`
      carries no `provenance` field (SD-2/BR4.1 — structural, not a
      value). Pass Step 3's tests.
- [x] Step 5: Data model / database behavior — Refactor: tidy field
      ordering/doc comments while tests stay green. No behavior change.
- [x] Step 6: Business logic — Red: write the failing tests for the
      fail-closed version check (SD-1, BR1.1, NFR6.4.4) — a
      `DesignPayload::from_json(&str) -> Result<DesignPayload, PayloadError>`
      (or equivalent `TryFrom<&str>`) that: (a) parses payload bytes
      generically enough to read `payloadVersion` first, (b) returns
      `Err(PayloadError::UnsupportedPayloadVersion(n))` for any value
      other than `1` without ever constructing a `DesignPayload` from
      the mismatched-version bytes, (c) on version 1 returns the fully
      deserialized `DesignPayload`. Include one test that an
      unrecognised version's bytes (with otherwise-matching field
      shapes) is rejected rather than silently parsed. Record the
      failing command output.
- [x] Step 7: Business logic — Green: implement `PayloadError` (a
      closed enum, not a raw `serde_json::Error` leak past this
      boundary) and the fail-closed wrapper. Pass Step 6's tests.
- [x] Step 8: Business logic — Refactor: tidy while tests stay green.
- [x] Step 9: Environment/build configuration — none beyond Step 1's
      `Cargo.toml`; this crate has no environment variables, no
      database, no network. Confirm `cargo build -p cityloom-design-payload`
      and `cargo clippy -p cityloom-design-payload --all-targets -- -D warnings`
      are clean, and `cargo fmt --check` passes for the new files.
- [x] Step 10: Documentation and traceability. Write `code-summary.md`,
      `source-manifest.json`, and `traceability.json` per the stage
      protocol. Update `docs/dependencies.md` with the new crate's
      single dependency (`serde`) and its licence.

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

**Applicable layers for this Unit**: "Data model / database behavior"
and "Business logic" only — this crate has no repository/data-access,
API/endpoint, or frontend layer of its own; it is a leaf data-shape
crate consumed by other Units' repository/API/frontend layers.

## Assumptions & Open Questions

None — the shape and its rules are already fully fixed by
`entities.md`, `rules.md`, and `contract-summary.md` Contract 3.
