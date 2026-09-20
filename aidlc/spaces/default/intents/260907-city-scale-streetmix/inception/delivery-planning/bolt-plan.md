# Bolt Plan — Streetmix at City Scale

Upstream inputs: `requirements.md` (requirements-analysis), `stories.md`
(user-stories), `mockups.md` (refined-mockups), `components.md`
(domain-design), `unit-of-work.md`, `unit-of-work-dependency.md` and
`unit-of-work-story-map.md` (units-generation), `contract-summary.md`
(contract-design), `team-practices.md` (practices-discovery).

This is the order the work gets built, as a sequence of **Bolts** — a Bolt
being one build pass over a piece of the work, ending in something that runs.
Each entry names what it includes, what has to be true for it to count as done,
and what shipping it will tell us that we do not know yet.

Twelve Bolts. The first is the **walking skeleton**: a thin end-to-end slice
that proves the architecture hangs together before any features are built on
it. The eleven after it each complete one Unit of Work, except the last of them,
which completes two for a reason given in its entry.

## How this order was chosen

Units Generation produced the dependency graph, which says what *can* come
before what. It does not say what *should*, and cannot: that is a judgement
about what is worth proving first. The judgement here, from Q2, is
**value-first following the product stages** — finish product Stage 1 (a
usable single-street and corridor editor) before Stage 3 output or Stage 2
accounts, wherever the graph allows it. `scope-document.md` already staged the
product that way; this makes the build order follow a decision already taken
rather than inventing a second one. The full argument is in
`risk-and-sequencing-rationale.md`.

**Where value-first had to give way.** Two Stage 1 intentions could not be
honoured, and the graph is why:

- `client-surfaces` — the map, the cross-section and the shell, all of
  product Stage 1 — is the single sink of the dependency graph. `AppShell`
  presents the export surface and the sharing surface, so the Unit holding it
  cannot be finished until Stage 3's `meeting-output` and Stage 2's
  `accounts-sharing` exist. It is therefore last, and Stage 3 and Stage 2 work
  lands before the Stage 1 surface that presents them.
- `design-storage` is a Stage 1 Unit because US8.3 ("keep a design that is not
  tied to one device") is a Stage 1 Must Have, and `local-persistence` depends
  on it. So the server storage lands before the device storage that uploads to
  it.

## The sequence

| Bolt | Completes | Product stage | Depends on |
|---|---|---|---|
| B-0 | *(nothing — thin slice through seven Units)* | 1 | — |
| B-1 | U1 `osm2streets-build` | 1 | — |
| B-2 | U2 `street-core` | 1 | — |
| B-3 | U9 `osm-extract-proxy` | 1 | — |
| B-4 | U4 `street-import` | 1 | B-1, B-2, B-3 |
| B-5 | U3 `design-payload-spec` | 1 | B-2 |
| B-6 | U5 `design-editing` | 1 | B-2, B-4 |
| B-7 | U10 `design-storage` | 1 | B-5 |
| B-8 | U7 `local-persistence` | 1 | B-2, B-4, B-5, B-6, B-7 |
| B-9 | U8 `meeting-output` | 3 | B-2, B-4, B-6 |
| B-10 | U11 `accounts-sharing`, then U12 `data-rights` | 2 | B-7 |
| B-11 | U6 `client-surfaces` | 1 | B-2, B-4, B-6, B-8, B-9, B-10 |

**This order was checked against the graph, not against itself.** The twelve
Unit names and their `depends_on` lists were parsed out of
`unit-of-work-dependency.md`'s edge block and every Bolt's position compared
against every dependency's position. A first draft of this table placed
`local-persistence` before `design-storage`, and `client-surfaces` before
`meeting-output` and `accounts-sharing` — three Bolts scheduled ahead of work
they need. The check found all three; reading the table had not.

**Product Stage 1 is usable at B-11**, not before: the editor has no surface
until `client-surfaces` is complete. B-0 is what makes that bearable — it puts
a working end-to-end path on screen at the very start, in thin form, so the
eleven Bolts after it are filling in something that already runs rather than
building toward a first demo at the end.

---

## B-0 — Walking skeleton: one street, imported, edited, saved

**Includes.** Partial work in seven Units — U1 `osm2streets-build`,
U2 `street-core`, U9 `osm-extract-proxy`, U4 `street-import`,
U5 `design-editing`, U6 `client-surfaces`, U7 `local-persistence`. It
completes none of them; each is finished by its own Bolt later.

**This is the walking skeleton** (Q1), and `team-practices.md` makes it solo
and gated: it is approved explicitly before any other Bolt runs.
`initiative-brief.md` calls it the one thing that would reverse the Go
recommendation if it fails.

**Definition of Done.** All three of the following hold at once — the criterion
`team-practices.md` sets, where any one alone is not a pass:

1. A named real street imports through the pinned osm2streets build with the
   correct lane count and order.
2. An edit is made, saved, reloaded, and comes back identical.
3. Mapped values are visibly distinguished from inferred ones in what is
   rendered.

Plus two the same practice attaches: the pinned build produces a working
artifact end to end, and — added by Q6 — **one basemap tile service is proven
to supply stable OpenStreetMap way identities**, or proven not to.

**Confidence hypothesis.** That osm2streets returns usable lane geometry for
real streets, that provenance survives the whole path from adapter to screen,
and that a street can be identified on a map well enough to select it. A no to
any of those changes the design rather than the schedule. `raid-log.md` records
osm2streets as the project's single Critical dependency (D-1, R-1); this is
where that stops being a risk and becomes a fact either way.

**Expected demo.** Open the app, find one named street on a map, see its lanes
with measured and guessed widths visibly different, change one, reload, see
the change still there.

**Implementation note.** Every part is thin on purpose. One street, not a
corridor. One import path, no failure taxonomy. Storage that round-trips, not
the index or the storage-unavailable handling. The point is the path, not the
coverage.

---

## B-1 — `osm2streets-build`

**Includes.** U1 `osm2streets-build` (packaging).

**Definition of Done.** The fork is pinned to a commit SHA with `abstutil`
pinned the same way, `Cargo.lock` is committed, and one committed file records
the upstream commit, the build command and the toolchain versions. The
pass/fail test `team-practices.md` states directly: this can be rebuilt in six
months without reconstructing what was done. The golden-fixture suite is
authored here and runs green.

**Confidence hypothesis.** That the dependency is reproducible — and that the
fixture suite goes red when upstream moves, which is its whole purpose.

**Expected demo.** A clean checkout builds the pinned dependency; deliberately
bumping the pin turns the fixture suite red.

---

## B-2 — `street-core`

**Includes.** U2 `street-core` (library).

**Definition of Done.** `Street`, `Lane` and `StreetNetworkGraph` exist with
provenance carried at the type level, where the un-annotated form is
unconstructable. The derived lane key, the full overlay key (way id plus
direction-normalised bounding node-id pair plus lane discriminator), the
carriageway-width definition, bounds-checked lane access, and the
`StreetSource` port are all in place. Metres are the only unit. Coverage meets
the 80% floor on this crate.

**Confidence hypothesis.** That provenance-at-the-type-level is workable rather
than merely stated — that it does not force awkward code at every call site.
This is the rule `raid-log.md` R-2 rests on, so it needs to survive contact
with real use.

**Expected demo.** Tests showing a dimension cannot be constructed without its
provenance, and that a correction resolves to the same lane across two
independent imports of the same street.

---

## B-3 — `osm-extract-proxy`

**Includes.** U9 `osm-extract-proxy` (service).

**Definition of Done.** The proxy fetches and caches extracts keyed on the
extract itself, rate-limits its own traffic toward the public OpenStreetMap
API, and logs neither addresses nor requested locations nor a pair of the two.
Contract 1 in `contract-summary.md` is implemented as specified. **Its egress
cost is measured against one real street** and recorded — `team-practices.md`
requires the figure at B-0 and `project.md` treats growth past the ~$5/month
budget as a constraint change rather than something to absorb.

**Confidence hypothesis.** That proxying OpenStreetMap fetches through Railway
is affordable at all. This is the open question `contract-summary.md` carries
and the one that could reopen the fetch design.

**Expected demo.** A fetch through the proxy, a cache hit on the second, and
the recorded egress figure for one street.

---

## B-4 — `street-import`

**Includes.** U4 `street-import` (library) — the fetcher, the adapter, and
corrections.

**Definition of Done.** The adapter is the only code depending on the
osm2streets crate, enforced by the crate boundary. Every osm2streets error and
any panic it cannot avoid maps into the project's closed typed-failure set,
with no native error type reaching a caller. Corrections carry an import
fingerprint and resolve to exactly one of the three re-import outcomes. The
committed fixture set covers a well-tagged street, a thinly-tagged street, a
one-way, a street with a separately mapped cycleway, and one intersection.

**Confidence hypothesis.** That the three re-import outcomes are the right
three — that a real changed import lands in one of them rather than somewhere
unanticipated. This is the hardest behaviour in product Stage 1 and the one
with the least precedent.

**Expected demo.** Each fixture importing correctly; a correction surviving an
unchanged re-import silently, surviving a changed one with a notice, and going
unresolved when its lane disappears.

---

## B-5 — `design-payload-spec`

**Includes.** U3 `design-payload-spec` (spec).

**Definition of Done.** Contract 3 in `contract-summary.md` is implemented as
the shared Rust types: the design, its edits, its corrections, every
provenance state, and `payloadVersion` written from this first release. A
reader that meets an unrecognised version fails in the stated, handled way.

**Confidence hypothesis.** That one shape genuinely serves all four consumers —
device storage, upload, server storage and export — without either the server
or client side needing a variant of it.

**Expected demo.** A round-trip test: a design with mapped, inferred and
user-set values serialises, deserialises, and compares equal.

---

## B-6 — `design-editing`

**Includes.** U5 `design-editing` (library) — the overlay, the editing state
machine, and corridor work.

**Definition of Done.** Every action on the committed editing-action list
applies synchronously with no network request. Undo reverses the most recent
action, and reverses a corridor apply across every street it touched as one
unit. Fit is assessed on total width alone with no lane-type compatibility
rule, reporting four states rather than two. A bulk apply is not atomic:
streets that succeed keep the design and those that fail are named. Coverage
meets the floor.

**Confidence hypothesis.** That matching a design onto a structurally
different street by lane type and ordinal from the kerb produces useful results
often enough — and that when it does not, reporting the unmatched change is
better than guessing. `raid-log.md` and the corridor decision both rest on
this, and it is the product's central differentiating claim.

**Expected demo.** A design extended along a corridor of three streets: one
fitting, one not fitting with the shortfall named, one with a change that could
not be matched and is reported as such.

---

## B-7 — `design-storage`

**Includes.** U10 `design-storage` (service).

**Definition of Done.** Contract 2 in `contract-summary.md` is implemented.
An anonymous design expires 30 days after creation. An ungranted request
returns not-found or forbidden and never the design, asserted as a response
rather than a UI state. Access is rate-limited by identifier so an anonymous
identifier cannot be found by guessing.

**Confidence hypothesis.** That an unguessable identifier is adequate
protection for an anonymous uploaded design — the only protection it has.

**Expected demo.** An upload, a fetch by identifier, a rejected fetch for a
wrong identifier returning no design, and the rate limit engaging under
repeated guesses.

**Why a server Unit sits this early.** US8.3 is a product Stage 1 Must Have and
`local-persistence` depends on this Unit to upload into. Stage 1 is not
device-only.

---

## B-8 — `local-persistence`

**Includes.** U7 `local-persistence` (ui) — device storage and the upload
affordance.

**Definition of Done.** Designs and corrections persist and reload identically
— lane order, types, widths, every provenance state. More than one design is
held, with an index. Storage being unavailable is detected at load rather than
on first edit, and a save that fails mid-session surfaces at the moment it
fails. A first-time visitor with empty storage is not told anything was lost.
Upload is explicit, mints its 128-bit identifier from a cryptographic source,
and removal needs no account.

**Confidence hypothesis.** That browser-first persistence is good enough to be
the product's primary Stage 1 storage — that the failure modes are handleable
rather than merely handled.

**Expected demo.** Two designs saved, the browser closed and reopened, both
back identically; storage disabled and the app saying so rather than failing;
one design uploaded and then removed without an account.

---

## B-9 — `meeting-output` · product Stage 3

**Includes.** U8 `meeting-output` (ui).

**Definition of Done.** Output is chosen by purpose rather than file type. The
print purpose produces a single page at a named size and orientation with a
stated scale. Output identifies the street and its location and states the
before-and-after relationship. Every inferred value's marking is carried
through at least one non-colour channel, and any value whose marking cannot be
carried is omitted rather than shown unmarked.

**Confidence hypothesis.** That a proposal leaves this tool in a form someone
can actually take to a public meeting — the product's stated purpose, and the
thing no earlier Bolt tests.

**Expected demo.** A before-and-after sheet for one edited street, printed,
with the guessed values still legible as guesses on paper.

**Why Stage 3 lands before the Stage 1 surface.** `AppShell` presents the
export surface, so `client-surfaces` cannot be finished until this Unit
exists. The graph forces it; value-first would have put it last.

---

## B-10 — `accounts-sharing`, then `data-rights` · product Stage 2

**Includes.** U11 `accounts-sharing` then U12 `data-rights`, in that order.
Two Units in one Bolt because the second gates the first: `project.md`
mandates that the accounts and sharing surface stays behind a server-side
access gate until erasure and export exist, so shipping U11 without U12 would
be shipping a surface that cannot be opened. Splitting them would produce a
Bolt whose Definition of Done is "built, but unreachable."

**Definition of Done.** Accounts and sessions work, with cookies carrying
`HttpOnly`, `Secure` and a `SameSite` policy. A design migrates from device
storage on sign-up with every edit and provenance state preserved. Named
grants and link access are independent in both directions. Erasure removes an
account with its personal data and its designs, states what will be removed
before it happens, and never leaves a half-deleted account. Export is
machine-readable and keeps every inferred value identifiable as inferred.
**The access gate is only then opened**, and CodeQL default setup is enabled
at the start of this Bolt.

**Confidence hypothesis.** That data subject rights are deliverable as
functional requirements rather than operational work — which is what
`project.md` asserts and nothing has yet tested.

**Expected demo.** Sign up, migrate a device design, share it by link and by
named grant, then delete the account and watch every one of those become
not-found.

**Note.** This Bolt also applies the two `data-rights` amendments carried from
Contract Design (Q5) — the missing `components.md` dependency and the two
missing Unit-graph edges.

---

## B-11 — `client-surfaces` · product Stage 1 is usable here

**Includes.** U6 `client-surfaces` (ui) — the map, the cross-section, the
shell, and the WebAssembly entry point.

**Definition of Done.** Each lane is an individually focusable, announceable
element showing provenance in three channels at once. Every editing action is
operable by keyboard alone. The live region announces all 23 committed status
messages without moving focus. The map carries its four overlays, evicts
full-import records as they leave the viewport, and every map-only affordance
has a twin in the document flow. The surface works at 360 CSS pixels with a
44-pixel activation target floor. The automated accessibility suite and the
keyboard-path tests are green, and the manual screen-reader walkthrough of the
full edit path has been done.

**Confidence hypothesis.** That WCAG 2.1 AA on an editing surface is
achievable as designed — that real page elements for the lane strip deliver
the automated verification the canvas alternative would have cost.
`team-practices.md` is explicit that a green pipeline is not conformance, which
is why the manual pass is in the Definition of Done rather than beside it.

**Expected demo.** The full product Stage 1 path driven entirely by keyboard,
then again with a screen reader.

**Note.** This is the largest Unit in the set — eleven stories and the whole
accessibility surface — and `unit-of-work.md` records that as a consequence of
the acyclicity constraint rather than an estimate. It is last because it is the
graph's single sink, not because it is least valuable; it is the opposite.

## Before Construction starts

Two things happen before B-0, from Q5 and Q6:

1. **Three missing requirements are added to `requirements.md`** — for undo
   (US5.5), for a design no longer on this device (US8.2), and for keeping a
   design off this device (US8.3). All three are product Stage 1 Must Haves
   that Construction will build from, and all three currently trace to no
   requirement. FR9.3's expiry wording is amended at the same time to bind
   uploaded designs only.
2. **The basemap spike** is carried inside B-0 rather than deferred to
   Infrastructure Design, because a tile service that cannot supply stable
   OpenStreetMap way identities leaves US2.2 with no mechanism.

The remaining amendments — the `DataRightsService` dependency and the two Unit
graph edges — are carried to B-10, where the Unit that needs them is built.
