# Units of Work — Streetmix at City Scale

Upstream inputs: `components.md` and `decisions.md` (domain-design),
`requirements.md` (requirements-analysis), `stories.md` (user-stories),
`team-practices.md` (practices-discovery), `units-generation-questions.md`
(this stage).

A Unit of Work is an independently implementable piece of the solution. This
file defines the twelve Units, what each owns, and what each is. It describes
**topology only** — `unit-of-work-dependency.md` carries the dependency graph,
and the order in which Units are actually built is Delivery Planning's
decision, not this stage's. Nothing here recommends a build order, names a
critical path, or ranks Units by value or risk.

## How this decomposition was arrived at

The eleven questions in `units-generation-questions.md` settled it. Four
answers shaped it most:

- **Unit boundaries follow feature slices** (Q1) — Units named for what they
  deliver rather than for which architectural layer they occupy.
- **Four server Units** (Q3, Q8) — the proxy, design storage, accounts and
  sharing, and data subject rights, kept apart because they do not share a
  schedule and because `project.md` makes the rights work a release gate.
- **Three foundation Units** (Q5, Q6, Q10) — the pinned osm2streets build, the
  stored-design payload contract, and the core street model. Each is shared by
  everything and owned by no feature.
- **The presentation surface is one Unit** (Q11) — forced, not chosen. Feature
  slicing produced a cyclic Unit graph, because `MapView`, `CrossSectionView`
  and `AppShell` each read from every model layer while no model layer reads
  back. The detail is in `unit-of-work-dependency.md` § "Why the views are one
  Unit"; the short form is that views are sinks, so pairing a view with one
  feature's model draws edges into every other feature's model and those
  features draw edges back.

**The Unit count is twelve, and the 8–10 band chosen at Q2 was not reachable.**
Four server Units plus three foundation Units is seven before any feature slice
exists, and the client needs five. This is recorded rather than smoothed over:
each per-Unit stage in Construction runs twelve times, and `team-practices.md`
sets Construction Autonomy Mode to gate every Bolt through Stage 1, so those
gates cost the owner's own time.

## Unit index

| Unit ID | Directory | Kind | Complexity | Deployment model |
|---|---|---|---|---|
| U1 | `u1-osm2streets-build` | packaging | M | embedded — compiled into the client artifact |
| U2 | `u2-street-core` | library | L | embedded — a crate in the client Cargo workspace |
| U3 | `u3-design-payload-spec` | spec | S | embedded — consumed in place by four Units |
| U4 | `u4-street-import` | library | L | embedded — a crate in the client Cargo workspace |
| U5 | `u5-design-editing` | library | L | embedded — a crate in the client Cargo workspace |
| U6 | `u6-client-surfaces` | ui | XL | embedded — the WASM bundle, served by the single Railway service |
| U7 | `u7-local-persistence` | ui | M | embedded — a crate in the client Cargo workspace |
| U8 | `u8-meeting-output` | ui | M | embedded — a crate in the client Cargo workspace |
| U9 | `u9-osm-extract-proxy` | service | S | shared — one Railway service (Q7) |
| U10 | `u10-design-storage` | service | M | shared — the same Railway service |
| U11 | `u11-accounts-sharing` | service | L | shared — the same Railway service |
| U12 | `u12-data-rights` | service | M | shared — the same Railway service |

**Four Units, one deployable.** Q7 chose a single Railway service serving both
the built WASM bundle and the API, against the ~$5/month working budget
`project.md` protects. U9 through U12 are four *work* boundaries inside one
*deployment* boundary. Which Railway builder can build a Rust workspace to a
WASM artifact — a Nixpacks Rust provider versus a project-supplied
`Dockerfile` — is confirmed at environment-provisioning, per
`team-practices.md`, not assumed here.

## Unit definitions

### U1 — `u1-osm2streets-build` · packaging

**Owns.** The forked `a-b-street/osm2streets` repository pinned to a specific
commit SHA, its `abstutil` git dependency pinned the same way, the committed
`Cargo.lock`, the single committed file recording the upstream commit, the
build command and the toolchain versions (`rustc`, `wasm-pack`/`trunk`), and
the release-mode WASM build configuration.

**Boundary.** Everything needed to reproduce the dependency, and nothing that
uses it. `StreetImportAdapter` lives in U4.

**Why it is its own Unit.** `raid-log.md` records osm2streets as the project's
single Critical dependency (D-1, R-1) and `project.md` mandates the pin.
`team-practices.md` states the pass/fail test for the practice directly — "can
this be rebuilt in six months without reconstructing what was done" — which is
a Definition of Done, and a Definition of Done belongs to a Unit.

**Components.** None from the catalogue. This Unit is build and distribution
artefacts, which is what the `packaging` kind names.

**Implementation notes.** `team-practices.md` requires checking the core
`osm2streets` crate's own `Cargo.toml` for the same unpinned-`abstutil`
pattern found in `osm2streets-js/Cargo.toml` rather than assuming they match.
The golden-fixture suite (`team-practices.md` Testing Posture) is authored
here and asserted from U4.

**Constraints.** `project.md`: never an unpinned branch, an untagged
repository reference, or a floating `git` dependency with no `rev`. Build with
`cargo build --locked` in CI and at deploy.

---

### U2 — `u2-street-core` · library

**Owns.** `StreetModel` — the `Street`, `Lane` and `StreetNetworkGraph`
entities, the `Dimension`/`Provenance` types where the un-annotated form is
unconstructable, the derived lane key, the project-wide carriageway-width
definition, bounds-checked lane access, and the `StreetSource` port that
`StreetImportAdapter` implements.

**Boundary.** The imported baseline and its vocabulary. No overlay, no
editing, no rendering, no jurisdiction constants.

**Why it is its own Unit.** Q10. Under feature slicing there was nowhere for
it to live: every feature slice depends on all of it and none of them owns it.

**Components.** `StreetModel`.

**Implementation notes.** Metres are the only unit in the core; conversion
happens at the presentation boundary (`project.md` TC-5 mandate, restated in
`team-practices.md` Code Style). The overlay key is the OSM way id plus the
direction-normalised bounding node-id pair plus the project-owned lane
discriminator — the corrected scheme recorded in `team.md`'s Corrections, never
osm2streets' positional indices. The baseline is held behind shared, never
mutable, references.

**Constraints.** `project.md`: the model stays expressible without baking in
any single jurisdiction's classifications, widths or terminology.

---

### U3 — `u3-design-payload-spec` · spec

**Owns.** The serialised shape of a design: its lane edits, its corrections,
the anchors for lanes that exist in no import, and every value's provenance.

**Boundary.** The shape only. The four components that read and write it live
in U7, U10 and U12.

**Why it is its own Unit.** Q5. One shape crosses the device/server line and
the Rust/database line, and is read by `LocalDesignStore`, `UploadClient`,
`DesignRepository` and `DataRightsService`'s export. Contract Design (2.8)
formalises it; giving it one owner means a change to it has one place to break.

**Components.** None from the catalogue — a contract consumed in place, which
is what the `spec` kind names.

**Implementation notes.** AC11.2.3 requires an export in which every inferred
value is identifiable as inferred, so provenance is part of the wire shape and
not a rendering-time decoration. AC8.1.1 and AC8.1.2 require a reload to come
back identical in lane order, types, widths and every provenance state.

---

### U4 — `u4-street-import` · library

**Owns.** Fetching extract bytes and classifying what comes back; running the
pinned osm2streets crate and converting its native structs into the project's
provenance-carrying types; and corrections — what a user fixed because the
import was wrong, and what happens to those fixes on a later import.

**Boundary.** From the request for an area to a correct, corrected street in
the core's types. No editing, no rendering.

**Components.** `ExtractFetcher`, `StreetImportAdapter`, `CorrectionOverlay`.

**Implementation notes.** The adapter is the only place permitted to depend on
the osm2streets crate (`team-practices.md` Code Style), enforced by the crate
boundary rather than by review. It implements the core's `StreetSource` port
rather than being called directly by the outer ring (`decisions.md` ADR-009).
Its error log is local to the component; the maintainer-facing half of US7.4
is realised at `observability-setup`. Corrections carry an import fingerprint
and resolve to exactly one of three outcomes on re-import (AC7.5.2–AC7.5.4).

**Constraints.** `project.md`: never edit the underlying OpenStreetMap data.
Every osm2streets error and any panic it cannot avoid triggering maps into the
project's closed typed-failure set; no osm2streets-native error type reaches a
caller.

---

### U5 — `u5-design-editing` · library

**Owns.** The proposed change and the machine that applies it: lane edits,
additions and removals held as a proposal over the baseline; selection state;
the undo history including multi-street entries; and corridor work —
connectivity, fit assessment and the type-and-ordinal correspondence rule.

**Boundary.** Model and state, not surface. The controls that issue these
actions are in U6.

**Components.** `DesignOverlay`, `EditingSession`, `CorridorPlanner`.

**Implementation notes.** Corridor work folded in here per Q9, because
`CorridorPlanner` writes the applied design onto each target through
`DesignOverlay` and undo reverses a corridor apply as a single unit
(AC5.5.2) — the two share a state machine. Editing operations return an
outcome plus findings rather than throwing. Fit is assessed on total width
alone against the core's carriageway definition, with no lane-type
compatibility rule, so no jurisdiction's classifications enter the core
(AC6.2.6). A bulk apply is not atomic (AC6.4.3).

---

### U6 — `u6-client-surfaces` · ui

**Owns.** Everything the user looks at and the entry point that assembles it:
the map surface and its four overlays; the lane strip and its controls; the
landing page, routing, the live status region carrying all 23 committed
messages, the layered Escape rule and the skip link; and the WebAssembly entry
point that constructs the concrete adapter, supplies it through the core's
port, and mounts the application.

**Boundary.** Presentation and composition. It holds no model.

**Why these four are one Unit.** Q11, and it is the one boundary in this file
that was forced rather than chosen. See `unit-of-work-dependency.md` § "Why the
views are one Unit".

**Components.** `MapView`, `CrossSectionView`, `AppShell`, `CompositionRoot`.

**Implementation notes.** `decisions.md` ADR-003 settles the split: real page
elements for the lane strip and its controls, canvas only for the map. Every
map-only affordance has a twin in the document flow, because a canvas is a
surface automated tooling cannot inspect. Provenance renders in three channels
at once — hatch, written qualifier, accessible name — which is what makes B-0's
third pass criterion automatable rather than a manual screen-reader
walkthrough forever. `CompositionRoot` is the single outer component permitted
to name a concrete adapter type and holds no product behaviour
(`decisions.md` ADR-009).

**Constraints.** WCAG 2.1 AA. `team-practices.md` requires keyboard-only
interaction tests for every committed editing action and a manual
screen-reader walkthrough before each stage's release; a green pipeline is not
read as conformance. This is the largest Unit in the set — 11 stories — and
that is a direct consequence of the acyclicity constraint, not an estimate.

---

### U7 — `u7-local-persistence` · ui

**Owns.** Everything held in this browser — designs, their corrections, the
index that finds them again, storage-availability and save-failure
detection — plus the one affordance that moves a design off the device and the
anonymous identifier that lets it be reached and removed again.

**Boundary.** The device, and the single line that crosses off it.

**Components.** `LocalDesignStore`, `UploadClient`.

**Implementation notes.** Upload folded in with local storage per Q9: the
upload reads the stored design and records its upload state on the same index
entry, so they share a record. Nothing leaves the device unless explicitly
asked (AC8.3.2). Removal requires no account because the upload required none
(AC8.3.4). A first-time visitor with empty storage must not be told anything
was lost (AC8.2.1, AC8.2.2). `decisions.md` ADR-006 settles the identifier: a
128-bit browser-minted random value from a cryptographic source, no edit token.

---

### U8 — `u8-meeting-output` · ui

**Owns.** Meeting-ready output chosen by purpose rather than file format, and
the rule that provenance either survives into the output through a non-colour
channel or the value is omitted.

**Boundary.** Reads the model; writes nothing back.

**Components.** `ExportComposer`.

**Implementation notes.** Product Stage 3, and a distinct lifecycle from
everything else in the client. AC12.2.3 requires omitting a value whose
marking cannot be carried rather than showing it unmarked — which is the same
rule as `raid-log.md` R-2, applied at the output boundary.

---

### U9 — `u9-osm-extract-proxy` · service

**Owns.** Fetching OpenStreetMap extracts on a browser's behalf, caching them
keyed on the extract itself, rate-limiting its own traffic toward the upstream
public API, and logging neither addresses nor requested locations.

**Boundary.** No user data, no design data, no account data.

**Components.** `OsmExtractProxy`.

**Why it is its own Unit.** Q3. It holds no user data, carries a privacy
obligation none of the other server components has (`decisions.md` ADR-004: a
log pairing an address with a requested location is location data about an
identifiable device), and is needed before anything else exists — nothing
imports without it.

**Implementation notes.** Its egress cost is unmeasured. `team-practices.md`
requires a real figure at B-0, and `project.md` forbids letting spend grow past
the ~$5/month working budget without treating that growth as a constraint
change. The measurement is a gate, not a curiosity.

**Known divergence.** AC3.1.7 ("no request is made to the application's own
server" during a full import) is false by design under this Unit, and NFR5.2
needs restating. Both are recorded as divergences 6 and 7 in `decisions.md`
ADR-001 and remain outstanding amendments to `requirements.md`.

---

### U10 — `u10-design-storage` · service

**Owns.** Stored designs — anonymous uploads now, account-owned designs from
product Stage 2 — their retention, object-level authorisation on every read,
and rate limiting against identifier guessing.

**Boundary.** Storage and authorisation. Not accounts, not grants, not
erasure.

**Components.** `DesignRepository`.

**Implementation notes.** This Unit is needed in product Stage 1, not Stage 2:
US8.3 is a Stage 1 Must Have and uploads to it. An uploaded anonymous design
expires 30 days after creation; an account's design is retained until the
account is deleted. It takes no action about designs held only on a device,
because it never received them (AC11.3.3). Refusal returns not-found or
forbidden and never the design, asserted as a response rather than a UI state
(AC10.1.2, AC10.1.3).

**Known divergence.** `scope-document.md` places all server-side storage in
Stage 2. Divergence 5 in `decisions.md` ADR-001 records that Stage 1 now
includes server-side storage of anonymous uploaded designs, and that the
amendment is outstanding.

---

### U11 — `u11-accounts-sharing` · service

**Owns.** Accounts and sessions, the server-side access gate that holds the
whole surface back, migration of a device design on sign-up, and the two
independent sharing mechanisms — named grants and a public link.

**Boundary.** Identity and authorisation grants. Erasure and export are U12.

**Components.** `AccountService`, `SharingService`.

**Implementation notes.** `project.md` mandates that this surface stays behind
a server-side access gate — a single flag read from configuration, defaulting
to off, enforced in the service and never by hiding the interface — until
erasure and export exist. The deploy step is explicitly *not* that gate. Named
access and link access are independent in both directions (AC10.2.1–AC10.2.4);
`decisions.md` ADR-007 settles that a named grant requires an account and a
public link does not.

**Constraints.** Session cookies carry `HttpOnly`, `Secure` and a `SameSite`
policy — an engineering requirement recorded in `stories.md` § "Verification
notes" and asserted in the Stage 2 test suite rather than as an acceptance
criterion. CodeQL default setup is enabled at the start of product Stage 2
(`team-practices.md`).

---

### U12 — `u12-data-rights` · service

**Owns.** Account and design erasure including its partial-failure behaviour,
and machine-readable export preserving provenance.

**Boundary.** The release gate itself.

**Why it is its own Unit.** Q4, confirmed against Q3 at Q8. `project.md`
mandates data subject rights as functional requirements gating product Stage
2's public release, never deferred operational work, and `components.md` calls
`DataRightsService` "a release gate rather than a feature". Giving it a Unit
boundary is what makes the gate verifiable independently of the surface it
gates: U11 cannot be opened to the public until U12 is done, and that is an
edge in the graph rather than a line in a checklist.

**Components.** `DataRightsService`.

**Implementation notes.** A deletion that fails partway is retried to
completion or leaves the account intact and says so — never a half-deleted
account with orphaned designs, because the gate it serves is a legal one
(AC11.1.4). Export carries lane order, types, widths and provenance, with
every inferred value identifiable as inferred (AC11.2.1–AC11.2.3), which is
why it consumes U3 rather than serialising its own shape.

## Units that carry no story of their own

Four of the twelve are foundations or infrastructure. They serve named
acceptance criteria owned by other Units' stories rather than delivering a
story themselves, and `unit-of-work-story-map.md` records which:

| Unit | Serves |
|---|---|
| U1 `osm2streets-build` | AC3.1.4 and AC3.1.5 — the golden-fixture assertions against the pinned build, and the only assertion in the set that goes red the day the dependency moves |
| U2 `street-core` | The provenance half of US4.1 and US4.2; the derived lane key behind AC5.2.3 and AC7.3.4; the carriageway definition behind AC6.2.1–AC6.2.5 |
| U3 `design-payload-spec` | AC8.1.1, AC8.1.2 (reload identical); AC8.3.1–AC8.3.5 (upload); AC9.2.1 (migration preserves every edit and provenance state); AC11.2.1–AC11.2.3 (export preserves provenance) |
| U9 `osm-extract-proxy` | The fetch half of US3.1, and its own privacy obligation, which traces to NFR6 rather than to a story |

This is a property of the decomposition, not an omission: a feature-sliced
plan that also has foundation Units will have Units that no single feature
owns. It is recorded here so the coverage check in
`unit-of-work-story-map.md` reads as deliberate.

## What this file does not decide

Build order, critical path, Bolt grouping, and which Unit ships first. Those
are Delivery Planning's (2.9) economic decisions, taken over the dependency
graph this stage produces. The graph permits many orderings; choosing among
them requires judgement about what to prove first, which is not a topological
question.

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-10T23:15:02Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `unit-of-work-dependency.md` § "Why the views are one Unit", paragraph on `AppShell` | The claim "`AppShell` depends on local persistence, upload, export, accounts and sharing — every one of which depends back on the editing model" is checked against `components.md`'s own `depends_on` edges and is false for two of the five: `AccountService` depends only on `DesignRepository`, and `SharingService` depends only on `DesignRepository` and `AccountService` — neither reaches `DesignOverlay`, `EditingSession` or `CorridorPlanner` (the editing model) directly or transitively. Only `LocalDesignStore`/`UploadClient` and `ExportComposer` actually close a loop with the editing model as claimed. This is the section the stage explicitly calls "load-bearing" for departing from pure feature slicing (Q1) into the forced `client-surfaces` grouping (Q11), so the overstatement matters even though the section's ultimate conclusion is independently supported by `MapView`'s real cycle with `CorrectionOverlay`/`CorridorPlanner`/`DesignOverlay` (verified accurate). | Correct the sentence to name only the three dependencies that actually close a loop (local persistence, upload, export), or state explicitly that accounts/sharing are included in the grouping for a different reason (e.g. Q11's "all three views as one Unit" choice, not a cycle each of them individually causes). | New |
| R-02 | Minor | `unit-of-work-dependency.md` § "Depth of the graph" | The chain "`street-core → design-payload-spec → design-storage → accounts-sharing → client-surfaces`" is captioned "the longest chain of dependencies is six Units", but the chain as written lists five Units (4 edges, 5 nodes) and a topological longest-path check over the parsed edge block confirms 5 is the correct length (tied with the other listed chain, which is correctly captioned "five"). | Correct "six Units" to "five Units" (or otherwise reconcile the caption with the listed chain). | New |
| R-03 | Minor | `unit-of-work-dependency.md` edge block, `local-persistence` `depends_on` | `local-persistence` (U7) declares `depends_on: [street-core, ...]`, but no component owned by U7 (`LocalDesignStore`, `UploadClient`) depends directly on `StreetModel` in `components.md`'s own edge list — the path to `street-core` runs only transitively through `street-import`/`design-editing`, both of which U7 already declares. This is a redundant (not incorrect) edge with no direct component edge behind it, per the stage's own stated derivation rule ("Unit A depends on Unit B when a component in A depends on a component in B"). | Either drop the redundant direct edge to `street-core` (transitivity through `street-import`/`design-editing` already implies it) or note in the derivation section that a small number of edges are added for direct-consumption clarity beyond the strict component-edge derivation rule. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| Custom parser (`python3`/PyYAML) over `unit-of-work-dependency.md`'s fenced edge block | PASS: 12 uniquely-named units, all `depends_on` references resolve to declared units, no self-dependencies, all `kind` values in `{service, spec, ui, packaging, library}`, topological sort completes (acyclic) | Confirms the stage's own "Verification" section claims against the actual parsed structure, not the prose |
| Unit-set equality check: `unit-of-work.md` § "Unit index" vs. the edge block's `name:` set | PASS: identical 12-name sets | Confirms check #2 in the review brief |
| Component-partition check: 19 components in `components.md`'s YAML block vs. the "Components" lines per Unit in `unit-of-work.md` | PASS: every one of the 19 components (`ExtractFetcher` … `DataRightsService`) is assigned to exactly one Unit, enumerated by name, no open-ended bucket | Confirms check #3 |
| Component-edge-to-Unit-edge derivation check: every cross-unit component `depends_on` edge in `components.md` mapped forward and checked against the declared Unit edges | PASS (with one Minor exception, R-03): all 19 derived cross-unit component edges have a corresponding declared Unit edge; no missing edges found (the case that would let a Unit build against something not yet declared to exist) | Confirms check #4 — no missing dependency found; one redundant declared edge found (R-03) |
| Story/traceability check: 40 `### USx.y` headings in `stories.md` vs. 40 `upstream_ids`/`coverage` entries in `traceability.json`, cross-checked against the Unit column in `unit-of-work-story-map.md` | PASS: exact 1:1 match, every `OK`/`Deferred` target matches its story's row in the story map; sampled stories (US5.1, US9.2, US11.3, US13.1) checked against the target Unit's stated "Owns" responsibilities in `unit-of-work.md` and each is plausibly deliverable by its assigned Unit | Confirms check #6 |

### Summary

The graph is genuinely acyclic, the component partition and story traceability are complete and accurate by direct parsing, and the stage's no-sequencing constraint is respected (the "Depth of the graph" and "Parallel development opportunities" sections consistently disclaim critical-path/build-order status and Delivery Planning is named as the owner of that decision). The one substantive concern (R-01) is that the load-bearing "why the views must be one Unit" argument overstates its evidence for two of five named dependencies, though the grouping's conclusion survives independently through the `MapView` cycle, which checks out. Two further Minor inaccuracies (R-02, R-03) are cosmetic. None of the three findings changes the Unit boundaries, the DAG, or the traceability, so the artifacts are implementable as written.
