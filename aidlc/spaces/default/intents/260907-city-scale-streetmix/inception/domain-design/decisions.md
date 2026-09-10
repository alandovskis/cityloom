# Architecture Decision Record — Streetmix at City Scale

Decisions taken at Domain Design. The Rationale table in `components.md` gives a
one-line justification per component; this is the durable log of the choices
that had alternatives.

---

## ADR-001: Proceed with a design that seven approved statements contradict

**Status**: Accepted · **Date**: 2026-09-10

### Context

`stories.md` records five amendments required of approved artifacts, each marked
"before Domain Design". None landed. This stage's own decisions added two more.
All seven are cases where an approved artifact now says something the design
does not do:

| # | Approved artifact says | Design does | Origin |
|---|---|---|---|
| 1 | Nothing about undo | US5.5 is a Stage 1 Must Have, realised by `EditingSession` | user-stories |
| 2 | Nothing about device-storage loss | US8.2 is a Stage 1 Must Have, realised by `LocalDesignStore` | user-stories |
| 3 | Nothing about opt-in upload | US8.3 is a Stage 1 Must Have, realised by `UploadClient` | user-stories |
| 4 | FR9.3: "Anonymous designs shall expire 30 days after creation" | Only *uploaded* designs expire; the product never holds the others | user-stories |
| 5 | `scope-document.md` places all server-side storage in Stage 2 | Stage 1 has server-side upload and an OSM proxy | user-stories |
| 6 | AC3.1.7: "no request is made to the application's own server" during a full import | Every import goes through the Railway proxy | this stage, Q3 |
| 7 | NFR5.2: per-user computation runs in the browser so cost does not scale with usage | Computation still does; the *fetch* does not, and NFR5.2 was what AC3.1.7 made observable | this stage, Q3 |

The problem is not that the design is wrong. It is that `requirements.md` and
`scope-document.md` are what every later stage reads, and this file is not.

### Decision

Proceed, and cover the three orphan stories explicitly in `components.md` and
`traceability.json`, so the design is complete even though the requirements are
not. Record all seven divergences here, in one place, as the standing record of
where the approved artifacts and the real design differ.

### Consequences

**Positive.** No stage re-runs. The design is complete against the stories,
which are the more recent and more specific statement of intent. A single
table names every divergence rather than leaving them scattered.

**Negative.** `requirements.md` remains wrong in four ways and
`scope-document.md` in one, and a later stage that trusts either without reading
this ADR will design against a stale picture. Divergence 4 is the sharpest:
FR9.3 as approved asserts an obligation over data the product never receives,
which is not merely incomplete but false.

**Neutral.** The amendments remain outstanding work. They are cheaper to apply
now than after Construction begins.

### Alternatives Rejected

- **Amend `requirements.md` and `scope-document.md` first.** Correct, and the
  option the human was offered. Rejected on cost: both stages are approved and
  closed, so it means jumping back through two stages and their review cycles to
  fix documents that no code has yet been written against.
- **Proceed and record the divergence only, without covering the orphans.**
  Rejected because it leaves three Stage 1 Must Haves with neither a requirement
  nor a component, which would push the same gap into Units Generation.

---

## ADR-002: Separate the correction overlay from the design overlay

**Status**: Accepted · **Date**: 2026-09-10

### Context

The model has three layers: the imported baseline, corrections to it, and the
proposed design. `team.md` requires the baseline to be immutable with edits in
an overlay, but does not say whether corrections and design edits are one
overlay or two.

They differ in what they claim. A correction says *the import was wrong* — it is
permanent, its provenance is `user-set` and never returns to `mapped`, it
carries a fingerprint of what was imported, and a re-import can leave it
unresolved. A design edit says *I propose a change to this street* — it makes no
claim about reality, so it cannot go stale against OpenStreetMap and needs no
fingerprint.

### Decision

Two separate components, `CorrectionOverlay` and `DesignOverlay`, with distinct
entities. The fingerprint and the three re-import outcomes live only on
corrections.

### Consequences

**Positive.** The distinction between a claim about the street and a proposal
about it is carried by the type system rather than by a field, which matches
`team.md`'s own principle that the un-annotated form should be unconstructable.
A bug cannot silently convert a proposal into a claim, which is the failure
class the whole provenance model exists to prevent.

**Negative.** Two overlays to key, serialise and merge. Three components in the
catalogue are thin as a result.

**Neutral.** Revert semantics now span both overlays: reverting a design edit
returns to the corrected baseline where a correction exists and the imported
baseline otherwise, carrying that value's own provenance.

### Alternatives Rejected

- **One overlay with a per-entry layer tag.** Simpler storage and one key
  scheme. Rejected because the fingerprint logic would filter by tag, so the
  distinction rests on a field a bug can mislabel — and because the
  reversibility is asymmetric: merging two overlays later is a local change,
  while splitting a merged one after designs have been stored requires a
  migration.

---

## ADR-003: Real page elements for the editing surface, canvas only for the map

**Status**: Accepted · **Date**: 2026-09-10

### Context

Three stages deferred this decision here. `team-practices.md` establishes that
Rust compiled to WebAssembly does not force a canvas — Leptos, Yew and Dioxus
all render real DOM — and states the cost of choosing one: axe-core treats
`<canvas>` as opaque, so a canvas editing surface makes the lane strip's focus,
names, states and activation targets unverifiable unless the canvas exposes an
accessibility tree *and* queryable hit-test regions.

The sharpest consequence is on the walking skeleton. B-0's third pass criterion
is that mapped values are visibly distinguished from inferred ones *in what is
rendered*. Under canvas without an accessibility tree there is no automated
route to that at all, and it would fall to a manual screen-reader walkthrough on
the very first Bolt and every release after.

### Decision

The lane strip and every editing control are real page elements. The map is a
canvas, because that is what a basemap library is and it carries no editing
affordances.

### Consequences

**Positive.** Each lane is individually focusable and announceable, so AC4.1.4,
AC4.1.5 and AC5.3.4 are automatable, every canvas-conditional row in
`accessibility-checklist.md` resolves to its DOM branch, and **B-0's third
criterion is testable from the first Bolt.** `requirements.md` OQ2 and
`stories.md` OQ-US7 both close.

**Negative.** Drawing a to-scale cross-section from DOM elements is more
constrained than drawing it on a canvas, particularly for a lane a few pixels
wide. The activation-target rule already anticipates this: the target is
enlarged beyond the drawn extent, and the drawing is never widened, because a
rendered lane width is data.

**Neutral.** The map remains a surface nothing can inspect, so
`accessibility-checklist.md` M5 binds rather than being precautionary: every
map-only affordance needs a twin in the document flow.

### Alternatives Rejected

- **Canvas throughout**, with a commitment to expose an accessibility tree and
  queryable hit-test regions. Rejected because the commitment is load-bearing
  and unverifiable until built, and its failure mode is discovering after B-0
  that the walking skeleton's own criterion cannot be tested.
- **Real elements throughout, including the map.** Rejected as unrealistic: a
  basemap at city scale is a tile-rendering problem, and no accessibility
  benefit accrues to a surface whose affordances all have document-flow twins.

---

## ADR-004: Fetch OpenStreetMap extracts through a proxy that logs nothing about requesters

**Status**: Accepted · **Date**: 2026-09-10

### Context

`requirements.md` OQ1 left the fetch path open with a stated cost consequence:
routed through the Railway service, egress bills at $0.05 per GB against a ~$5
monthly budget; fetched by the browser directly, it costs nothing. At the city
scale NFR2.1 sets, OQ1 calls this "the difference between a negligible and a
budget-breaking bill".

Two constraints bear on it. `project.md` carries an affirmed prohibition:
hosting spend must not grow past ~$5 a month without being treated as a
constraint change requiring explicit justification. And `scope-document.md`'s
claim that Stage 1 carries no privacy obligation rests on Stage 1 holding
nothing about anybody — which the browser-first storage decision at user-stories
was chosen to preserve.

A proxy fetching extracts on a user's behalf necessarily observes their IP
address and the place they are looking at. A request log pairing those two is
location data about an identifiable device.

### Decision

Fetch through a Railway-hosted proxy. The proxy **logs neither IP addresses nor
requested locations, and never a pair of the two**, and caches keyed on the
extract rather than on the requester. Its egress cost is unmeasured; B-0 records
a real figure for one street import, and Infrastructure Design revisits the
budget with that number.

### Consequences

**Positive.** Control over caching and over politeness toward the upstream
public API, which a browser-direct design cannot coordinate. The no-logging rule
keeps Stage 1's privacy position intact on the surface the proxy introduces.

**Negative.** AC3.1.7 is false by design and NFR5.2 needs restating — both
recorded in ADR-001. The budget question is deferred rather than answered, and
the affirmed prohibition means it must be answered before spend grows, not
after. Debugging is harder when the logs deliberately omit who asked for what.

**Neutral.** The proxy is the only server-side component in Stage 1, which makes
it the only Stage 1 surface with a privacy obligation at all.

### Alternatives Rejected

- **Browser fetches directly from a public OpenStreetMap API.** Costs nothing
  and keeps AC3.1.7 true by construction. Rejected by the human at Q3 in favour
  of the control a proxy gives; recorded here because it remains the fallback if
  B-0's measurement makes the proxy unaffordable.
- **Browser-direct with a server cache only for extracts that already failed or
  proved expensive.** Bounds egress by construction. Rejected as two paths to
  build and test for a benefit the measurement may show is unnecessary.
- **Proxy with full request logging.** Rejected outright: it would reopen on a
  new surface exactly the Stage 1 privacy question the storage decision closed.

---

## ADR-005: Match corridor changes by lane type and ordinal from the kerb

**Status**: Accepted · **Date**: 2026-09-10

### Context

A corridor apply is per-lane mapping rather than wholesale replacement, so the
target keeps what the design does not speak to. The rule that performs the
matching was left to this stage. Elm has six lanes and its bike lane was
widened; Oak has four, arranged differently. Something has to decide which of
Oak's lanes receives the change, and AC6.1.3 requires the product to report what
it could not match.

### Decision

Match by lane type and ordinal from the kerb, counting from the same edge the
carriageway-width definition uses. Report every change that finds no match.

### Consequences

**Positive.** Deterministic, testable against the committed fixtures, and
explicable to a user in one sentence. It introduces no new domain vocabulary,
so nothing about a jurisdiction's lane classifications enters the core model.

**Negative.** It matches positionally, so a street whose lane order differs
structurally — a cycleway on the far side, say — produces an unmatched change
rather than a sensible one. That is reported rather than guessed, which is the
right failure, but it is a failure the user has to resolve by hand.

**Neutral.** It depends on `StreetModel` owning the definition of which edge is
the kerb, which it already does for the fit check. Two definitions of the kerb
in one product would be a bug waiting for a thin street.

### Alternatives Rejected

- **Match by role within the cross-section**, expressing changes as intent —
  "the bike lane nearest the kerb becomes 2.5 m". More forgiving on dissimilar
  streets. Rejected because it needs a role vocabulary the core model does not
  have, and inventing one is exactly where a jurisdiction's assumptions enter.
- **Match by type only**, applying a change to every lane of that type.
  Rejected: it widens both bike lanes when the user meant one, and it is not
  obviously wrong on screen, which is the worst combination.

---

## ADR-006: A browser-minted random identifier, and no edit token, for anonymous uploads

**Status**: Accepted · **Date**: 2026-09-10

### Context

AC8.3.5 requires an uploaded anonymous design to be stored against an identifier
carrying no name, no contact details and no cross-site linkage. Refined Mockups
declined to specify the mechanism and named this stage.

### Decision

A 128-bit random identifier from a cryptographic source, minted in the browser
at upload, derived from nothing about the design or the device, and held in
device storage so the design can be reached and removed again. No edit token.
`DesignRepository` rate-limits access by identifier.

### Consequences

**Positive.** Nothing links two uploads by the same person; there is no account,
no cookie and no server-assigned identity. Removal needs no account, which it
must not, because the upload needed none.

**Negative.** The identifier is the only protection the design has, so anyone
holding it can delete the design. That is acceptable only because it is
unguessable and the delete path is rate-limited — both of which are therefore
requirements rather than implementation details.

**Neutral.** Losing the identifier means losing the ability to remove the
design; it will expire at 30 days regardless.

### Alternatives Rejected

- **Identifier plus a server-issued edit token** returned once at upload.
  Stronger against a guessed identifier. Rejected because it is a second secret
  the user can lose, and losing it means an undeletable design — which is worse
  for a data-rights posture than the guessing risk it removes.

---

## ADR-008: The street network graph is a core type the adapter constructs

**Status**: Accepted · **Date**: 2026-09-10 · **Supersedes part of the first
draft of ADR-005's component placement**

### Context

The corridor work needs street connectivity, which comes from the network
topology osm2streets produces — not from the coarse display pass, which on a
basemap tile layer yields no connectivity at all.

The first draft of this catalogue had `CorridorPlanner` read the network graph
from `StreetImportAdapter`. At the component level that graph was acyclic, and
the catalogue's validator confirmed it. But `team.md` enforces layering through
Cargo crate boundaries, and `CorridorPlanner` is core while
`StreetImportAdapter` is the adapter. The adapter already depends on the core to
construct the types it returns. A core component calling back into the adapter
therefore describes two crates that depend on each other, which Cargo cannot
build.

The catalogue also claimed, under its diagram, that the acyclic shape was "what
the affirmed inward-pointing rule requires". That claim was false: a DAG at the
component level says nothing about whether the crate boundaries hold.

### Decision

`StreetNetworkGraph` is an entity owned by `StreetModel` — a core type.
`StreetImportAdapter` constructs it at import time, alongside the per-street
cross-section. `CorridorPlanner` and `MapView` read it from the core and never
call the adapter. The outer ring wires the adapter to the core, which is the
only layer permitted to depend across both boundaries.

The catalogue's validation now checks edge direction against the three layers as
well as checking for cycles.

### Consequences

**Positive.** The crate graph is buildable, and the layering `team.md` mandates
is enforced by the same mechanism it chose — a violation is a compile error.
Nothing in the core names the adapter, which is the property that makes the core
testable without osm2streets present.

**Negative.** The core now owns a type whose only producer is the adapter, so
the core's public API is shaped partly by what the adapter can supply. That is
the ordinary cost of a ports-and-adapters boundary, and it is the direction the
affirmed rule already chose.

**Neutral.** `AppShell` gains an edge to `StreetImportAdapter`, because with the
reverse edge removed something in the outer ring has to trigger an import.
**Superseded by ADR-009**, which found that edge to be wider than `team.md`
grants the outer ring, and moved it to a dedicated composition root.

### Alternatives Rejected

- **Keep the graph as an adapter type and let the core read it.** What the first
  draft did. Rejected because it does not compile under the affirmed crate
  boundaries — not a style preference but a build failure.
- **Give the graph its own crate between the adapter and the core.** Removes the
  cycle without moving the type. Rejected as a fourth layer introduced to avoid
  a decision, when the graph is plainly domain data and belongs in the domain.

---

## ADR-007: A named grant requires an account; a public link does not

**Status**: Accepted · **Date**: 2026-09-10

### Context

`stories.md` OQ-US5 asked whether someone a design is shared with needs an
account to view it. It lands on P2, the city planner, whose stated barrier is
being unwilling to sign up for anything to read one proposal — and P2 is the
persona whose acceptance is the product's differentiator.

### Decision

A public link opens a design read-only for anyone holding it, with no account. A
named grant requires the recipient to hold an account.

### Consequences

**Positive.** Named access becomes genuine authorisation rather than a shared
secret, which is what FR8.2's two independent mechanisms are for. The
account-free path still exists, so P2 remains reachable through a link.

**Negative.** `SharingService` now depends on `AccountService` to resolve a
grantee, and US10.2's named path depends on US9.1 in a way the story dependency
graph does not currently draw. A named grant to someone without an account
requires them to create one before they can read anything.

**Neutral.** The two mechanisms remain independent in both directions, as
AC10.2.3 requires; this decision changes who can use one of them, not how they
interact.

### Alternatives Rejected

- **No account to view anything.** Zero friction for P2 on both paths. Rejected
  because a named grant would then be a shared secret rather than authorisation,
  and the difference between "people I invite" and "anyone with the link" would
  be nominal — which is the very confusion the S7 wireframe correction was made
  to remove.

---

## ADR-009: A core-owned import port, wired by the entry crate

**Status**: Accepted · **Date**: 2026-09-10 · **Supersedes the "Neutral"
paragraph of ADR-008**

### Context

ADR-008 removed a crate cycle by making `StreetNetworkGraph` a core type. With
the core no longer calling the adapter, something else had to trigger an import,
and that draft gave the edge to `AppShell`. The architecture review's R-05 found
that this compiles, is not a cycle, and is still wrong: `team.md`'s Code Style
grants the outer ring an inward dependency "on the core crate's public API", and
the adapter crate is not the core. The practical cost is that `AppShell` — which
owns routing, the live status region and the cross-component focus rules, so it
is a component with a great deal worth testing — could not be exercised without
the adapter crate, and therefore without the pinned osm2streets dependency,
compiled in.

The obvious repair is a port: declare the shape of "import a street" in the core
and have the adapter implement it. On its own that is not enough, and the reason
matters. A trait narrows the API that crosses the boundary but does not remove
the Cargo edge, because some component must still construct the concrete
`StreetImportAdapter`. If `AppShell` constructs it, `AppShell` still declares the
adapter crate in its `Cargo.toml` and nothing has been gained. The edge only
disappears when construction moves to a component that does nothing else — and a
WebAssembly build already has exactly one such component, the entry point that
mounts the application.

### Decision

`StreetModel` owns a `StreetSource` port describing "import a street".
`StreetImportAdapter` implements it. A new component, `CompositionRoot` — the
WebAssembly entry point — constructs the concrete adapter, supplies it as the
core's `StreetSource`, and mounts `AppShell`. `AppShell` depends on the core
port, never on the adapter.

`CompositionRoot` is the single outer component permitted to name a concrete
adapter type. It holds no product behaviour and owns no entity, and that is the
point: the exception is parked in the one place where it costs nothing to test
around.

The catalogue's layer check now enforces this directly — an outer-ring edge into
the adapter fails unless its source is `CompositionRoot`. The layer table also
gains a Server tier, so the five server-side components are no longer folded into
the outer ring; a rule about Cargo crate boundaries has nothing to say about a
separate deployable, and saying so is cheaper than re-deriving it later.

### Consequences

**Positive.** Every component with behaviour worth testing now depends inward on
the core's public API only, which is `team.md`'s rule stated literally rather
than stretched. The port is also a test seam: everything above import can run
against a fake `StreetSource` with osm2streets not compiled in at all. Under the
affirmed TDD posture, with the adapter characterised by golden fixtures at
exactly this boundary, that seam is leverage rather than ceremony.

**Negative.** One more component, one more indirection, and a trait in the core
that the core itself never calls — it exists for its implementors and its
callers. The dependency injection is real machinery a direct call would not have
needed.

**Neutral.** The component count moves from 18 to 19. `CompositionRoot` is not a
boundary choice in the sense Option A and Option B were; it is the entry point
every build needs, promoted to a named component because R-05 made its role
load-bearing.

### Alternatives Rejected

- **Record the direct `AppShell` → adapter edge as a deliberate composition-root
  pattern, and widen `team.md`'s wording to match.** The cheaper option, and the
  one the reviewer offered first. Rejected because it widens an affirmed practice
  to fit a draft rather than the reverse, and because it leaves the coupling in
  the component that has the most behaviour to test — spending the rule's
  credibility and the test seam together.
- **A port with no composition root, `AppShell` constructing the adapter
  itself.** Rejected because it does not work: the Cargo edge survives the trait,
  so this buys the indirection without the property it was introduced for.
- **A fourth crate between the outer ring and the adapter to hold the wiring.**
  Rejected as the same fourth-layer evasion ADR-008 already declined. The
  WebAssembly entry point is not a new layer; it exists whether or not it is
  named here.
