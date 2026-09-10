# Evidence — Streetmix at City Scale

## Project Type

**Not greenfield in the sense the original draft claimed — this was the
draft's own factual error, and it is corrected here.** The original draft
said "the workspace contains no application code... nothing here was
inferred from code, because there is no code," and treated that as
justification for inspecting nothing at the workspace root beyond
`aidlc/` and `.claude/`. The developer review checked directly and found
that is false: `CLAUDE.md`, `scripts/verify.sh`, `docs/state/features.md`,
`docs/state/handoff.md`, `docs/templates/initializer-prompt.md`, and
`docs/templates/task-template.md` all exist and are the owner's own stated
conventions, which outrank framework defaults as evidence of how this
project actually works. What is genuinely missing — `justfile`,
`scripts/state-summary.sh`, `scripts/next-tasks.sh`, `docs/tasks/` — are
referenced by `CLAUDE.md` but not built, so `just verify` fails today and
`./scripts/state-summary.sh` fails at every session start. The correct
framing is: **a workspace with the owner's stated conventions and an
incomplete toolchain**, not a blank slate. Q2 resolves the incompleteness
(build the missing pieces).

There is still no product code, no git history of a product beyond this
framework, and no dependency manifest — those parts of the original claim
stand.

## What Was Inspected

- `aidlc/spaces/default/memory/org.md` — read in full; its five sections are
  the org defaults every practice below starts from.
- `aidlc/spaces/default/memory/team.md` — confirmed empty (template only).
- `aidlc/spaces/default/memory/project.md` — `## Corrections` entries from
  Ideation (foundation-checking discipline, licence-reading discipline,
  contradiction-handling discipline) read and applied throughout this
  integration; no `Way of Working` / `Testing Posture` / `Deployment` /
  `Code Style` content existed there yet.
- `initiative-brief.md`, `constraint-register.md`, `raid-log.md`,
  `scope-document.md`, `wireframes.md` (Ideation, approved) — read in full by
  the lead and cross-checked by all three reviewers.
- `CLAUDE.md`, `scripts/verify.sh`, `docs/state/`, `docs/templates/` —
  workspace-root files, read directly by the developer review (see Project
  Type above).
- `osm2streets-js/Cargo.toml` and `src/lib.rs` on `a-b-street/osm2streets`
  `main` — read directly by the lead (per the dispatch brief) and
  independently by the developer and devsecops reviews. Confirmed: the
  `abstutil` dependency is `{ git = "https://github.com/a-b-street/abstreet" }`
  with no `rev`, `tag`, or `branch`; `JsStreetNetwork`'s query methods return
  JSON strings (`toGeojsonPlain`, `toJson`, `toLanePolygonsGeojson`,
  `toLaneMarkingsGeojson`, `getGeometryForWay`, `getOsmTagsForWay`,
  `findBlock`, `findAllBlocks`); the same class exposes in-place mutation
  methods (`overwriteOsmTagsForWay`, `collapseShortRoad`,
  `collapseIntersection`, `zipSidepath`); addressing into the network is by
  positional (`usize`) index, not stable identifier.
- npm registry state for `osm2streets-js` (devsecops) — `0.1.4`, published
  2023-06-04, single maintainer account, 4 files, zero runtime dependencies,
  no npm provenance attestation.
- GitHub API state of `a-b-street/osm2streets` (devsecops) — last pushed
  2025-10-02, Apache-2.0, not archived, 34 open issues.
- `a-b-street/abstreet` LICENSE file, read directly rather than trusted from
  repository metadata (devsecops), per `project.md`'s licence-reading
  correction — confirmed Apache-2.0.
- GitHub's published Actions pricing and security-feature availability
  matrix for public vs. private repositories (all three reviews,
  independently).
- Railway's builder install behaviour — Nixpacks runs `npm ci`; Railpack may
  need `RAILPACK_INSTALL_CMD` to install from the lockfile (devsecops);
  flagged for confirmation at environment-provisioning, not resolved here.
- `practices-discovery-questions.md` — the full interview and all eight
  human answers.

## Correction 1 — the CI-cost tension was a factual error, caught independently by all three reviews

The original draft (`team-practices.md` Testing Posture, `evidence.md` "What
Could Not Be Established") presented "should CI run at all, given the
~$5/month budget?" as an open tension between org.md's "CI execution before
merge" default and `constraint-register.md` OC-4. **This was wrong, and it
was wrong for a specific, checkable reason**: it conflated hosting spend
(Railway, metered) with CI spend (GitHub Actions). GitHub Actions on standard
GitHub-hosted runners is free and unmetered for public repositories, on
every plan including Free, and this held through the January 2026 pricing
change (which added a per-minute charge for *self-hosted* runners and
explicitly exempted public-repo usage of hosted runners). OC-4 binds Railway
hosting only. With the product already committed to open source (LC-1) and
now decided as public from the first commit (Q1), CI on every merge costs
$0.

All three reviews caught this independently, without seeing each other's
work — the strongest form of evidence available at this stage. The tension
is struck rather than softened: it is removed from `team-practices.md`
Testing Posture and from this file's "What Could Not Be Established," and it
is not carried into `discovered-rules.md` in any form. The devsecops review
additionally flagged that leaving it in risked the interview answering "skip
CI to protect the budget" on a false premise, which would have removed the
only place any security control in this project can run. The lead accepts
this correction in full; there is no disagreement to record.

## Correction 2 — the Streetmix naming/module-distance rule is dropped, replaced with a checkable control

The original draft proposed, as Code Style: "naming and modules must not use
Streetmix's own naming or file layout as a starting point." Both the quality
and developer reviews independently called this unenforceable — it has no
pass/fail criterion, which the inception-phase guardrails require of every
requirement-shaped statement — and the developer review additionally
identified that it collides with `constraint-register.md` TC-1, which fixes
the street/lane model's shape as osm2streets', and osm2streets' vocabulary
(lane, width, sidewalk, cross-section) overlaps Streetmix's heavily because
both describe streets. A rule to keep naming distant from Streetmix would
push the code away from the schema it is required to mirror, putting two
approved constraints in conflict for no enforcement benefit.

**Decision: drop the rule.** It is replaced, per both reviews' converging
proposal, with:
- a **positive naming authority** — domain vocabulary follows osm2streets'
  schema terms (now in `team-practices.md` Code Style); and
- the **checkable control** the devsecops review sharpened further — Streetmix
  is never added as a dependency and never cloned into the workspace,
  verifiable by grepping the lockfile and the git remotes, plus a committed
  asset-and-dependency manifest (origin and licence for every third-party
  asset and dependency) checked by `scripts/verify.sh`, which fails the gate
  if a Streetmix package appears. This is now `discovered-rules.md`'s
  sharpened Forbidden rule and the manifest practice in `team-practices.md`
  Code Style.

Both reviews are accepted in full; this is not a case of picking one over
the other, since they converged on the same diagnosis and complementary
fixes.

## Correction 3 — the workspace was described as empty and is not

Covered under Project Type above. The developer review caught this; the
lead accepts it in full and has corrected both this file and
`team-practices.md` accordingly. Q2's answer (build the missing pieces) is
the human's resolution of the resulting contradiction.

## Affirmation-Gate Decision — client language changed to Rust compiled to WebAssembly

This decision was made by the human **at the affirmation gate, after the
interview closed** — not during `practices-discovery-questions.md`'s eight
questions, and not something any of the three support reviews (quality,
developer, devsecops) evaluated, since it did not exist yet when they ran.
It is recorded here rather than folded silently into the corrections above
because it is a genuinely new input to this stage, not a fix to something
the lead or a reviewer got wrong.

**What it changed:**
- Testing Posture's tooling: `cargo test` / `cargo-llvm-cov` replace a
  JavaScript test runner and coverage tool; the coverage denominator is
  restated in terms of Rust crates/modules in a Cargo workspace rather than
  TypeScript-shaped layers, and the earlier "WASM glue" exclusion category
  is gone because the whole client is now WASM, not a JS app with a WASM
  dependency.
- Code Style's formatter/linter: `rustfmt` and `clippy` (with `clippy`
  denying warnings in CI) replace Prettier and ESLint; the three
  inward-pointing layer boundary is now enforced by Cargo crate boundaries
  and module visibility (`pub(crate)`, workspace member separation) rather
  than ESLint's `import/no-restricted-paths` — same intent (a boundary
  violation fails automatically, with no human reviewer to catch it
  otherwise), different, and in this case stricter, mechanism (a compile
  error rather than a lint warning).
- Deployment and build: the client becomes a single Cargo workspace, and
  osm2streets is consumed as a **crate dependency directly**, not through
  `osm2streets-js`. Q6's decision (build from source, pinned) still applies,
  now realised as a Cargo git-dependency pin (`rev = "<sha>"`) rather than a
  pin inside a separate WASM-binding package.
- The `npm audit`-as-a-gate rejection (devsecops review, accepted above)
  does not carry over as a rejection: `cargo audit` against RustSec is
  assessed fresh in Deployment and Code Style above and lands on the
  opposite side of the "produces a queue nobody triages" test, because
  RustSec is a smaller, curated database against a shallower dependency
  tree than a typical JavaScript frontend's.

**What it did NOT change:** all eight interview answers stand exactly as
answered (Q1 public-from-commit, Q2 build the missing tooling, Q3 B-0's
pass/fail criterion, Q4 tests-first/TDD, Q5 the editing-surface choice
deferred to Domain Design, Q6 build-from-source-pinned, Q7 one environment
deploy-on-merge, Q8 the access gate ahead of data subject rights). The
`Methodology: tdd` field and its fixture-before-adapter ordering are
unchanged — tests-first works the same way in Rust as in TypeScript, and
nothing about the language choice touches it. The WCAG 2.1 AA commitment
survives without qualification: Rust/WASM does not force a canvas-rendered
editing surface. Leptos, Yew, and Dioxus all render real DOM — Leptos
through fine-grained reactivity with no virtual DOM, Yew and Dioxus through
virtual-DOM diffing — so Q5's deferred choice stays exactly as live as it
was, and `team-practices.md`'s open-constraint note on Domain Design is
restated (not weakened) to say so explicitly: a DOM-rendering Rust
framework preserves automated accessibility verification of the editing
surface; a canvas approach, in any language, does not. The devsecops
review's secret-scanning, push-protection, `SECURITY.md`, and
Dependabot-security-only package is unaffected — none of it names a
language, and it stands as originally accepted.

**A genuine benefit this decision buys, that neither the reviews nor the
lead identified before the human made the call:** the developer review's
concern about the `osm2streets-js` JSON-string API boundary — that
`JsStreetNetwork`'s query methods return JSON strings which must be
parsed and validated at the adapter, and that the adapter is the one place
permitted to hold a WASM handle — is **resolved by this decision, not
merely mitigated**. With osm2streets consumed as a native Rust crate
dependency inside the same Cargo workspace, there is no JS/WASM boundary
between the client and osm2streets at all; both compile into the same
binary. The adapter still exists as a deliberate module boundary (parse a
native osm2streets struct into this project's own provenance-carrying
types, never re-export an osm2streets type past it), but it is no longer
doing string-parsing at a runtime boundary — the type safety the JSON
string API could never offer is available at compile time instead. This is
a strictly better position than anything proposed in response to the
original review's finding, and it exists only because of a decision made
after that review ran.

## Affirmation-Gate Addition — SonarQube Cloud, directed by the human after the devsecops review rejected a second SAST product

The human asked for SonarQube Cloud directly at the affirmation gate, on this
second revision turn — after the devsecops support review had already run
and after that review's stated position on a second SAST product. This is
recorded as a directed addition, not folded silently into the devsecops
review's own recommendations, because the review did not propose it and the
lead is not attributing it retroactively.

**The devsecops review's original position, unchanged and not softened
here:** it explicitly rejected adding "a second SAST product," and set the
organising rule now stated in `team-practices.md` Deployment — every security
control here either blocks at the moment of the mistake with a specific
actionable message, or it is off; nothing produces a dashboard, a periodic
report, or a queue for one person to work through. That rule stands exactly
as the review wrote it. The review was right to reject a second SAST product
shaped as a dashboard nobody would visit; nothing below reopens that
rejection or claims the review would have agreed to what follows.

**The reconciliation:** what the human asked for, and what is now specified,
is not the thing the review rejected. Configured as a blocking quality gate
on new code (Sonar's "Clean as You Code" mode) rather than a dashboard to
browse, SonarQube Cloud lands on the *blocking* side of the review's own
test — a failing gate stops the squash-merge with a specific finding
attached, the same shape as `cargo clippy -D warnings` or `cargo audit`
already blocking above it, not a report that accumulates unread. And it is
free rather than a cost the review's solo-builder-budget reasoning was
guarding against: SonarQube Cloud has no line-of-code cap and no expiry on
public repositories (the private-project 50k-LOC cap does not apply), and
this repository has been public from the first commit since Q1. Verified
live for this revision rather than recalled, per `project.md`'s
foundation-checking correction:
`https://docs.sonarsource.com/sonarqube-cloud/analyzing-source-code/languages/rust`,
`https://www.sonarsource.com/blog/introducing-rust-in-sonarqube/`,
`https://www.sonarsource.com/plans-and-pricing/`.

**The residual tension, stated rather than smoothed away:** this is still,
literally, a second static-analysis product in the pipeline, and the
devsecops review's instinct that a solo builder should be wary of stacking
tools is not wrong in general — it is specifically answered here, not
disproved. The Sonar Rust analyzer is built around Clippy, so its core
overlaps a control this project already runs and already blocks on
(`cargo clippy -D warnings`); a real share of whatever Sonar would flag on
this codebase is a lint Clippy would have caught first. And it adds a CI
step with real latency — a CI-based Rust scan, not a configuration toggle,
because Rust is not supported by SonarQube Cloud's Automatic Analysis mode
— paid on every PR to `main` in the slow tier, which is why it is tiered
there rather than run on every push. Being free and gate-shaped answers the
review's stated objection; it does not make the tool cost-free in the
non-monetary sense the review was also implicitly weighing — one more
product surface for a solo builder to keep configured, and one more thing
that can produce a false-positive block at 11pm with no second person to
argue about it. This is recorded as a deliberate, human-directed exception
to the review's general posture, not as evidence the posture was wrong.

## Other places a review objected and the lead accepted

- **Methodology in `discovered-rules.md` / `team-practices.md`**: the quality
  review objected to carrying `test-after` forward unexamined and proposed
  `custom` (test-first for the model/state/persistence layers, spike-then-
  characterise at the osm2streets boundary, implement-then-test for UI). The
  interview put this to the human as three options; **the human chose Q4
  option C — tests first throughout — which is stricter than the quality
  review's proposed mixed approach.** `Methodology: tdd` is recorded as
  specified, with the ordering written as one explicit sentence that
  includes the osm2streets boundary: the test is written against a
  documented expected output (the committed fixture) before the adapter code
  that produces it. This is the human's explicit, informed override of a
  reviewer's more nuanced recommendation — recorded here rather than
  smoothed into something closer to the review's original proposal. The
  quality review's underlying concern (you cannot write a meaningful test for
  osm2streets output you have not yet seen) is real and is addressed by the
  fixture-first ordering rather than by reverting to spike-first, which
  would have contradicted the human's answer.
- **Accessibility check design**: the quality review's finding that axe-core
  cannot see inside a `<canvas>` element, making the original draft's
  proposed check produce false assurance, is accepted in full and now shapes
  `team-practices.md` Testing Posture (axe as floor, keyboard-path tests as
  the real per-action check, defined manual pass).
- **Adapter design**: the developer review's finding that the osm2streets
  boundary is a JSON-string boundary (not merely an object boundary) is
  accepted and makes the adapter proposal concrete — parse-and-validate,
  single importer, provenance attached at that boundary.
- **Provenance as a type, not a rendering decision**: the developer review's
  argument that `TC-3`/R-2's provenance requirement will not survive as a
  rendering-time decoration is accepted; `team-practices.md` Code Style now
  states the `Dimension`/`Provenance` type constraint.
- **Layer boundaries**: the developer review's three inward-pointing zones,
  enforced by `import/no-restricted-paths`, are accepted as stated, with the
  clarification (which the review itself made) that this is a dependency
  boundary, not a rejection of feature-first organisation inside the outer
  ring.
- **Supply-chain silence**: the devsecops review's finding that the original
  draft said nothing about secret handling, dependency intake, or the build
  path is accepted; `team-practices.md` Deployment now carries push
  protection, `gitleaks`, `SECURITY.md`, Dependabot posture, and the
  fork-and-pin practice for the osm2streets build.
- **DAST**: the devsecops review's rejection of DAST (no safe environment to
  scan, given single-environment deployment) in favour of deterministic
  security-header and object-level-authorization tests in the normal suite
  is accepted — a security specialist arguing against a control their own
  default posture would recommend is stronger evidence than either the lead
  or a generic checklist would produce alone.
- **CodeQL timing**: the devsecops review's recommendation to enable CodeQL
  at the start of Stage 2, tied to the existing public-release gate rather
  than left as "do it sometime," is accepted as stated.

## Where a review's proposal was not promoted, and why

- The lead held the line, endorsed explicitly by all three reviews, that a
  mandate requires a stated constraint with a named consequence — not merely
  a good idea. This kept the majority of all three reviews' recommendations
  (the adapter pattern, the provenance type, layer boundaries, CodeQL timing,
  the DAST rejection, Dependabot posture, lint plugin choices) in
  `team-practices.md` as practice rather than in `discovered-rules.md` as
  mandate, consistent with the devsecops review's own stated discipline (two
  of its twelve recommendations met the bar).
- The devsecops review proposed a conditional Forbidden rule on secrets
  ("conditional on the visibility answer" from what was then an open
  question). Q1 has since settled the repository as public from the first
  commit, so the rule is recorded in `discovered-rules.md` unconditionally
  rather than with the hedge the review proposed, since the condition it was
  hedging against no longer applies.

## Open Item — carried to Domain Design, not resolved here

Q5 explicitly defers whether the editing surface is built from real page
elements or drawn on a raster canvas to Domain Design. This interacts
directly with the tests-first posture (Q4) and the WCAG 2.1 AA commitment:
if Domain Design selects canvas, automated accessibility verification of the
editing surface is not available (axe-core cannot inspect canvas contents),
and conformance can only be checked by the manual screen-reader pass, every
release, indefinitely. This item survives the Rust/WebAssembly client
decision unchanged in substance and, if anything, more load-bearing: see
the Affirmation-Gate Decision section above — Rust/WASM does not force
canvas, so Q5 is now also a framework-selection question (Leptos/Yew/Dioxus
all render real DOM) as well as a rendering-technique one.
`team-practices.md` Testing Posture records this as a constraint on Domain
Design's choice, not as a decision made here.

## Open Item — carried to Requirements Analysis, not resolved here

The quality review's point that the "week to under a day" workflow-time
measure (`initiative-brief.md`) must not become an acceptance criterion
until a real baseline exists is accepted and carried forward rather than
decided in this stage. `raid-log.md` R-4 already records it as unvalidated,
with no baseline and no user timed. `team-practices.md` Testing Posture
notes it as a hypothesis; Requirements Analysis is the stage that must not
invent a threshold nobody has measured.

## What Could Not Be Established

- Which Test Strategy level applies to which future Bolt beyond what this
  stage settles for `feature` scope generally.
- Whether the Railway builder in use is Nixpacks or Railpack, which decides
  whether the committed lockfile is actually honoured at deploy —
  confirmed at environment-provisioning.
- The editing-surface choice (Q5) and its downstream effect on whether WCAG
  2.1 AA is automatically verifiable — Domain Design.
- A real baseline for the "week to under a day" workflow-time claim —
  Requirements Analysis.

## Sources

- `aidlc/spaces/default/memory/org.md`, `team.md`, `project.md`
- `initiative-brief.md`, `constraint-register.md`, `raid-log.md`,
  `scope-document.md`, `wireframes.md`
- `CLAUDE.md`, `scripts/verify.sh`, `docs/state/`, `docs/templates/`
- `osm2streets-js/Cargo.toml`, `osm2streets-js/src/lib.rs` on
  `a-b-street/osm2streets` `main`
- npm registry (`osm2streets-js`), GitHub API (`a-b-street/osm2streets`,
  `a-b-street/abstreet`), GitHub Actions pricing and security-feature
  documentation
- `practices-discovery-questions.md`
- `contributions/aidlc-quality-agent.md`, `aidlc-developer-agent.md`,
  `aidlc-devsecops-agent.md`
- Rust tooling facts checked live for this revision, per `project.md`'s
  foundation-checking correction, rather than recalled: `cargo-llvm-cov` is
  the current standard LLVM-based coverage tool for Rust and supports CI
  threshold enforcement; `cargo clippy ... -- -D warnings` and
  `cargo fmt --check` are the standard 2026 CI pattern; Leptos renders via
  fine-grained reactivity with no virtual DOM, while Yew and Dioxus both
  render real DOM via virtual-DOM diffing (none of the three requires
  canvas); `cargo audit` checks `Cargo.lock` against the RustSec advisory
  database, a curated database maintained by the RustSec project.
- SonarQube Cloud facts checked live for this revision, per `project.md`'s
  foundation-checking correction, rather than recalled: Rust is fully
  supported since April 2025 and the Sonar Rust analyzer is built around
  Clippy; SonarQube Cloud is free for public repositories with no line-of-code
  cap and no expiry (the 50k-LOC cap applies to private projects only); Rust
  is not supported by SonarQube Cloud's Automatic Analysis and requires a
  CI-based scan with Cargo and a Rust toolchain including Clippy.
  (`https://docs.sonarsource.com/sonarqube-cloud/analyzing-source-code/languages/rust`,
  `https://www.sonarsource.com/blog/introducing-rust-in-sonarqube/`,
  `https://www.sonarsource.com/plans-and-pricing/`)
