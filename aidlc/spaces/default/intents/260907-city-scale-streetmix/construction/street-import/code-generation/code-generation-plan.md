# Code Generation Plan — `street-import` (U4)

Upstream inputs: `entities.md`, `rules.md`, `functional-spec.md`
(functional-design, this Unit); `security-design.md`, `logical-components.md`
(nfr-design, this Unit); `tech-stack-decisions.md` (nfr-requirements, this
Unit); `cicd-pipeline.md` (infrastructure-design, this Unit);
`components.md` (`ExtractFetcher`, `StreetImportAdapter`,
`CorrectionOverlay`); `contract-summary.md` Contract 1 and Contract 8;
`unit-of-work.md` U4.

This Unit is a new workspace crate, `cityloom-street-import`, compiled to
`wasm32-unknown-unknown` as part of the client. It owns three components:
`ExtractFetcher` (fetch + classify), `StreetImportAdapter` (the only
component permitted to depend on `osm2streets`; converts native output
into `street-core`'s types, assigning provenance), and `CorrectionOverlay`
(correction storage and re-import reconciliation). Applicable Testing
Contract layers: "Data model / database behavior" (the entity types),
"Repository / data access" (`ExtractFetcher`'s fetch, the closest analogue
to a data-access boundary in this Unit), and "Business logic"
(`StreetImportAdapter`'s conversion and `CorrectionOverlay`'s
reconciliation). No API/endpoint or frontend layer applies.

## Story/AC traceability

| Plan step | Story / AC | Rule |
|---|---|---|
| Steps 3-5 | AC7.1.1, AC7.1.2, AC7.4.1, AC7.4.3 | BR3.1 |
| Steps 3-5 | AC7.5.1 | BR8.1 (ImportFingerprint, CorrectionReconciliationOutcome shapes) |
| Steps 6-8 | AC3.1.6, AC7.2.1, AC7.2.2 | BR1.1, BR2.1 |
| Steps 9-11 | AC3.1.1, AC3.1.4 (fixture-asserted), AC3.1.5 (fixture-asserted) | BR4.1 |
| Steps 9-11 | AC7.3.1, AC7.3.2, AC7.3.3, AC7.3.4, AC7.3.5 | BR5.1, BR6.1, BR7.1 |
| Steps 9-11 | AC7.5.2, AC7.5.3, AC7.5.4 | BR8.1 |
| Steps 9-11 | AC7.2.3, AC7.2.4 | BR9.1 (boundary only — see functional-spec.md) |
| All | NFR6.4.1-6.4.4, NFR6.1.3 | security-design.md SD-1 through SD-5 |

## Steps

- [x] Step 1: Project structure and production configuration skeleton.
      Create `crates/cityloom-street-import/` as a new workspace member:
      `Cargo.toml` (name `cityloom-street-import`; dependencies:
      `osm2streets` via the existing workspace-pinned reference from
      `u1-osm2streets-build`, `cityloom-street-core` (workspace path),
      `cityloom-api-types` (workspace path, from `osm-extract-proxy`),
      `serde`/`serde_json` (reused workspace versions), `gloo-net` for
      browser fetch, `gloo-timers` for the timeout budget, `thiserror` for
      error ergonomics), `src/lib.rs`. Add the crate to the workspace root
      `Cargo.toml`'s `members` list (already `["crates/*"]` — likely no
      edit needed, confirm by inspection).
- [x] Step 2: Bootstrap the minimal test runner/configuration and record
      the exact unit-scoped command. `cargo test -p cityloom-street-import`
      is the unit-scoped command. Record it in `unit-test-instructions.md`.
      Confirm the fixture files at `fixtures/osm2streets/*.osm.xml` are
      readable from this crate's test code via a workspace-relative path
      (e.g. `concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/osm2streets/...")`).
- [x] Step 3: Data model / database behavior — Red: write the failing
      tests for `ImportFailure`, `Correction`, `ImportFingerprint`, and
      `CorrectionReconciliationOutcome` (per `entities.md`): construction,
      the `retryable` flag matching Contract 1's retryable-classes table
      (BR2.1), and that `Correction` carries no bare `provenance` field of
      its own (structural — its existence as a `Correction` is what
      asserts UserSet, per BR5.1). Record the failing command output.
- [x] Step 4: Data model / database behavior — Green: implement the four
      types plus the `FailureReason` re-export/wrap from
      `cityloom-api-types`. Pass Step 3's tests.
- [x] Step 5: Data model / database behavior — Refactor: tidy while green.
- [x] Step 6: Repository / data access — Red: write the failing tests for
      `ExtractFetcher` — issuing the `GET /api/extract` call (Contract 1)
      via `gloo-net`, applying the timeout budget via `gloo-timers` (BR1.1),
      classifying transport-level outcomes into `ImportFailure` with the
      correct `retryable` flag (BR2.1), and retry succeeding returns the
      imported result, not a blank one (AC7.2.2). Use a mockable transport
      (a trait `ExtractTransport` that `ExtractFetcher` is generic over, or
      an injectable `gloo-net` request builder) so these tests do not
      require a live network call. Record the failing command output.
- [x] Step 7: Repository / data access — Green: implement `ExtractFetcher`.
      Pass Step 6's tests.
- [x] Step 8: Repository / data access — Refactor: tidy while green.
- [x] Step 9: Business logic — Red: write the failing tests for
      `StreetImportAdapter` and `CorrectionOverlay`:
      - Against each of the five committed fixtures
        (`fixtures/osm2streets/*.osm.xml`): `well-tagged-street` produces
        lanes entirely `Mapped` provenance (BR4.1, AC3.1.1); `thinly-tagged-street`
        produces at least one `Inferred` value (BR4.1, `team.md`'s named
        provenance-test floor); `one-way-street` attributes all lanes to a
        single direction; `street-with-cycleway` does not conflate the
        road and the separately-mapped cycleway way; `one-intersection`
        produces a `StreetNetworkGraph` connecting the split segments.
        Lane count/order exactness against `well-tagged-street` and the
        raw-extract fixture is the AC3.1.4/AC3.1.5 assertion — read
        `cicd-pipeline.md`'s tiering note: `well-tagged-street.osm.xml`'s
        assertion runs in the fast tier, and if a genuinely release-mode-only
        assertion is needed for AC3.1.5, gate it behind `#[cfg(not(debug_assertions))]`
        or an explicit slow-tier test filter — do not skip it outright.
      - A synthetic malformed-XML byte sequence (not a committed fixture —
        constructed inline) triggers `ImportFailure::malformed_extract`
        without panicking past the adapter boundary (SD-1, catch_unwind).
      - `CorrectionOverlay`: construct a `Correction` (BR5.1, BR6.1's
        keying), and the three re-import reconciliation outcomes (BR8.1,
        AC7.5.2-AC7.5.4) using two synthetic `ImportFingerprint` values
        (matching and differing) rather than requiring two real fixture
        variants.
      Record the failing command output.
- [x] Step 10: Business logic — Green: implement `StreetImportAdapter`
      (wrapping every osm2streets call in `catch_unwind` per SD-1) and
      `CorrectionOverlay`. Pass Step 9's tests.
- [x] Step 11: Business logic — Refactor: tidy while green.
- [x] Step 12: Environment/build configuration. Confirm
      `cargo build -p cityloom-street-import`,
      `cargo clippy -p cityloom-street-import --all-targets -- -D warnings`,
      and `cargo fmt --check` are clean. Confirm exactly one resolved
      `osm2streets` package in the dependency graph (SD-3) via
      `cargo tree -p cityloom-street-import -i osm2streets` or equivalent.
- [x] Step 13: Documentation and traceability. Write `code-summary.md`,
      `source-manifest.json`, and `traceability.json`. Update
      `docs/dependencies.md` with the new crate's dependencies
      (`gloo-net`, `gloo-timers`, `thiserror`, plus the existing
      `osm2streets`/`cityloom-street-core`/`cityloom-api-types` reused
      references) and their licences.

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

**Applicable layers for this Unit**: "Data model / database behavior",
"Repository / data access", and "Business logic" — this crate has no
API/endpoint or frontend layer of its own.

## Assumptions & Open Questions

- **[assumption]** `osm2streets`'s actual public API shape (function
  names, struct fields for lane specs and provenance signals) is not
  fully known until the developer agent reads the pinned crate's source
  directly (via the `u1-osm2streets-build` pin) — `entities.md` and this
  plan describe the required project-owned output shape and the required
  behaviour, not the exact osm2streets call sequence, which the developer
  agent resolves against the real crate at Green time (consistent with
  `project.md`'s "read the source, not the summary" practice).
- **[assumption]** AC3.1.5's release-mode-only assertion may need a slow-tier
  test filter mechanism the workspace does not yet have; if so, the
  developer agent documents this as a genuine gap in `code-summary.md`
  rather than silently running it in the fast tier or skipping it.
