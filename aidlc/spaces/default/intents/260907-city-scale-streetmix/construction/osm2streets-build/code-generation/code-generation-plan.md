# Code Generation Plan — `osm2streets-build` (U1)

Upstream inputs: `security-design.md` (nfr-design), `infrastructure-specification.md`,
`cicd-pipeline.md` (infrastructure-design), `unit-of-work.md` (units-generation),
`requirements.md` NFR7.2 (requirements-analysis), `team.md`/`project.md`
(practices).

## What is built

This Unit is almost entirely a **pin and its provenance**, not application
code: the Cargo git dependency declaration for `osm2streets` (pinned to a
specific commit), the same for its transitive `abstutil` dependency, the
committed provenance record, and the golden-fixture files this Unit is
responsible for authoring (asserted from `u4-street-import`, not from this
Unit's own test suite, per `unit-of-work.md`).

**Verified facts, checked live on 2026-09-16 (the plan's `tech-stack-decisions.md`
already flagged this must be checked at implementation time rather than
assumed):**
- `a-b-street/osm2streets` `main` is at commit `fc119c47dac567d030c6ce7c24a48896f58ed906`
  (2025-10-02), licensed Apache-2.0 (compatible with this project's stance —
  distinct from the AGPL Streetmix concern `project.md` Forbidden addresses;
  osm2streets was never the licence risk).
- Its own `osm2streets/Cargo.toml` carries `abstutil = { git =
  "https://github.com/a-b-street/abstreet" }` with **no `rev`/`tag`/`branch`**
  — confirming `team.md`'s prediction that the core crate has the same
  unpinned pattern `osm2streets-js` did. This is the fact `NFR7.2.2` exists
  to close.
- `a-b-street/abstreet` `main` is at commit `0964f29315820c91b171b585eb51e300164e9197`
  (2025-09-10). This is the commit `abstutil`'s own pin will target.
- The repository is a Cargo workspace; the crate this project needs is the
  `osm2streets` member (not `osm2streets-js`, `osm2streets-py`, or
  `osm2streets-java` — those are language bindings this project does not use,
  per `team.md`'s "not through `osm2streets-js`" decision).

## A genuine constraint this plan cannot execute around

**NFR7.2.3 (fork the repository into the project's own account) requires
creating a GitHub repository under a human-controlled account.** This Unit's
own build/test tooling has no GitHub write credentials and no standing to
create a repository on the project owner's behalf without their explicit
action. This plan implements everything that does **not** require that
action, and records the fork itself as a named manual follow-up (`code-summary.md`
"Deviations"), rather than silently treating a pin against `a-b-street/osm2streets`
directly as equivalent to `NFR7.2.3`. The interim state (pinned directly to
the verified upstream commit, `rev =`, no `branch`/`tag`) already satisfies
`NFR7.2.1`, `NFR7.2.2`, and `NFR7.2.4` — only the "own fork" identity and its
branch protection (`NFR7.2.3`, `NFR7.2.5`) remain a follow-up once the human
creates the fork (at that point the `Cargo.toml` `git =` URL and this Unit's
provenance record both need a one-line update to point at the fork instead
of upstream — recorded explicitly as the follow-up's exact scope).

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

None of the five generic `testable_layers` apply to this Unit — it has no
data model, no repository, no business logic, no API, and no frontend; it
is a pinned dependency plus committed fixture data. The plan below adapts
the methodology's intent (verify before declaring done) to this Unit's
actual shape: Step 1 verifies the pin's real-world facts before recording
them (the closest analogue to "Red" this Unit has — confirm the claim is
true before writing it down), Step 2 is the fixture-authoring work itself,
and Step 3 is the provenance/traceability record. Full TDD Red/Green/Refactor
does not apply to data-only, no-logic artifacts; this is the same
"omitting a genuinely inapplicable layer without changing the methodology"
allowance the stage protocol names for the Frontend-behavior layer on
`osm-extract-proxy`.

## Layout

```
Cargo.toml                        workspace member list gains no new crate
                                   (this Unit adds a dependency declaration,
                                   not a new crate, to whichever crate
                                   consumes osm2streets — recorded here as
                                   a documented pending edit for u4-street-import,
                                   since that crate does not exist yet)
docs/dependencies.md              gains the osm2streets + abstutil rows
docs/osm2streets-pin.md           the single committed provenance file:
                                   pinned commit, build command, toolchain
                                   versions, and the fork follow-up status
fixtures/osm2streets/             the committed golden-fixture set (real,
                                   small OSM extracts + their osm2streets
                                   output), authored here, asserted from U4:
  well-tagged-street.osm.xml
  thinly-tagged-street.osm.xml
  one-way-street.osm.xml
  street-with-cycleway.osm.xml
  one-intersection.osm.xml
  README.md                       what each fixture exercises and why
```

## Story-to-step traceability

This Unit serves AC3.1.4 and AC3.1.5 (the golden-fixture assertions) per
`unit-of-work.md`'s "Units that carry no story of their own" table — it has
no user story of its own.

## Steps

### Step 1 — Verify the pin targets (already done above; record it)

- [x] 1.1 Write `docs/osm2streets-pin.md`: the pinned `osm2streets` commit
  (`fc119c47dac567d030c6ce7c24a48896f58ed906`), the pinned `abstutil` commit
  (`0964f29315820c91b171b585eb51e300164e9197`), the exact build command
  (`cargo build --workspace --locked`, no separate binding step — the crate
  compiles as an ordinary workspace dependency), the toolchain versions
  (`rustc`/`cargo` 1.97.1, matching the pin `osm-extract-proxy` already
  established; the WASM build tool is still deferred to `u6-client-surfaces`'s
  framework choice per `tech-stack-decisions.md` and is recorded as `TBD` here
  rather than guessed), and a **Fork status** section stating plainly: pinned
  directly to upstream `a-b-street/osm2streets` as an interim measure;
  forking to the project's own account is a manual follow-up (not yet done);
  the follow-up is exactly a `git =` URL change plus a repointed `rev =` once
  the fork exists (the same pinned commit, on the fork).
- [x] 1.2 Add the two rows to `docs/dependencies.md` (origin, licence,
  pinned commit) for `osm2streets` (Apache-2.0) and `abstutil` (its own
  licence, checked from `a-b-street/abstreet`'s `LICENSE` file rather than
  assumed).

### Step 2 — The golden-fixture suite

- [x] 2.1 Fetch five small, real OSM extracts via the Overpass API (public,
  no credential needed), chosen to match `team.md`'s named set: a
  well-tagged street (complete lane tags), a thinly-tagged street (missing
  width/lane-count, forcing osm2streets to infer), a one-way street, a
  street with a separately-mapped cycleway, and one intersection. Save each
  as committed `.osm.xml` under `fixtures/osm2streets/`.
- [x] 2.2 Write `fixtures/osm2streets/README.md`: for each fixture, the
  real-world location (or a synthetic-but-realistic substitute if a
  suitably small real example cannot be found for one category — recorded
  explicitly if used, per `team.md`'s "real, not synthetic" preference for
  this specific suite), what it exercises, and why it was chosen.
- [x] 2.3 This Unit does **not** write the test code that runs osm2streets
  against these fixtures and asserts output — `unit-of-work.md` assigns that
  to `u4-street-import`, which does not exist yet. Record this explicitly in
  `code-summary.md` as what the next Unit touching this fixture set needs.

### Step 3 — Provenance and manifest

- [x] 3.1 Write `source-manifest.json` and `traceability.json` under this
  Unit's record directory.

## Quality targets carried in, not negotiable

Every dependency pin uses `rev =`, never `branch`/`tag` (NFR7.2.1,
NFR7.2.2). `Cargo.lock` commits once a consuming crate exists (deferred:
this Unit alone adds no workspace member, so no lockfile change is
expected yet — recorded as a gap in `code-summary.md`, not silently
skipped). The fork-identity and branch-protection requirements (NFR7.2.3,
NFR7.2.5) are explicitly NOT met yet and are named as the human follow-up,
never presented as done.
