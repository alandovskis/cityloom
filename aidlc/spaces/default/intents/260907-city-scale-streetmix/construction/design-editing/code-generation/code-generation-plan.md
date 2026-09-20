# Code Generation Plan — `design-editing` (U5)

Upstream inputs: `entities.md`, `rules.md`, `functional-spec.md`
(functional-design, this Unit); `security-design.md`, `logical-components.md`
(nfr-design, this Unit); `tech-stack-decisions.md` (nfr-requirements, this
Unit); `cicd-pipeline.md` (infrastructure-design, this Unit);
`components.md` (`DesignOverlay`, `EditingSession`, `CorridorPlanner`);
`unit-of-work.md` U5.

This Unit is a new workspace crate, `cityloom-design-editing`, compiled to
`wasm32-unknown-unknown`. It owns three components: `DesignOverlay`
(proposed lane edits, additions, removals; revert semantics),
`EditingSession` (selection + undo state machine), and `CorridorPlanner`
(connectivity, fit assessment, correspondence, bulk apply). Applicable
Testing Contract layers: "Data model / database behavior" (the entity
types), "Business logic" (the three components' actual behaviour). No
repository/data-access (no I/O), API/endpoint, or frontend layer applies.

## Story/AC traceability

| Plan step | Story / AC | Rule |
|---|---|---|
| Steps 3-5 | AC5.2.1, AC5.2.2, AC5.2.3, AC5.2.4 | BR4.1 |
| Steps 6-8 | AC5.1.2, AC5.1.3, AC5.1.4, AC5.1.5, AC5.4.1, AC5.4.2, AC5.4.3 | BR1.1, BR2.1, BR3.1, BR5.1 |
| Steps 6-8 | AC5.5.1, AC5.5.2, AC5.5.3 | BR6.1 |
| Steps 9-11 | AC6.1.1, AC6.1.2, AC6.1.3, AC6.1.4, AC6.4.1 | BR7.1 |
| Steps 9-11 | AC6.2.1, AC6.2.2, AC6.2.3, AC6.2.4, AC6.2.5, AC6.2.6 | BR8.1, BR9.1 |
| Steps 9-11 | AC6.3.1, AC6.3.2, AC6.3.3 | BR11.1 |
| Steps 9-11 | AC6.4.2, AC6.4.3, AC6.4.4 | BR10.1 |
| All | NFR6.4.1, NFR6.3.1, NFR6.1.1 | security-design.md SD-1 through SD-3 |

## Steps

- [x] Step 1: Project structure and production configuration skeleton.
      Create `crates/cityloom-design-editing/` as a new workspace member:
      `Cargo.toml` (name `cityloom-design-editing`; dependencies:
      `street-core` and `cityloom-street-import` (both workspace paths);
      no new third-party dependency per `tech-stack-decisions.md`),
      `src/lib.rs`.
- [x] Step 2: Bootstrap the minimal test runner/configuration and record
      the exact unit-scoped command. `cargo test -p cityloom-design-editing`
      is the unit-scoped command. Record it in `unit-test-instructions.md`.
- [x] Step 3: Data model / database behavior — Red: write the failing
      tests for `Design`, `LaneEdit`, `EditOutcome`, `EditFinding`,
      `EditSession`, `UndoEntry`, `CorridorSelection`, `FitState`,
      `CorrespondenceResult`, `CorridorApplyOutcome` (per `entities.md`):
      construction, and that an added `LaneEdit` carries an `anchor` and
      no `target_lane_discriminator` while a change/removal carries a
      `target_lane_discriminator` and no `anchor` (BR4.1). Record the
      failing command output.
- [x] Step 4: Data model / database behavior — Green: implement the ten
      types. Pass Step 3's tests.
- [x] Step 5: Data model / database behavior — Refactor: tidy while
      green.
- [x] Step 6: Business logic — Red: write the failing tests for
      `DesignOverlay` and `EditingSession`: width validation rejecting
      `<= 0` or `> 20.0` metres with the previous value retained (BR2.1);
      a width change setting UserSet provenance (BR3.1); revert
      returning the pre-edit value with that value's own provenance,
      resolving through `street-import`'s `CorrectionOverlay` where a
      correction exists (BR3.1); the three-layer delta report, including
      the "nothing has changed" case (BR5.1); undo reversing the most
      recent action synchronously, a corridor-apply undo reversing every
      affected street as one unit, and undo-with-nothing-to-undo
      reporting rather than failing (BR6.1); and that every operation
      returns synchronously with no `.await` point anywhere in the call
      path (BR1.1 — assert this by construction: no method signature in
      this Unit's public API returns a `Future` or is declared `async`).
      Record the failing command output.
- [x] Step 7: Business logic — Green: implement `DesignOverlay` and
      `EditingSession`. Pass Step 6's tests.
- [x] Step 8: Business logic — Refactor: tidy while green.
- [x] Step 9: Business logic — Red: write the failing tests for
      `CorridorPlanner`: the connected-street set computed from
      `street-core`'s `StreetNetworkGraph` adjacency, never a coarse
      pass, with a stable human-readable fallback identifier for an
      unnamed street (BR11.1); the correspondence rule matching by
      `(lane_type, ordinal_from_kerb)`, leaving unmatched lanes
      untouched and named, and leaving the target's own corrections
      intact (BR7.1); the four fit states — `fits`, `does_not_fit` (with
      shortfall), `could_not_be_checked` (wholly `Inferred` or absent
      carriageway width), `not_yet_checked` — using total width only,
      no lane-type compatibility rule (BR8.1); applying past a
      `does_not_fit` warning without scaling any width (BR9.1); and a
      bulk apply where one target street fails preparation, confirming
      the other target streets still succeed and the failure is named
      with a reason, never rolled back (BR10.1). Record the failing
      command output.
- [x] Step 10: Business logic — Green: implement `CorridorPlanner`. Pass
      Step 9's tests.
- [x] Step 11: Business logic — Refactor: tidy while green.
- [x] Step 12: Environment/build configuration. Confirm
      `cargo build -p cityloom-design-editing`,
      `cargo clippy -p cityloom-design-editing --all-targets -- -D warnings`,
      and `cargo fmt --check` are clean. Confirm the undo stack and
      corridor target-street set have a stated bounded capacity in code
      (SD-1) — this is a code-level assertion (e.g. a documented
      constant plus a test asserting the bound is enforced), not an
      infrastructure check.
- [ ] Step 13: Documentation and traceability. Write `code-summary.md`,
      `source-manifest.json`, and `traceability.json`. Update
      `docs/dependencies.md` (no new third-party dependency to add, but
      confirm the new crate itself is listed if `docs/dependencies.md`
      tracks internal workspace crates too — follow the existing file's
      convention).

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

**Applicable layers for this Unit**: "Data model / database behavior" and
"Business logic" only — this crate has no repository/data-access
(no I/O at all), API/endpoint, or frontend layer of its own.

## Assumptions & Open Questions

None — the shape and rules are already fully fixed by `entities.md`,
`rules.md`, `functional-spec.md`, and `security-design.md`.
