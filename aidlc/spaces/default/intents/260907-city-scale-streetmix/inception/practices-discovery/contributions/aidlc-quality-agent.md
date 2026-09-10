**Collaborator:** aidlc-quality-agent

## Contribution

Read: the three lead drafts, `org.md`, `project.md`, `initiative-brief.md`,
`constraint-register.md`, `raid-log.md`, `scope-document.md`, and
`aidlc-state.md` (scope `feature`, Depth Standard, Test Strategy Standard).
Two external facts below were checked against live sources rather than
reasoned from reputation, per `project.md` `## Corrections`.

The draft is honest about being a greenfield draft and it separates
`[org default]` / `[ideation]` / `[proposed]` cleanly — that provenance
marking is worth keeping through integration. My substantive changes are
below, ordered by how much they change the practice.

### 1. The CI-cost tension is not real. Resolve it; do not take it to the interview as posed.

`evidence.md` lists "whether CI should run at all given the $5/month
constraint" as an open tension against OC-4, and `team-practices.md` repeats
it under Testing Posture. Checked: **GitHub Actions on standard
GitHub-hosted runners is free and unmetered for public repositories**, and
that remained true through the January 2026 pricing change (which cut hosted
runner rates and added a per-minute platform charge for *self-hosted*
runners; public-repo runner usage stayed free). `LC-1` already commits this
product to being open source. On a public repo, CI-before-merge costs **$0**
against OC-4. Even in the private case, GitHub Free gives 2,000 Linux
minutes/month, which a solo builder's push volume does not approach.

- Sources: https://github.blog/changelog/2025-12-16-coming-soon-simpler-pricing-and-a-better-experience-for-github-actions/ and https://github.com/resources/insights/2026-pricing-changes-for-github-actions

**Change to make:** drop the CI-affordability tension from
`team-practices.md` Testing Posture and from `evidence.md`'s "could not be
established". Keep org.md's "CI execution before merge" unchanged. Replace
the interview question with a much narrower one that actually settles it:
*"Is the repository public from day one?"* — that single answer decides CI
cost, and `LC-1` already points at yes.

The cost that *is* real is wall-clock, not dollars: the WASM build step
(`TC-4`), and browser-based tests, which run 2-4x slower than jsdom. Handle
it with tiering, not by dropping CI (see §3).

### 2. The proposed accessibility check will produce false assurance as written. This is the most important correction in this contribution.

`team-practices.md` proposes "at least one automated accessibility check
(e.g. axe-core or equivalent)" per Bolt touching the editing canvas. Checked:
**axe-core treats a `<canvas>` element as an opaque node and performs no
accessibility checks on its contents** — it can only assess DOM *outside* the
canvas. Automated tooling catches roughly 57% of WCAG issues at best across a
normal page; on a raster-rendered editing surface the fraction of the
*editing* surface it covers is approximately zero. A green axe run on a
canvas editor means nothing about the editor.

- Source: https://github.com/bokeh/bokeh/discussions/14057 (axe-core maintainer-adjacent discussion of exactly this limitation); tooling context: https://playwright.dev/docs/accessibility-testing

So the practice has to be stated in two halves, and the first half is a
**design constraint, not a test**:

**(a) Testability constraint to carry into Domain Design and Functional
Design.** The *interactive* editing surface — the lanes a user selects and
changes — must be real focusable DOM/SVG elements carrying accessible name,
role and state. The basemap may be canvas/WebGL (it is a view, not an
editing target); the lane objects the user acts on must not be. This is not
an extra requirement bolted on: it falls straight out of the brief's own
interaction principle. Select-then-act is only *implementable* for a keyboard
and a screen reader if there is something to focus and something to announce.
Writing this down now, before Domain Design, costs nothing; discovering it
after a canvas renderer exists costs a rewrite of the editor. If raster
canvas proves unavoidable for performance, the rule becomes: maintain a
synchronised parallel DOM tree of the same objects as the accessible
interface, and test that tree.

**(b) What the test tier then is.** axe-core (driven from Playwright) is the
*floor*, not the check — it earns its place on contrast, names, roles and
landmarks in the surrounding UI. The per-action check is a **keyboard-only
interaction test**: for every editing action (select a lane, change its type,
change its width, extend along the corridor, undo), a test that drives it
with keys alone from a fresh load and asserts both the resulting model state
and the announced accessible name/state. Plus a **defined manual pass** — a
screen-reader walkthrough of the full edit path — before each stage's
release. Say explicitly in `team-practices.md` that WCAG 2.1 AA is *not*
fully CI-verifiable, so a later stage does not read a green pipeline as
conformance.

Three WCAG 2.1 AA criteria bite specifically here and are worth naming as
testable, because they constrain Stage 1's visual-identity work as much as
the editor: **2.1.1 Keyboard (A)**; **2.5.1 Pointer Gestures (A)** — no
path-based or multipoint gesture may be the only route to any function, which
the select-then-act principle already satisfies; **1.4.11 Non-text Contrast
(AA)** — applies to lane graphics, selection and focus indicators, i.e. the
core visual language of the editor; and **1.4.10 Reflow (AA)** — 320 CSS px
without two-axis scrolling, which is the real test of the map+drawer phone
layout. Worth recording that **2.5.7 Dragging Movements is WCAG 2.2 AA, not
2.1**, so it is outside the stated commitment — but the select-then-act
principle meets it incidentally, which is a free claim the project can make.
Verified: https://wcag22aa.org/new-criteria/dragging-movements/ and
https://dequeuniversity.com/resources/wcag2.1/2.5.1-pointer-gestures

### 3. Name the merge gate concretely — it is the substitute reviewer.

`team-practices.md` Way of Working says review means "the AI support-agent
passes plus the owner's own approval at the gate". Being explicit that no
second human exists is right and I endorse it. But as a *quality control*
that sentence is weaker than it reads, because advisory agent prose cannot
fail a merge. For a solo builder the only thing that can fail a merge is the
automated gate, so the practice must name it. Proposed Testing Posture
bullet, stated as a list so `build-and-test` can check it:

> **Merge gate — all must pass in CI before squash-merge to `main`:**
> 1. `tsc --noEmit` clean under TypeScript `strict`
> 2. ESLint clean at `--max-warnings 0` (a solo builder with a warning
>    backlog has no lint gate)
> 3. Prettier `--check` clean
> 4. unit + component tests green
> 5. coverage floor met on the measured set (§5)
> 6. osm2streets adapter golden-fixture suite green (§4)
> 7. keyboard-path + axe tests green for any Bolt touching the editor

**Two CI tiers**, so this stays affordable in wall-clock:
- *Fast tier, every push, gates the merge*: 1-6 above (typecheck, lint,
  format, unit/jsdom component tests, coverage, fixtures).
- *Slow tier, on PR to `main` and nightly*: 7 (browser-mode/Playwright a11y
  and keyboard paths) plus the WASM build from source.
- Build the WASM artifact **once**, cache it by checksum in CI, and restore
  rather than rebuild per push. `R-1` says the published `osm2streets-js`
  packages are old against a much newer repository (npm shows 0.1.4, last
  published roughly a year ago, API declared unstable —
  https://libraries.io/npm/osm2streets-js), so building from source is the
  likely case and a Rust+wasm-pack build on every push would make the loop
  unusable even though it is free.

**TypeScript `strict` from the first commit** belongs in Code Style or here.
It is the cheapest quality control available on an empty repo and expensive
to retrofit. Add one rule specific to `TC-4`: every value crossing the WASM
boundary is **parsed and validated at the adapter**, never admitted by an
`as` cast. Casts on osm2streets output are precisely where bugs will hide,
because the types are hand-written for a compiled artifact nobody here
controls.

### 4. Endorse the adapter proposal, and sharpen what its tests must do.

The osm2streets adapter proposal is the strongest thing in the draft and I
endorse it without reservation. "Wrap it and give it a test suite"
understates the job, though. Two distinct duties:

**(a) Golden fixtures pin the dependency's behaviour.** Commit a small set of
real OSM extracts — one well-tagged street, one thinly-tagged street, a
one-way, a street with a separately-mapped cycleway, one intersection — plus
the osm2streets output they currently produce, and assert the adapter's
normalised output against them. This is a *characterisation* suite: its job
is to tell the builder the day the dependency's behaviour moves underneath
them (`R-3` unmaintained, a fork, or a rebuild at new toolchain versions),
not to prove osm2streets correct. Given `D-1` is Critical with no fallback
but forking, this is the single highest-value test asset in the project.
Fixtures are **committed**, never fetched from the OSM API at test time —
otherwise the suite is non-deterministic, network-dependent, and impolite to
a free public API.

**(b) Test the inferred-vs-mapped distinction, or `R-2`'s mitigation is only
an intention.** `R-2` is High-likelihood: the tool may produce
confident-looking designs resting on defaults. Its stated treatment is "make
derived values visibly distinguishable from mapped ones in the product".
Proposed Testing Posture bullet: **every cross-section value carries a
provenance flag (`mapped` | `inferred` | `user-set`), and the test floor for
any Bolt touching import or the editor includes one test asserting that a
thinly-tagged fixture yields `inferred`, and one asserting the UI renders it
distinguishably.** This also needs carrying into Requirements Analysis as a
functional requirement; flag it in `evidence.md` as a handoff, not just a
practice.

### 5. Keep the 80% floor. Bound what it is measured over.

Org policy ties an 80% line-coverage floor to `feature` scope and forbids
weakening it at `build-and-test`; I am not proposing to weaken it. Two
additions make it mean something rather than becoming theatre:

- **State the measured set.** Line coverage over a codebase containing a
  WebGL basemap, generated WASM glue and visual-identity assets is a number
  that says nothing. Measure the floor over: the domain/lane model, the
  provenance model, the osm2streets adapter, the persistence layer, and the
  editing state machine. Rendering, WASM glue and scaffolding leave the
  denominator via an explicit listed config exclusion — never by lowering the
  number.
- **Report branch coverage on the adapter and the lane/provenance model.**
  That code is a thicket of "tag present / tag absent / tag malformed"
  branches, which line coverage satisfies happily by walking one path.
  Report it (not gate it) until a real number exists, then set a floor on it
  at `nfr-requirements`.
- **Every defect gets a failing test that reproduces it before the fix.** The
  draft says nothing about defects. This is the rule that stops the same
  osm2streets edge case returning in three months when the context is gone;
  it is worth more to a solo builder than to a team.

### 6. Methodology: `test-after` is the wrong answer here. Propose `custom`.

The draft carries org.md's `test-after` default forward unexamined. I dispute
it on two of the three layers, and the stage file explicitly sanctions
`custom` for a mixed cadence.

- There is **no second human**. Tests are the only reviewer. Test-after has a
  specific failure mode when the same agent writes implementation and then
  tests in one pass: the tests get written against the code that exists and
  encode its bugs as expected behaviour. Writing the acceptance criterion
  first, as an executable expectation, is the cheapest available substitute
  for the missing reviewer.
- But **test-first is genuinely wrong for the exploratory work**. B-0 exists
  to find out whether osm2streets works at all (`R-1`, `A-1`). You cannot
  write a meaningful test for output you have not yet seen. Spike, then
  characterise, is correct there.

Proposed structured fields for the lead to paste if the human accepts — both
are stricter than or equal to the org default on every layer, so promotion
should pass the admission conflict-check:

> - **Methodology**: custom
> - **Ordering**: For the domain and lane/provenance model, editing state
>   machine, and persistence — and for anything with a stated acceptance
>   criterion — write the failing test from the criterion first, then
>   implement. For the osm2streets adapter and any exploratory work against
>   the WASM dependency, spike first, freeze the real output as a golden
>   fixture, then refactor behind the adapter interface with those fixtures
>   green. For UI and rendering, implement then test at the interaction level
>   (keyboard path plus axe), never at the pixel level.

Explicitly **no snapshot tests of rendered geometry**. They are pure
maintenance cost for one person and they fail on every legitimate visual
change, which trains the builder to regenerate snapshots without reading
them.

### 7. Performance testing is out of scope, and I am saying so as the performance specialist.

`constraint-register.md` records that no availability or performance target
is fixed; `scope-document.md` repeats it. Test Strategy Standard already
skips performance tests absent an NFR. Load-testing a single-instance $5/month
app against no target is waste, and I would rather state that in
`team-practices.md` than have a later stage add k6 on reflex.

Two corrections to how that interacts with the strategy, though:

- The Standard strategy's "E2E skipped unless NFR requirements exist" must
  **not** be read as skipping the a11y/keyboard browser tests. WCAG 2.1 AA is
  a committed non-functional requirement with a named standard, and a real
  browser is its only validation method. Say this in `team-practices.md` so
  the exclusion is not applied to it later.
- One cheap *budget* guard is worth having, and it is cost, not latency:
  assert a ceiling on the built client bundle and the WASM artifact size in
  CI. That protects `R-8`/`OC-4` on the egress side and phone usability at
  once, for the price of one check.

### 8. Additions to "What the Interview Must Resolve".

The lead's five are right; these are the holes:

6. **Is the repository public from day one?** Settles the CI question outright
   (§1). `LC-1` points at yes.
7. **Is the editing surface DOM/SVG or raster canvas?** Determines whether
   WCAG 2.1 AA is automatically verifiable at all, and therefore both the
   test strategy and the editor's architecture (§2).
8. **What is B-0's pass/fail criterion?** "One street imported, edited and
   saved" has no checkable threshold, yet the brief says B-0 failing reverses
   the Go recommendation. The inception guardrail requires a clear pass/fail
   per requirement, and `project.md` `## Corrections` says to ask rather than
   record the gap as an assumption. Proposed criterion to put to the human:
   *a named real street; the adapter returns lanes with type, direction and
   width for it; the cross-section renders; a lane's type and width can be
   changed by keyboard alone; the change persists and reloads identically;
   and the whole thing runs from a clean checkout with one documented
   command.* That is what retires `R-1`, and it must be written before B-0 is
   built, not after.
9. **Browser and device support matrix.** "Works on phones" is untestable
   without one. Recommend a deliberately small matrix, and note that
   Playwright's WebKit is not Safari — for a product committed to touch
   editing on phones, iOS Safari is the engine that actually breaks, so at
   least one manual pass on a real iOS device belongs in the release
   checklist rather than in CI.
10. **Does the "week to under a day" measure become an acceptance criterion?**
    My position: **not yet.** `R-4` says it is unvalidated with no baseline
    and no user to time. It should be recorded as a hypothesis, and a later
    stage must be blocked from inventing a threshold nobody measured.

### 9. One thing I would cut.

The proposed Code Style hard rule — "naming and modules must not use
Streetmix's own naming or file layout as a starting point" — has no pass/fail
criterion, so nothing can check it and it will sit in `team.md` as
decoration. `R-10`'s stated treatment is stronger *and* enforceable as a
behaviour: do not read Streetmix source while building the editor. Keep that.
If a repo-level rule is wanted, state the verifiable version: the dependency
manifest never contains a Streetmix package, and no file carries a Streetmix
copyright header — both assertable by one CI check. Leave the naming
substance to the developer agent.

## Positions

- AGREE: Wrapping osm2streets behind a thin adapter with its own suite — it is the correct response to `TC-4` and the highest-value test asset in the project, given `D-1` is Critical with only forking as a fallback.
- AGREE: Stating plainly that "review" has no second human and naming what stands in for it — an honest weak control is safer than a review step implying a reviewer who does not exist.
- AGREE: Keeping `[org default]` / `[ideation]` / `[proposed]` provenance on every bullet — it makes the interview a set of real choices rather than a rubber stamp.
- AGREE: Promoting nothing from `team-practices.md` into `discovered-rules.md` — a mandate needs a stated constraint with a named consequence, and the draft holds that line correctly.
- AGREE: Deploy-on-merge to a single Railway environment with no staging tier — for one person on `OC-4`, a second environment buys less than the merge gate does and costs budget that `R-8` says is already tight.
- OBJECT: The CI-cost tension carried into the interview — GitHub Actions is free and unmetered on public repos as of the January 2026 pricing change, and `LC-1` already commits to open source, so the tension dissolves and the question should instead be "is the repo public from day one?".
- OBJECT: "One automated accessibility check per canvas-touching Bolt" — axe-core cannot inspect canvas contents at all, so as written this buys false assurance; the practice must instead constrain the editing surface to focusable DOM/SVG and rest on keyboard-path tests plus a defined manual screen-reader pass.
- OBJECT: `Methodology: test-after` carried forward unexamined — with no second human the tests are the only reviewer, and test-after lets one pass encode the implementation's bugs as expected behaviour; propose `custom` with test-first for the model layers and spike-then-characterise for the WASM boundary.
- OBJECT: The 80% floor left unbounded — line coverage measured over WebGL rendering, WASM glue and visual assets is meaningless; name the measured set and add branch-coverage reporting on the adapter and provenance model.
- OBJECT: No defect practice anywhere in the draft — "every defect gets a failing test before the fix" is worth more to a solo builder against an opaque dependency than to a team, and it is missing.
- OBJECT: B-0 has no pass/fail criterion despite the brief saying its failure reverses the Go decision — the inception guardrail and `project.md` `## Corrections` both require asking rather than recording that as an assumption.
- OBJECT: The Code Style rule about distance from Streetmix naming — unenforceable and uncheckable as written; `R-10`'s "do not read Streetmix source" already covers the risk and can be stated as a verifiable repo check.
