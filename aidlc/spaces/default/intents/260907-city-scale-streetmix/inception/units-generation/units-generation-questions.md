# Units Generation — Decomposition Questions

Upstream inputs: `components.md` and `decisions.md` (domain-design),
`requirements.md` (requirements-analysis), `stories.md` (user-stories),
`team-practices.md` (practices-discovery).

**What this stage decides:** how the 19 components in the catalogue group into
independently implementable Units of Work, and which Unit can depend on which.
It describes *what can depend on what*. It does **not** decide what to build
first — that is Delivery Planning's decision, and no question here asks about
build order, value, or risk sequencing.

**What is already settled and not re-asked here:** the editing surface is real
page elements with canvas only for the map (Q2 at domain-design); OpenStreetMap
data comes through a Railway-hosted proxy (Q3, Q8); the corridor correspondence
rule (Q4); anonymous upload identity (Q5); that a public link needs no account
(Q6); and that corrections and design edits are two separate overlays (Q7).

**The shape you are decomposing.** The catalogue's five layers, from
`components.md` Part B:

| Layer | Components | Deployable |
|---|---|---|
| Adapter | `ExtractFetcher`, `StreetImportAdapter` | Rust/WASM client |
| Core | `StreetModel`, `CorrectionOverlay`, `DesignOverlay`, `EditingSession`, `CorridorPlanner` | Rust/WASM client |
| Outer | `LocalDesignStore`, `UploadClient`, `MapView`, `CrossSectionView`, `ExportComposer`, `AppShell` | Rust/WASM client |
| Entry | `CompositionRoot` | Rust/WASM client |
| Server | `OsmExtractProxy`, `DesignRepository`, `AccountService`, `SharingService`, `DataRightsService` | Railway service(s) |

40 user stories: 28 in product Stage 1, 9 in Stage 2, 3 in Stage 3.

---

## Q1. What should a Unit boundary follow?

`team.md`'s Code Style already mandates three inward-pointing layers **enforced
by Cargo crate boundaries**, so that an inward-pointing violation is a compile
error rather than a review comment. That makes crate boundaries the one
decomposition the build itself checks. Feature slices, by contrast, are how the
stories are written and how a person would describe the product.

A. **Follow the crate/layer boundaries.** One Unit per architectural layer of
   the client (adapter, core, outer ring, entry), plus the server tier. The
   decomposition the compiler already enforces becomes the decomposition the
   plan uses, so there is exactly one boundary story.

B. **Follow feature slices.** Units named for what they deliver — import,
   edit, corridor, local persistence, upload, accounts, sharing, rights,
   output — each cutting vertically through the layers. Every Unit ships
   something a person can see.

C. **Follow the three product stages.** One Unit per stage boundary in
   `scope-document.md`: Stage 1 client, Stage 2 accounts/sharing/rights,
   Stage 3 output. Very few Units, aligned to the public-release gate.

D. **Hybrid — crate boundaries below, feature slices above.** The adapter and
   core become their own Units because `team.md` makes those boundaries
   compile-enforced and they are shared by everything; the outer ring and the
   server tier decompose by feature, because nothing enforces a boundary there
   and the stories are the better guide.

X. Other (please specify)

[Answer]: B

---

## Q2. How many Units, given that each one multiplies the process ahead of it?

This is the question with the largest hidden cost, so the cost is stated
plainly. On the default walk, four Construction design stages plus Code
Generation run **once per Unit**: Functional Design, NFR Requirements, NFR
Design, Infrastructure Design, Code Generation. `team.md` also sets
Construction Autonomy Mode to **gate every Bolt through Stage 1**, so those
runs are gated on your own time, not a reviewer's. Six Units means roughly 30
per-Unit stage runs; twelve means roughly 60.

Against that: a Unit is the isolation boundary. Finer Units mean smaller
blast radius per change and a Unit that can be finished and verified on its
own.

A. **Coarse — around 5 or 6 Units.** Fewest gates, largest Units. Each Unit
   spans several components, and a Unit's Definition of Done covers a lot of
   ground at once.

B. **Medium — around 8 to 10 Units.** A middle setting: the shared foundations
   are separate from the feature surfaces, and the Stage 2 work is not one
   monolith.

C. **Fine — around 12 to 15 Units.** Close to one Unit per component cluster.
   Smallest increments and the tightest verification boundary, at the cost of
   roughly double the per-Unit stage runs of option A.

X. Other (please specify)

[Answer]: B

---

## Q3. Does the server tier become one Unit or several?

The five server components do not share a schedule. `OsmExtractProxy` is
needed for the walking skeleton B-0 — nothing imports without it. But
`DesignRepository` is also needed in product Stage 1, because US8.3 ("keep a
design that is not tied to one device") is a Stage 1 Must Have that uploads to
it. `AccountService`, `SharingService` and `DataRightsService` are Stage 2 and
sit behind the access gate `project.md` mandates.

A. **One server Unit.** All five components in a single deployable Unit. One
   Railway service, one deployment story, one set of infrastructure design.

B. **Two server Units — the proxy, and everything that stores a design.**
   `OsmExtractProxy` is separated because it holds no user data, has a
   privacy obligation of its own (`decisions.md` ADR-004: it must log neither
   addresses nor requested locations), and is needed before anything else
   exists. The other four share a database and ship together.

C. **Three server Units — proxy, design storage, and Stage 2 identity.**
   `OsmExtractProxy`; `DesignRepository` (Stage 1 anonymous uploads); and
   `AccountService` + `SharingService` + `DataRightsService` behind the access
   gate. The Stage 1 and Stage 2 server work stop being entangled.

D. **Four or more.** Split the Stage 2 identity components too, so erasure and
   export are their own Unit.

X. Other (please specify)

[Answer]: C

---

## Q4. Should data subject rights be a Unit of their own?

`project.md` mandates that erasure, export, access and rectification are
functional requirements **gating Stage 2's public release**, never deferred
operational work — and that the accounts and sharing surface stays behind a
server-side access gate until erasure and export exist. `components.md` makes
`DataRightsService` a separate component for exactly this reason: "a release
gate rather than a feature".

The question is whether that separation survives into the Unit plan, where it
determines whether the gate can be verified independently of the surface it
gates.

A. **Yes — `DataRightsService` is its own Unit.** The release gate has its own
   Definition of Done and can be finished and verified before the accounts and
   sharing surface is opened. The dependency is explicit in the graph rather
   than living in a checklist.

B. **No — fold it in with accounts.** Erasure deletes accounts and their
   designs, so it touches the same data and the same database migrations;
   splitting them means two Units editing adjacent schema.

C. **Rights and the access gate together, separate from the sharing surface.**
   One Unit owns both the server-side gate flag and erasure/export, so the
   thing that holds the door and the thing that unlocks it ship as one.

X. Other (please specify)

[Answer]: A

---

## Q5. Is the stored-design payload format its own Unit?

One serialised shape — a design, its lane edits, its corrections, and every
value's provenance — is read and written by four different components:
`LocalDesignStore` (device storage), `UploadClient` (upload), and on the
server `DesignRepository` (storage) and `DataRightsService` (export, which
AC11.2.3 requires to keep every inferred value identifiable as inferred). It
crosses the client/server line and the Rust/database line.

Units Generation can tag a Unit with a `kind`, and one of the available kinds
is `spec` — a contract or schema consumed in place rather than a deployed
executable.

A. **Yes — a `spec` Unit.** The payload format is defined once, in its own
   Unit, and the four consumers depend on it. Contract Design (the next stage)
   formalises it. A change to the shape has one owner and one place to break.

B. **No — the client core owns it.** `DesignOverlay` and `CorrectionOverlay`
   already own the types; serialisation is just their `serde` derivation, and
   the server stores whatever the client sends.

C. **No, but Contract Design must formalise the boundary anyway.** No separate
   Unit, but the client/server payload is named as a contract in 2.8 so it is
   not left implicit.

X. Other (please specify)

[Answer]: A

---

## Q6. Where does the pinned osm2streets build live in the Unit plan?

`project.md` mandates pinning any osm2streets Cargo dependency to a specific
commit, with its `abstutil` git dependency pinned the same way. `team.md` adds
that the upstream commit, the build command and the toolchain versions
(`rustc`, `wasm-pack`/`trunk`) go in **one committed file**, and states the
pass/fail test for the practice: "can this be rebuilt in six months without
reconstructing what was done." `raid-log.md` records osm2streets as the
project's single Critical dependency.

Units Generation offers a `packaging` kind — build and distribution artefacts,
which owe no business-logic model.

A. **Its own `packaging` Unit.** The fork, the pin, the committed provenance
   file and the WASM build configuration are one Unit that everything else
   depends on. The project's single Critical dependency has a named owner in
   the plan rather than being a line item inside another Unit.

B. **Part of the adapter Unit.** `StreetImportAdapter` is the only component
   permitted to depend on osm2streets, so the pin belongs with it. One fewer
   Unit, and the dependency and its only consumer stay together.

C. **Part of a broader workspace/build Unit** that also covers the `justfile`,
   `scripts/verify.sh`, the CI tiers and the asset-and-dependency manifest —
   all the build-gate infrastructure `team.md` requires.

X. Other (please specify)

[Answer]: A

---

## Q7. How does the client reach the browser, in deployment terms?

`team.md` commits to one environment, deploy on merge to `main`, on Railway,
with no staging tier. The client is a Rust/WASM bundle; the server components
are a Rust service. `team.md` also flags that which Railway builder can build
a Rust workspace to a WASM artifact — a Nixpacks Rust provider versus a
project-supplied `Dockerfile` — is confirmed at environment-provisioning
rather than assumed.

This affects Unit boundaries because it decides whether the client is a
deployable Unit in its own right or an artefact another Unit serves.

A. **One Railway service serves both.** The server binary serves the built
   WASM bundle as static files. One deployment, one domain, no cross-origin
   configuration, and the ~$5/month budget covers one service.

B. **Two deployables — a static client and an API service.** The client is
   deployed as static hosting and the server is its own service. Cleaner
   separation, but two deploy targets and a cross-origin story, against a
   budget `project.md` forbids growing without explicit justification.

C. **Defer — record both as viable and let Infrastructure Design settle it.**
   The Unit plan does not depend on the answer; only the deployment model
   field per Unit does, and it can say "confirmed at environment-provisioning".

X. Other (please specify)

[Answer]: A

---

# Follow-up questions

Cross-checking the seven answers against each other found one literal
contradiction, one arithmetic conflict, and one gap. Recorded here rather than
resolved by assumption.

## Q8. Q3 and Q4 disagree about the server Unit count

Q3's answer (C) puts `AccountService`, `SharingService` **and**
`DataRightsService` together in one Stage 2 identity Unit — three server Units
in total. Q4's answer (A) makes `DataRightsService` its own Unit, which is
exactly Q3's option D — four server Units. Both cannot hold.

A. **Q4 governs — four server Units.** `OsmExtractProxy`; design storage;
   `AccountService` + `SharingService`; `DataRightsService`. Q3's stated
   reason for C — that Stage 1 and Stage 2 server work stop being entangled —
   is fully preserved by this, and Q4's reason is a `project.md` mandate: the
   release gate must be verifiable independently of the surface it gates.

B. **Q3 governs — three server Units.** `DataRightsService` stays inside the
   Stage 2 identity Unit, and the release-gate discipline is carried by that
   Unit's Definition of Done rather than by a Unit boundary. Fewer Units, but
   the gate and the thing it gates share a completion criterion.

X. Other (please specify)

[Answer]: A

---

## Q9. The Unit count implied by Q3–Q6 exceeds the band chosen at Q2

Q2 chose 8 to 10 Units. The later answers each create a Unit on their own
merits: a `packaging` Unit for the pinned osm2streets build (Q6), a `spec`
Unit for the stored-design payload (Q5), and three or four server Units
(Q3/Q4/Q8). That is 5 or 6 Units before a single client feature slice exists.
The client still has to cover 28 Stage 1 stories and 3 Stage 3 stories —
import, edit, corridor, local persistence, upload, the shell and status
region, and export — which is 6 or 7 more. The total lands around 12 or 13,
not 8 to 10.

A. **Keep every boundary and accept 12 to 13 Units.** The band was a guide;
   each separation since was chosen for a stated reason. Accept the higher
   per-Unit process cost that Q2 spelled out.

B. **Hold the band by merging client feature slices.** Keep the `spec`,
   `packaging`, server and rights boundaries exactly as answered, and combine
   the client slices instead — corridor work folds into the editing Unit,
   upload folds into local persistence, and the shell and status region folds
   into whichever Unit owns the editing surface. Lands at 9 or 10.

C. **Hold the band by dropping one of the later boundaries.** Fold the payload
   `spec` back into the core (Q5's option C) and/or the `packaging` Unit into
   the adapter (Q6's option B), keeping the client slices fine-grained.

X. Other (please specify)

[Answer]: B

---

## Q10. Under feature slicing, where do the core's shared foundations live?

Q1 chose feature slices, which cut vertically through the layers. But
`StreetModel` is not a feature: it owns `Street`, `Lane`, `StreetNetworkGraph`,
the `Dimension`/`Provenance` types, the derived lane key, the project-wide
carriageway-width definition and the `StreetSource` port — and every feature
slice depends on all of it. Pure feature slicing leaves nowhere for it to live.

A. **A foundation Unit.** The core model becomes its own `library` Unit that
   every feature slice depends on. This departs from pure feature slicing for
   exactly one Unit, for the same reason the payload spec and the osm2streets
   build got their own: it is shared by everything and owned by no feature.

B. **The import slice creates it.** The first slice that needs the types
   defines them and later slices extend them. No extra Unit, but the import
   Unit becomes much larger than any other and every remaining Unit depends
   on it — so the graph has the same shape as option A without naming it.

X. Other (please specify)

[Answer]: A


---

## Q11. Follow-up — feature slicing produces a cyclic Unit graph, and the DAG must be acyclic

Drawing the dependency graph from the confirmed answers produced cycles. This
is a hard failure, not a preference: `unit-of-work-dependency.md` must carry a
cycle-free edge block, and the `required-sections` sensor checks it at this
stage's gate.

**Why it happens.** Feature slices cut vertically, but the three presentation
components each read from *every* model layer while no model layer reads back:

- `MapView` — nominally part of finding and importing a street — depends on
  `CorridorPlanner` and `DesignOverlay`, which belong to editing. Editing's
  `EditingSession` and `DesignOverlay` in turn depend on `CorrectionOverlay`,
  which belongs to import. That closes a loop between the import slice and the
  editing slice.
- `AppShell` depends on local persistence, upload, export, accounts and
  sharing — every one of which depends back on the editing model. Folding the
  shell into the editing Unit, as Q9's option B directed, closes a loop with
  each of them.

Views are sinks. Any Unit that pairs a view with one feature's model draws an
edge into every other feature's model, and those features draw edges back.

**None of the arrangements below reaches the 8–10 band.** With four server
Units (Q8) and three foundation Units (Q5, Q6, Q10) already fixed, twelve is
the floor. The band is no longer reachable; only which twelve remains open.

A. **Views and shell as one Unit — 12 Units.** Client: `street-import`
   (fetcher, adapter, corrections), `design-editing` (design overlay, editing
   session, corridor), `client-surfaces` (map, cross-section, shell, entry),
   `local-persistence` (store, upload), `meeting-output` (export). Feature
   slicing survives for the model work; the presentation surface becomes one
   Unit precisely because every view reads every layer. `client-surfaces` is a
   clean sink — nothing depends on it.

B. **Finer views — 13 Units.** As A, but the map surface, the cross-section
   surface and the shell are three separate Units. Finest verification
   boundary and the smallest blast radius per change, at the cost of one more
   Unit's worth of gates.

C. **Switch to the hybrid basis — 12 Units.** Q1's option D, chosen now that
   the cycle is visible: the client's compile-enforced crate layers become
   Units (the whole core crate as one Unit, the adapter as one), and the outer
   ring and server tier decompose by feature. Cycles are impossible by
   construction, because the crate layers are already inward-pointing and
   acyclic. The cost is that no client Unit below the outer ring delivers a
   visible feature on its own.

X. Other (please specify)

[Answer]: A

---

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
