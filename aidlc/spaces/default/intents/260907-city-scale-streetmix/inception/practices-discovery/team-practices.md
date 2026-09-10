# Team Practices — Streetmix at City Scale

Integrated after three blind support reviews (quality, developer, devsecops)
and the human's answers to the practices-discovery interview
(`practices-discovery-questions.md`). Five sections match
`aidlc/spaces/default/memory/team.md` exactly. Where a bullet states a fact
rather than a choice, its source is named; everything else is the practice
this team is affirming going forward.

This project is **not greenfield in the sense the original draft claimed.**
The workspace root already carries the owner's own stated conventions —
`CLAUDE.md`, `scripts/verify.sh`, `docs/state/features.md`,
`docs/state/handoff.md`, `docs/templates/initializer-prompt.md`,
`docs/templates/task-template.md` — and they currently contradict each other:
`CLAUDE.md` names `just build`, `just format`, `just verify`, and
`./scripts/state-summary.sh`, none of which exist yet, alongside
`scripts/next-tasks.sh` and `docs/tasks/`, also missing. This is a workspace
with an incomplete toolchain, not a blank slate, and Q2 settles it.

## Way of Working

- Trunk-based development: all work merges to `main` via short-lived feature
  branches, squash-merged one commit per Bolt named by the Bolt slug.
  No multi-environment release-branch scheme; one trunk, environment-specific
  deploy config if a staging tier is ever justified.
- **The repository is public from the first commit** (Q1). This is not a
  cosmetic choice: on a public GitHub repository, Actions runners, secret
  scanning, push protection, code scanning (CodeQL) and dependency review are
  all free, and they are active during exactly the window — scaffolding, the
  first Railway deploy, the first database URL — when a secret is most likely
  to be committed by accident. Being open source was already decided in
  Ideation (LC-1); this makes it effective from commit one rather than at some
  later, undefined "presentable" point.
- **There is no second human, and "review" is named honestly rather than kept
  as a step that implies a reviewer who does not exist.** The durable
  substitute is the machine gate — `scripts/verify.sh` (or the `justfile` that
  wraps it, see below), run to a clean exit before anything is called done.
  AI support-agent passes (quality, developer, devsecops) are additive on top
  of that gate, not a replacement for it: they are non-deterministic and
  unrepeatable, the machine gate is not.
- **`CLAUDE.md` is brought in line with reality (Q2 — build the missing
  pieces, not just describe what exists):**
  - Add a `justfile` with `build`, `format`, and `verify` targets, where
    `verify` wraps `scripts/verify.sh` so there is exactly one gate and one
    spelling of it.
  - Add `scripts/state-summary.sh` and `scripts/next-tasks.sh`, and create
    `docs/tasks/`, so the session-start instruction in `CLAUDE.md` and the
    task-listing convention it documents both actually work.
  - This is infrastructure, not decoration: for a solo builder working with
    AI assistance, a stale command in `CLAUDE.md` fails on the first attempt
    every session and burns a round every time.
- **Construction Autonomy Mode: gate every Bolt through Stage 1.** Gates cost
  the owner's own time, not a reviewer's, and there is no external deadline
  (`constraint-register.md` OC-3) pushing toward speed over caution while the
  walking skeleton's osm2streets integration risk is still being retired.
  Revisit the ladder once the core editor is proven through Stage 1.
- **Dependency scanning is enabled but tuned to stay quiet.** Enable
  Dependabot **security updates only** (a PR only when a CVE actually affects
  a dependency in use, grouped weekly) — not scheduled version-bump PRs, which
  produce a treadmill a solo builder will not merge and trains the same
  builder to ignore the security PRs sitting in the same queue.

## Walking Skeleton

- **On.** `scope-document.md` names it explicitly — B-0, one street imported,
  edited and saved — to retire the osm2streets integration risk
  (`raid-log.md` R-1) before anything else is built. `initiative-brief.md`
  calls B-0 the one thing that would reverse the Go recommendation if it
  fails.
- Persistence in the skeleton is **without accounts** — accounts belong to
  Stage 2.
- Bolt 1 (the skeleton) is solo and gated; the owner explicitly approves it
  before further Bolts run.
- **B-0's pass/fail criterion (Q3 — round-trips AND provenance holds):** a
  named real street imports through osm2streets with the correct lane count
  and order; an edit is made, saved, reloaded, and comes back identical; and
  mapped values are visibly distinguished from inferred ones in what is
  rendered. All three must hold — a green import that renders correctly but
  cannot tell the owner "this width was measured" from "this width was
  guessed" is not a pass, because presenting a guess as a measurement is the
  failure `raid-log.md` R-2 calls out as most damaging to credibility.
- **The osm2streets build path is settled by Q6, not left to be decided by
  accident during B-0**: build from source, pinned (see Deployment). B-0's
  exit therefore also confirms the pinned build actually produces a working
  binding end to end — it is not just a UI/product exit criterion.

## Testing Posture

- **Methodology**: tdd
- **Ordering**: for every unit of work, write the failing test — or the
  executable form of the stated acceptance criterion — before writing the
  implementation that satisfies it, including at the osm2streets adapter
  boundary, where the expected output is committed as a fixture (see below)
  before the adapter code that must reproduce it is written against that
  fixture. This is the strictest of the options put to the interview and was
  chosen deliberately over the mixed test-first-for-the-model /
  spike-then-characterise-for-the-boundary approach the quality review
  proposed; see `evidence.md` for that tension and why the human's explicit
  choice stands.
- **Coverage floor**: the org default's 80% line-coverage floor for `feature`
  scope applies, measured with **`cargo-llvm-cov`** over a **stated set** of
  Rust crates/modules in the Cargo workspace: the street/lane domain model
  crate, the provenance model, the osm2streets adapter module, the
  persistence layer, and the editing state machine. Because the client
  itself compiles to WASM, there is no separate "WASM glue" category to
  exclude — the whole client is WASM, and the denominator is simply the set
  of crates/modules named above. The view/rendering layer (whichever DOM
  framework Domain Design selects — see the open constraint below) and
  visual-identity assets leave the denominator through an explicit,
  committed `cargo-llvm-cov` ignore pattern — never by lowering the number.
  Branch coverage on the adapter and the lane/provenance model is
  **reported**, not gated, until a real number exists to set a floor against
  at `nfr-requirements`.
- **Every defect gets a failing regression test that reproduces it before the
  fix lands.** Worth more here than on a team: it is what stops the same
  osm2streets edge case returning in three months once the context that found
  it the first time is gone.
- **Merge gate — all of the following pass in CI before squash-merge to
  `main`**, run through the `justfile`/`scripts/verify.sh` gate above:
  1. `cargo build --workspace` and `cargo check --workspace` clean.
  2. `cargo clippy --workspace --all-targets -- -D warnings` clean (clippy
     denies warnings; nothing merges with an unresolved lint).
  3. `cargo fmt --all -- --check` clean.
  4. Unit and component tests green (`cargo test --workspace`).
  5. Coverage floor met on the measured set above (`cargo-llvm-cov`).
  6. The osm2streets adapter's golden-fixture suite green (see below).
  7. Keyboard-path and accessibility tests green, for any Bolt touching the
     editing surface.
- **Two CI tiers, so the gate stays fast**: a fast tier on every push runs
  1–6; a slow tier on PRs to `main` and nightly runs the browser-mode
  keyboard/accessibility tests (item 7) plus the full release-mode WASM
  build of the workspace — which now includes compiling the pinned
  osm2streets crate dependency from source as an ordinary part of the Cargo
  build, not a separate binding step. The build is cached by Cargo's own
  incremental/target-directory caching keyed on `Cargo.lock`, never rebuilt
  from scratch per push.
- **The osm2streets adapter is characterised, not trusted.** Capture real
  osm2streets output for a small, committed set of real OSM extracts (a
  well-tagged street, a thinly-tagged street, a one-way, a street with a
  separately mapped cycleway, one intersection) as committed fixture files,
  and test everything above the adapter against those fixtures — never
  against a live OSM fetch at test time, which would make the suite
  non-deterministic and
  impolite to a free public API. This suite's job is to tell the builder the
  day the dependency's behaviour moves underneath them, not to prove
  osm2streets correct.
- **Provenance is tested, not just rendered.** For any Bolt touching import or
  the editor, the test floor includes at least one test asserting that a
  thinly-tagged fixture yields an `inferred` value, and one asserting the UI
  renders it distinguishably from a `mapped` one. This is what makes Q3's
  success criterion checkable rather than aspirational.
- **Accessibility testing is two distinct things, not one axe-core check.**
  Automated tooling (axe-core, driven from Playwright) is the floor for
  contrast, names, roles and landmarks in the surrounding UI — it cannot see
  inside a `<canvas>` element at all, so it is not evidence about the editing
  surface itself if that surface turns out to be canvas-rendered. The
  per-action check is a keyboard-only interaction test for every editing
  action (select a lane, change its type, change its width, extend along the
  corridor, undo), asserting both the resulting model state and the announced
  accessible name/state. A defined manual screen-reader walkthrough of the
  full edit path happens before each stage's release. **WCAG 2.1 AA is not
  fully CI-verifiable**; a green pipeline is not read as conformance.
- **Open constraint on Domain Design — more load-bearing now, not less, under
  Rust/WASM (Q5 defers the editing-surface choice; it does not remove this
  consequence):** the affirmation-gate decision to build the client in Rust
  compiled to WebAssembly does not force a raster-canvas editing surface.
  Rust/WASM UI frameworks split on exactly this axis: Leptos renders real DOM
  nodes through fine-grained reactivity with no virtual DOM, and Yew and
  Dioxus also render real DOM nodes, through virtual-DOM diffing — none of
  the three requires drawing to `<canvas>`. So Q5's choice remains live and
  is now a framework-selection question for Domain Design as much as a
  rendering-technique one. If Domain Design nonetheless selects a raster
  canvas for the interactive lane elements — in any of these frameworks, or
  by hand-rolling WASM-driven canvas drawing — automated accessibility
  verification of the editing surface itself is not available: axe-core
  cannot inspect canvas contents, and WCAG 2.1 AA conformance there can only
  be checked by the manual screen-reader pass, every release, indefinitely.
  A DOM-rendering choice (Leptos, Yew, or Dioxus used in its normal mode)
  makes each lane a real, focusable, announceable element and keeps the
  automated floor meaningful; a canvas choice does not, regardless of which
  language drew the canvas. This is not a decision made here; it is the cost
  Domain Design is choosing against if it picks canvas.
- No snapshot tests of rendered geometry — pure maintenance cost for one
  builder, and they fail on every legitimate visual change, training the
  habit of regenerating snapshots without reading them.
- No performance/load testing without a stated NFR target — none is fixed yet
  (`constraint-register.md`), and load-testing a single-instance $5/month app
  against no target is waste. One cheap budget guard instead: assert a
  ceiling on the built client bundle and the WASM artifact size in CI, which
  protects the hosting budget (OC-4) and phone usability at once.
- The Test Strategy Standard's "E2E skipped unless NFR requirements exist"
  does **not** apply to the keyboard/accessibility browser tests above — WCAG
  2.1 AA is itself a committed non-functional requirement, and a real browser
  is its only validation method.
- **Not yet resolved, carried to Requirements Analysis rather than decided
  here**: whether the "week to under a day" workflow-time claim
  (`initiative-brief.md`) becomes an acceptance criterion. `raid-log.md` R-4
  records it as unvalidated, with no baseline and no user timed yet. It stays
  a hypothesis until a real baseline exists; a later stage should not be able
  to invent a threshold nobody measured.

## Deployment

- **One environment, deploy on merge to `main` on Railway** (Q7). No separate
  staging tier — there is no budget for a second environment
  (`constraint-register.md` OC-4) and, per the point above, no second person
  whose sign-off a staging gate would be protecting. Rollback uses Railway's
  deployment history/redeploy feature, confirmed at
  environment-provisioning.
- **Deployed is not the same as public — Stage 2's accounts and sharing
  surface stay behind an access gate until data subject rights (erasure,
  export) exist** (Q8). Mechanism: a single server-side flag, read from
  configuration, defaulting to off, gating the account and sharing surface;
  no client-side-only gating and no per-feature toggles to forget. This
  closes the exact gap Q8 identifies: without it, the moment the Stage 2 code
  merges it would be live to the public, ahead of the data-subject-rights
  work that `constraint-register.md` RC-1/RC-2 make a condition of public
  release.
- **The client is a single Cargo workspace; osm2streets is a crate
  dependency, built from source and pinned** (Q6, now realised as a Cargo
  dependency pin rather than a JS binding). The affirmation-gate decision to
  build the client itself in Rust compiled to WASM means osm2streets is
  consumed as an ordinary workspace/path or git dependency of the client
  crate — not through `osm2streets-js`, the WASM-bindgen wrapper package.
  This is a genuine simplification the Rust choice buys, not merely a
  substitution of one dependency for another: `osm2streets-js` on npm is
  `0.1.4`, published 2023-06-04, unattested, and behind the repository's
  `main` (last pushed 2025-10-02) — stale enough on its own to force a
  from-source build either way — but consuming the crate directly also
  removes the JSON-string API boundary that package exposed (see Code Style
  below and `evidence.md`), because there is no longer a JS/WASM boundary to
  cross between the client and osm2streets at all; both compile into the
  same Rust binary. The unpinned-dependency risk itself is unchanged and
  still requires the same discipline: `osm2streets-js/Cargo.toml` on `main`
  carries `abstutil = { git = "https://github.com/a-b-street/abstreet" }`
  with no `rev`, `tag`, or `branch` — an unpinned dependency on a repository
  `raid-log.md` already records as dormant. The core `osm2streets` crate
  (consumed directly, rather than `osm2streets-js`) should be checked for
  the same unpinned pattern in its own `Cargo.toml` at implementation time,
  rather than assuming it is identical. The practice: fork
  `a-b-street/osm2streets` into the project's own account, add it to the
  Cargo workspace as a git dependency pinned to a specific commit SHA
  (`rev = "<sha>"`), pin the `abstutil` git dependency the same way, and
  commit `Cargo.lock`. Record the upstream commit, the build command, and
  the toolchain versions (`rustc`, `wasm-pack`/`trunk` or equivalent) in one
  committed file — the pass/fail test for this practice is "can this be
  rebuilt in six months without reconstructing what was done." This also
  pre-positions `raid-log.md` R-3's stated fallback (forking) as a fact
  rather than a paper plan.
- **Secrets are never committed.** All secrets are supplied as Railway
  environment variables; no `.env` file in the repository, `.env`/`.env.*` in
  `.gitignore` from the first commit. Because the repository is public, a
  leaked secret is immediately world-readable rather than an internal
  incident: if one is ever pushed, rotate it first, then clean history —
  history rewriting alone is not remediation once others may have cloned it.
- **Push protection stays on** (free and default on a public repository) as
  the highest-value control available, because it blocks at the moment of
  the mistake rather than reporting it afterwards. It will not catch this
  project's actual secret shapes (a session-signing key, a `DATABASE_URL`
  with an embedded password, an unrecognised basemap API token), so one
  `gitleaks` step runs in CI to cover the generic-entropy and
  connection-string classes it misses. CI is preferred over a local
  pre-commit hook because a hook that must be installed per machine is a
  control that can silently stop working; a local hook is a fine addition
  for faster feedback, never the substitute.
- **`SECURITY.md` with a disclosure contact, plus GitHub private
  vulnerability reporting, is enabled.** With no second person and a public
  repository, the realistic first discoverer of a flaw is a stranger; without
  a stated private channel they either file a public zero-day or say
  nothing. This is the one control that directly answers the solo-builder
  constraint (OC-1) at zero recurring cost.
- **CodeQL default setup is enabled at the start of Stage 2**, named as a
  line item on the public-release gate that already exists for that stage,
  rather than "do it sometime." It is free on a public repository and would
  scan a client-side editor with no auth and no personal data for no return
  if turned on any earlier.
- **SonarQube Cloud runs as a CI-based scan, not Automatic Analysis, and sits
  in the slow CI tier** (human-directed at the affirmation gate, after the
  interview closed — see `evidence.md`). Rust is not supported by SonarQube
  Cloud's Automatic Analysis mode as of this writing, so the scan is an
  explicit GitHub Actions step running the Cargo/Rust toolchain (Clippy
  included, since the Sonar Rust analyzer is itself built on Clippy) rather
  than a configuration toggle in the Sonar UI. It runs alongside the
  release-mode WASM build and the browser-mode accessibility suite in the
  slow tier described in Testing Posture — on PRs to `main` and nightly,
  never on every push — because it only makes sense to pay for a CI-based
  Rust scan once the fast-tier items (build, clippy, fmt, unit tests,
  coverage) have already passed. **A failing Sonar quality gate blocks the
  squash-merge to `main`**, on the same footing as the other slow-tier items;
  it is a merge gate, not a report to browse. The ordering cost is accepted
  as the price of that gate: a CI-based Rust scan is slower than any
  fast-tier check, which is exactly why it is tiered rather than run on
  every push.
- **No DAST.** There is nowhere safe to point a scanner with a single
  production environment and no staging tier — an active scan would write
  into the live database and drive metered usage the budget (OC-4, R-8)
  cannot absorb. Replaced with deterministic assertions in the normal test
  suite: response headers (`Content-Security-Policy`,
  `Strict-Transport-Security`, `X-Content-Type-Options: nosniff`,
  `Referrer-Policy`), session cookie flags (`HttpOnly`, `Secure`, `SameSite`,
  Stage 2), and an object-level authorization check — an unauthenticated
  request for another user's saved design returns 403/404, not the design
  (Stage 2). These run on every merge for free instead of as a periodic scan
  nobody has a staging target for.
- **Every security control here either blocks at the moment of the mistake
  with a specific actionable message, or it is off.** No control produces a
  dashboard, a periodic report, or a queue for one person to work through —
  that is the honest reading of the solo-builder constraint, and it is why
  DAST and a second SAST product are rejected above and not just deferred.
  The equivalent question for the Rust toolchain is `cargo audit` against
  the RustSec advisory database — and it falls on the other side of that
  line from `npm audit`: RustSec is a small, human-curated database scoped
  to crates.io, and this workspace's own dependency tree (the client crates
  plus the pinned osm2streets fork) is far shallower than a typical
  JavaScript frontend's transitive tree, so `cargo audit` does not reproduce
  the low-severity, high-volume noise that made `npm audit` an unmerged
  queue here. `cargo audit` runs as an actual CI gate (the slow tier,
  alongside the release-mode WASM build), scoped the same way as
  Dependabot above — a finding blocks merge because a genuine RustSec
  advisory against a dependency in use is rare enough to be worth a solo
  maintainer's attention every time, not a queue to triage.
- Commit `Cargo.lock` and build from it in CI and at deploy — `cargo build
  --locked` (or the WASM-target equivalent), never a build that lets
  dependency versions float. Which Railway builder can actually build a Rust
  workspace to a WASM artifact (a Nixpacks Rust provider versus a
  project-supplied `Dockerfile`) is confirmed at environment-provisioning
  rather than assumed — this is a build-toolchain question the Rust/WASM
  decision reopens and Node tooling never posed.

## Code Style

- **Formatter and linter, per the org default's language-idiomatic
  clause: `rustfmt` and `clippy`, not Prettier and ESLint** — the Rust/WASM
  client decision moves the whole toolchain, not just the target platform.
  Formatting **checked** (`cargo fmt --all -- --check`) in CI rather than
  only configured; `clippy` runs with warnings denied
  (`cargo clippy --workspace --all-targets -- -D warnings`), so a lint that
  would have been a Prettier/ESLint warning is instead a compile-time-grade
  failure — stricter than the org default's "linter runs in CI, failure
  blocks the PR," not a weaker substitute for it.
- **SonarQube Cloud extends static analysis beyond Clippy, configured as a
  blocking quality gate on new code, not a dashboard** (human-directed at
  the affirmation gate, after the interview closed — see `evidence.md`).
  It runs in Sonar's own "Clean as You Code" mode: the gate evaluates only
  newly-written or changed code on each merge, not the project's historical
  code as a whole, so it yields one pass/fail on the diff rather than a
  backlog to work through. Named honestly rather than oversold: the Sonar
  Rust analyzer is itself built around Clippy, so a real share of what it
  would flag on this project's own code already fails the merge gate via
  `cargo clippy -D warnings` above. What Sonar adds beyond that overlap is
  duplication detection, cognitive-complexity and maintainability metrics,
  and its own security ruleset — none of which Clippy provides. Free for
  this project: SonarQube Cloud has no line-of-code cap and no expiry on
  public repositories (the 50k-LOC cap applies to private projects only),
  and the repository is public from the first commit (Q1). See Deployment
  below for where the scan sits in the merge gate and CI tiering.
- **The central data shape is an ordered, positionally-addressed lane
  list; index safely.** Rust does not have TypeScript's
  `noUncheckedIndexedAccess` flag because the equivalent failure mode is
  handled differently in Rust: direct `Vec` indexing (`lanes[i]`) panics at
  runtime on an out-of-range index, so lane access uses `.get(i)` returning
  `Option<&Lane>` (or an explicit bounds-checked accessor on the lane-list
  type) everywhere an index is not already known valid by construction —
  converting the same off-by-one/undefined-at-the-edge bug class into a
  compile-time `Option` the caller must handle, rather than a runtime panic
  or an unchecked read.
- **Metres are the only unit in the core.** osm2streets and OSM are metric;
  the product must also serve non-metric jurisdictions (TC-5). Store metres,
  convert only at the presentation boundary.
- **Naming authority is osm2streets' schema.** Domain vocabulary (road, lane,
  intersection, lane-type and direction terms) follows osm2streets', so the
  adapter's mapping stays near-identity and readable. This replaces the
  earlier proposed rule about keeping naming "distant" from Streetmix — see
  `evidence.md` for why that rule was dropped.
- **Every dimension carried in the imported model states its provenance at
  the type level; the un-annotated form is unconstructable.** No width,
  count, or offset for a physical quantity is a bare `f64` once it has
  crossed the osm2streets adapter boundary:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq)]
  pub enum Provenance { Mapped, Inferred, UserSet }

  #[derive(Clone, Copy, Debug)]
  pub struct Dimension { pub metres: f64, pub provenance: Provenance }
  ```
  This operationalises Q3's success criterion and `raid-log.md` R-2's stated
  treatment as a type rather than a rendering-time decoration that a later
  function taking a bare number can silently drop.
- **The imported baseline is immutable; a design is an overlay, never a
  mutation of it.** The osm2streets-derived baseline is held behind
  shared (`&`), never mutable (`&mut`), references once constructed; edits
  live in a separate, project-owned overlay layer. This is the code-level
  form of the existing `NEVER edit OSM data` rule below, and it is also what
  forbids reaching for the underlying osm2streets crate's own in-place
  mutation methods (the same operations the earlier `osm2streets-js`
  binding exposed as `overwriteOsmTagsForWay`, `collapseShortRoad`,
  `collapseIntersection`, `zipSidepath`, now reachable as native Rust
  methods on the crate directly rather than across a JS/WASM boundary) as
  an editing mechanism.
- **The overlay is keyed on OSM way id plus a project-owned lane
  discriminator, never on osm2streets' positional (`usize`) indices** — those
  indices are not stable across a library version bump or a re-fetch of the
  OSM extract, and a saved design keyed on them would silently corrupt. This
  belongs in `CLAUDE.md`'s `## Known traps` once there is a first trap to
  record.
- **Three inward-pointing layers, enforced by crate boundaries and module
  visibility, not by a linter plugin, so a boundary violation fails the
  build itself rather than a review that has no reviewer**: (1) the
  osm2streets adapter — its own crate (or a `pub(crate)`-sealed module), the
  only place permitted to depend on the osm2streets crate directly, a
  parse-and-validate boundary that turns its native structs into
  project-owned plain data carrying provenance, and never re-exports an
  osm2streets type past it; (2) the core street model and editing
  operations — a separate workspace crate with no dependency on the
  osm2streets crate, no map-rendering crate, no UI framework crate, no
  jurisdiction constants; (3) the outer ring — UI, persistence, jurisdiction
  packs, export — which depends inward (on the core crate's public API) and
  is organised by feature, not by layer. Cargo's own dependency graph is the
  enforcement mechanism: a workspace member cannot depend on a crate that
  isn't declared in its `Cargo.toml`, so an inward-pointing violation is a
  compile error, not a lint that can be silenced with a comment. Because
  osm2streets is now a direct crate dependency rather than a WASM-bound npm
  package (see Deployment), the adapter's job simplifies from "parse and
  validate a JSON string" to "convert a native osm2streets struct into this
  project's own provenance-carrying types" — the type-safety boundary
  osm2streets-js's JSON-string API could not offer is now available for
  free at compile time (see `evidence.md`).
- **Errors at integration boundaries are typed results, not exceptions
  (Rust: not `panic!`).** The adapter maps every `Result`/error the
  osm2streets crate itself returns — and any panic it cannot avoid
  triggering — to a small closed set of typed failure reasons in the
  project's own error enum, and never lets an osm2streets-native error type
  reach a caller (logging the underlying error once with the OSM way id,
  never rendered to a user as-is). Editing operations return outcome plus
  findings — validation the UI can render before anything is applied —
  using the same shape. Rust's `panic!` is reserved for genuine programmer
  error (an invariant the type system could not express), matching the
  existing "exceptions are reserved for programmer error" intent exactly.
- **Test hooks: accessible name and role first, `data-testid` second.** This
  is a DOM-testing convention, not a JavaScript-toolchain one, and it
  carries over unchanged to a Rust/WASM DOM-rendering framework (Leptos,
  Yew, or Dioxus, whichever Domain Design selects — see the open constraint
  above): the framework still renders real DOM elements that a browser
  automation tool queries the same way regardless of which language
  produced them. Query interactive elements by role and accessible name as
  the primary selector, so interaction tests double as accessibility
  evidence; reserve a `data-testid` attribute for elements that legitimately
  have no accessible name (individual lane blocks, if the editing surface
  ends up DOM-rendered) or, if Domain Design selects canvas, for the canvas
  element itself.
- **A committed asset and dependency manifest, checked by
  `scripts/verify.sh`**, records origin and licence for every third-party
  asset (icon, image, font) and dependency, updated whenever one is added.
  `verify.sh` fails the gate if a Streetmix package appears in the dependency
  manifest or the lockfile. This — not a naming-distance convention — is the
  enforceable form of keeping Streetmix at arm's length; see `evidence.md`
  and `discovered-rules.md`.
- **For the injection class, prefer the DOM-rendering framework's default
  escaping over any raw-HTML-injection API.** Leptos, Yew, and Dioxus all
  escape text interpolated into DOM nodes by default; the practice is to
  never reach for whichever raw/unchecked HTML-insertion API the selected
  framework exposes (e.g. Yew's `Html::from_html_unchecked`, or an
  equivalent `inner_html`-style escape hatch in Leptos/Dioxus) for
  user-authored street or lane labels. There is no ESLint-style linter
  plugin doing this enforcement automatically in the Rust ecosystem today;
  it is a stated code-review and `clippy`-comment discipline until or
  unless a `clippy` lint for it becomes available, and the practice is
  named here specifically so it is not silently dropped for lack of an
  automated check.
- Streetmix source is never read while building the editor, per
  `raid-log.md` R-10's own stated avoidance treatment — a behavioural
  practice, not a naming convention, and the one that actually protects
  against the licence risk.

## Sources

- `aidlc/spaces/default/memory/org.md`, `team.md`, `project.md`
- `initiative-brief.md`, `constraint-register.md`, `raid-log.md`,
  `scope-document.md`, `wireframes.md` (Ideation, approved)
- `CLAUDE.md`, `scripts/verify.sh`, `docs/state/`, `docs/templates/` (workspace
  root, inspected directly)
- `osm2streets-js/Cargo.toml` and `src/lib.rs` on `a-b-street/osm2streets`
  `main` (read directly)
- `practices-discovery-questions.md` — the interview and its eight answers
- `contributions/aidlc-quality-agent.md`, `aidlc-developer-agent.md`,
  `aidlc-devsecops-agent.md`
- SonarQube Cloud documentation and pricing, checked live for this revision
  (`https://docs.sonarsource.com/sonarqube-cloud/analyzing-source-code/languages/rust`,
  `https://www.sonarsource.com/blog/introducing-rust-in-sonarqube/`,
  `https://www.sonarsource.com/plans-and-pricing/`)
