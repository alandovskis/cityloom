**Collaborator:** aidlc-developer-agent

## Contribution

I read the draft against the osm2streets source at `a-b-street/osm2streets` `main`
rather than from memory. Five facts from that source drive most of what follows,
so I state them once here and reference them below:

- **F1** — `osm2streets/src/road.rs`: `pub struct Road { pub id: RoadID, /// The
  original OSM ways making up this road. One road may consist of multiple ways
  pub osm_ids: Vec<osm::WayID>, pub src_i: IntersectionID, pub dst_i:
  IntersectionID, ... pub lane_specs_ltr: Vec<LaneSpec>, ... }`. `RoadID` is
  handed out from `road_id_counter` in `StreetNetwork`, so it is a build-order
  counter, not an identity.
- **F2** — `streets_reader/src/split_ways.rs` splits one OSM way into several
  `Road`s at shared points and "endpoints of every road", constructing each with
  `Road::new(id, vec![*osm_way_id], i1, *i2, pl, tags, &streets.config)`. So one
  way id maps to *many* roads, and (per F1's doc comment, and the
  `collapse_short_road` / `zip_sidepath` transforms) one road may carry *many*
  way ids. The relation is many-to-many in both directions.
- **F3** — `streets_reader/src/lib.rs`: `pub fn osm_to_street_network(input_bytes:
  &[u8], clip_pts: Option<Vec<LonLat>>, cfg: MapConfig, timer: &mut Timer) ->
  Result<(StreetNetwork, Document)>` — "Create a `StreetNetwork` from the contents
  of an `.osm.xml` or `.pbf` file. If `clip_pts` is specified, use these as a
  boundary polygon." There is no per-street entry point at this level. But
  `osm2lanes/src/algorithm.rs` exports `/// Purely from OSM tags, determine the
  lanes that a road segment has. pub fn get_lane_specs_ltr(tags: &Tags, cfg:
  &MapConfig) -> Vec<LaneSpec>`, re-exported from the `osm2streets` crate root.
  The cross-section is way-local; the geometry and network are not.
- **F4** — `osm2lanes/src/lib.rs`: `pub struct LaneSpec { pub lt: LaneType, pub
  dir: Direction, pub width: Distance, pub allowed_turns: EnumSet<TurnDirection>,
  pub lane: Option<Lane> }`, and in `algorithm.rs` the width is `let width =
  lane.width.map_or_else(|| LaneSpec::typical_lane_widths(lt, highway_tag)[0].0,
  distance_from_muv);`. Width is tagged-or-defaulted, and the `lane` field is the
  only thing distinguishing the two.
- **F5** — `MapConfig { driving_side, override_driving_side, country_code,
  bikes_can_use_bus_lanes, inferred_sidewalks, parallel_street_parking_spot_length,
  vehicle_width_for_parking_spots, turn_on_red, include_railroads, inferred_kerbs,
  date_time }`. `Road::total_width()` is `self.lane_specs_ltr.iter().map(|l|
  l.width).sum()` — every lane, `Sidewalk`, `Shoulder`, `Footway`, `SharedUse` and
  `Buffer(Verge)` included. Kerbs exist only as `Buffer(BufferType::Curb)`
  entries, and only when `inferred_kerbs` is on and the tagging carries them.

---

### 1. US5.2 / US7.3 and AC7.3.4 — the identity problem is real, and the draft
### under-states it in a way that will cost a rebuild

**The overlay key named in `team-practices.md` does not work as stated, and it
fails on imported lanes before it fails on user-added ones.**

The rule is "OSM way id plus a project-owned lane discriminator". Per F2, a way
id does not identify a road segment: a single named street is normally several
`Road`s, each carrying the same `osm_ids: vec![way_id]`. An overlay keyed on way
id alone applies an edit made on one block to every block of that way, or picks
one non-deterministically. After `collapse_short_road` or `zip_sidepath` the
reverse also holds — one `Road`, several way ids — so even "the way id of this
road" is not single-valued. The keying rule needs a road-segment component, and
`RoadID` cannot be it (F1: counter-allocated).

The fix that survives a re-import is the **ordered pair of OSM node ids bounding
the segment**, direction-normalised, alongside the way id — because those nodes
are exactly what `split_ways` splits on (F2), and node ids are stable OSM
identities in a way `RoadID` is not. That is a Domain Design decision, but the
stories should stop asserting a key that demonstrably does not distinguish two
blocks of the same street. `AC5.2.4` is right to forbid positional indices and
right to require a minted identifier; it is wrong to imply the way id half is
settled.

**The harder half is the one no AC covers: how an *imported* lane is identified
across a re-import.** `AC5.2.4` handles the user-added case, which is the easy
one — mint a ULID at creation, store it. But `AC7.3.1` (correct a value),
`AC7.3.3` (remove an invented lane) and `AC5.4.3` (revert to baseline) all need a
stable handle on a lane the *import* produced, and the only things available from
`LaneSpec` (F4) are its position in `lane_specs_ltr` — forbidden — or a
content-derived key such as `(lt, dir, ordinal among lanes of that type from the
left edge)`. No AC says which. A developer cannot build US7.3 without that
answer, and choosing wrong silently corrupts saved designs later.

**AC7.3.4 is not achievable as written, and implementing it literally would
re-create the exact failure `raid-log.md` R-2 exists to prevent.** "*Given* I have
corrected a street, *When* that same street is imported again later, *Then* my
correction is still applied" promises re-application unconditionally. Per F3, the
cross-section is a pure function of `(way tags, MapConfig)` — so a re-import
returns something different only when the tags changed, the `MapConfig` changed,
or the pinned osm2streets `rev` changed. In every one of those cases the lane the
correction attached to may no longer be the lane it corrected. Re-applying a
stale user width onto a lane OSM has since re-tagged presents a guess as a
correction to something it never corrected — the same class of harm as presenting
an inference as a measurement.

Say plainly what is achievable: **a correction can always be preserved; it cannot
always be re-applied.** I would split `AC7.3.4` into three, and I would not accept
the story without the third:

- *Given* a re-import whose `(tags, MapConfig, osm2streets rev)` fingerprint
  matches the one recorded when the correction was made, *When* it completes,
  *Then* the correction is re-applied without comment.
- *Given* a re-import where the fingerprint changed but the corrected lane still
  resolves by its stable key, *When* it completes, *Then* the correction is
  re-applied and I am told the underlying import changed.
- *Given* a re-import where the corrected lane no longer resolves, *When* it
  completes, *Then* the correction is retained and surfaced as unresolved, and is
  neither applied nor discarded.

The cost, stated so Delivery Planning can see it: the stored design must carry a
fingerprint of what was imported, a stable per-lane key for imported lanes, and a
UI state for unresolved corrections. That is a story of its own — see §3.

**AC5.2.3 is a much weaker test than it reads as.** "saved and reloaded" against
browser storage is satisfied by any serialisation that round-trips; it does not
exercise the identity problem at all. Only `AC7.3.4` does. As currently ordered a
builder can pass `AC5.2.3` and believe US5.2 is done.

**A user-added lane also needs an anchor, which no AC provides.** `AC5.2.1` says
"at a chosen position" and `AC5.2.3` says it "returns in the same position". If
the imported list around it changed on re-import, "the same position" has no
referent — and `AC5.2.4` has just forbidden the only thing that would give it one.
The added lane needs a stored anchor (left-of / right-of a keyed baseline lane, or
an offset from a named edge). Add an AC.

**US7.3 introduces a third layer, and US5.4 contradicts it.** `AC7.3.2` and
`AC7.3.3` say a correction changes "the corrected baseline", while
`team-practices.md` holds the imported baseline immutable behind `&`. So the model
is import → correction overlay → design overlay, not the two layers the rest of
the draft assumes. `AC5.4.3` then breaks: "*Then* it returns to the imported
baseline value with its original provenance" would discard a correction the user
made precisely because the imported value was wrong, and restore a `mapped`
provenance the user has explicitly contradicted. Revert must go to the *corrected*
baseline. Fix `AC5.4.3` and add an AC to `AC5.4.1` distinguishing the three
layers, or US5.4 and US7.3 cannot both be implemented.

**No AC says which layer an added lane lands in.** `AC5.2.1` (design) and
`AC7.3.2` (correction) describe the same mechanism with opposite meanings. The
user must choose, or a default must be stated. A developer cannot build add-lane
without it.

### 2. US3.1 vs US3.2 — the split is real, but the draft describes the wrong seam

The two-path split is sound and FR2.2 already establishes it: osm2streets cannot
serve a coarse viewport pass, because `osm_to_street_network` (F3) consumes a
whole `.osm.xml`/`.pbf` and returns a whole `StreetNetwork`. So far so good.

But `AC3.2.2`'s "only the street I select gets the full import" assumes a
per-street import that F3 does not offer at that level — you must fetch an area
extract and run the whole pipeline. That matters because **the extract boundary is
an input that changes the answer**: `split_ways` creates an intersection at every
road endpoint (F2), so a way clipped by the boundary gains an artificial
intersection at the clip line, and the area-level transformations
(`collapse_short_road`, `collapse_intersections`, `dual_carriageways`,
`parallel_sidepaths`) can merge or delete roads. The same street imported from two
different extracts can yield different `Road`s and different `osm_ids` groupings —
which is the mechanism that makes `AC7.3.4` fragile.

There is a better seam available and the ACs should not foreclose it: per F3,
`get_lane_specs_ltr(tags, cfg)` computes the cross-section "purely from OSM tags",
deterministically, for one way, with no extract boundary involved. The
cross-section — the thing the editor actually edits — is way-local. Only the
geometry and the connected-street graph need the area pipeline. That reading makes
`AC3.1.2`'s 10-second budget comfortable and makes `AC7.3.4` tractable. I am not
asking the stories to choose; I am asking them not to write ACs that assume the
expensive path is the only one.

**`AC3.1.4` and `AC4.1.3` are not reproducible as written.** A committed fixture
of "a well-tagged real street" plus expected lane count and order does not pin the
result, because the result depends on the whole of `MapConfig` (F5) —
`country_code`, `driving_side`/`override_driving_side`, `inferred_sidewalks`,
`inferred_kerbs`, `date_time`, `bikes_can_use_bus_lanes`, `include_railroads` —
and on the pinned osm2streets `rev`. `inferred_sidewalks` alone changes the lane
count. Add an AC: the fixture records the extract, the full `MapConfig` and the
pinned `rev`, and a change to any of them is a fixture change. Without this the
golden-fixture suite `team-practices.md` mandates will produce false failures the
builder learns to regenerate without reading — the exact habit that document
rejects snapshot tests to avoid.

**`AC3.1.2`'s 10 seconds is not yet a testable budget.** No extract size and no
device are named, and NFR4.6 puts phones in the matrix. Measured on the builder's
laptop it will always pass. Name the extract size and the reference device.

**Provenance is derivable, and the draft is right that it is — but only through
one specific field.** Per F4, `LaneSpec.width` is `lane.width` when muv parsed one
and `typical_lane_widths(lt, highway_tag)[0]` otherwise, and `lane: Option<Lane>`
is the only carrier of that distinction. So `AC4.1.1` and `AC4.1.3` are
implementable — the adapter maps "muv supplied a width" to `Mapped` and the
fallback to `Inferred` — but the mapping rests on inspecting an osm2streets/muv
type at the adapter boundary, and whether muv's own `lane.width` is itself
sometimes derived needs checking at implementation time rather than assuming.
Worth one line in US4.1 so the adapter's author knows where to look instead of
rediscovering it.

**A dependency nothing in the story set owns: where the selection gets a stable
OSM way identity from.** `AC2.2.1` selects a street and `AC3.1.1` imports "the
street I selected" — the handoff between them is an OSM way id, and the coarse
pass is what must supply it. A raster basemap carries no ids at all; vector tiles
may or may not carry usable OSM ids. D-4 is unassessed and OQ1 is unsettled. Every
overlay key in the product hangs off this. US2.2 needs an AC that selection yields
a stable OSM way identity.

### 3. Epics wearing a story's clothes

- **US6.1** — the draft calls it "Large but not splittable further without losing
  the value". That is wrong. It contains four separable pieces: discover connected
  streets, present them as a keyboard-reachable selectable set, apply a design to N
  targets, keep N baselines. `US6.1a` — "extend the design to one adjacent street I
  pick" — is a complete vertical slice, retires the actual risk, and is the story I
  would build first. `US6.1b` is corridor selection; `US6.1c` is bulk apply with a
  per-street outcome. More seriously, **`AC6.1.2` is not implementable as written**:
  "the design is applied to each chosen street" does not say what "apply" means when
  the target's lane list differs from the source's, which is the normal case. Is it
  a wholesale replacement of the target's lane list, or a per-lane mapping? That is
  the biggest unanswered design question in the set — bigger than the keying problem
  — and no AC touches it.
- **US6.1's declared dependency on US3.2 is wrong.** Connectivity comes from the
  network graph (`Road.src_i` / `Road.dst_i`, F1), which is the area pipeline — not
  from the coarse *display* pass US3.2 is about. If the coarse pass is a basemap
  tile layer it yields no connectivity whatsoever. **No story owns building or
  holding the connected-street graph**, and `AC6.1.1` assumes it exists.
- **US5.3** — "every editing action the product offers" is universally quantified
  over a set that grows with every later story, so `AC5.3.1` and `AC5.3.2` can never
  be closed. `team-practices.md` already treats this as a merge gate. Keep it as a
  gate and put a concrete keyboard AC on each editing story (US5.1, US5.2, US6.1,
  US7.3), which `AC5.3.3` already models well. As a story it is permanently open.
- **US7.3** — three mechanisms plus cross-import persistence. Split `AC7.3.4` out
  into its own story: it carries the fingerprint, the stable imported-lane key and
  the unresolved-correction state, none of which the other four ACs need.
- **US12.1** — `AC12.1.2` says output is "suitable for that purpose at the stated
  size or medium" and no size or medium is stated anywhere; OQ5 and OQ-US3 concede
  nobody has been asked. A developer must guess. One named purpose with one named
  artifact (a page size, an orientation, a scale, whether a scale bar is required)
  is the minimum that makes it implementable; the rest can wait for a real city.
- **US3.1** — honest about being the highest-risk story, but it bundles the OSM
  fetch (OQ1 unsettled, cost consequence, rate limits, failure modes) with the
  adapter and the render. The fetch deserves its own story: it is where the 10
  seconds goes and it is the piece with an open dependency.

### 4. Dependency graph and critical path — three defects

- **US1.2 is missing from the graph entirely**, despite declaring "Depends on:
  US2.1" in its body.
- **US1.1 is drawn as a prerequisite of US2.2**, contradicting its own body
  ("Independent of every editor story", "Depends on: nothing"). A landing page does
  not gate street selection.
- **US3.2 is drawn hanging off US3.1**, contradicting its own body ("Depends on:
  US2.1", "Independent of US3.1"). That is the one that matters, because the graph's
  version makes the coarse pass wait on the expensive import — the opposite of why
  the two were separated.
- **US7.3 has no path to US8.1.** `AC7.3.4` requires a correction to survive to a
  later import, which requires persistence. As drawn, US7.3 is buildable before any
  storage exists and `AC7.3.4` then cannot be satisfied.
- **US7.2 depends on US5.1, not just US7.1.** A blank cross-section you cannot edit
  is nothing.
- **The critical path understates B-0.** It claims US2.1 → US2.2 → US3.1 → US5.1 →
  US8.1 plus US4.1 "is exactly the walking skeleton B-0". `team-practices.md` also
  makes B-0's exit confirm that the pinned osm2streets build produces a working
  artifact end to end (R-1, D-1 — the highest-risk item in the project), and OQ1
  must be settled for any of it to run. Neither is a user story, fairly, but the
  sentence should not claim equivalence it does not have.

### 5. `traceability.json`

The `N/A` justifications are mostly honest — NFR5.1, NFR6.1, NFR7.1–7.3 are
genuinely engineering or repository practices with no user-observable behaviour,
and each names its owner. Three things I would change:

- **NFR4.1 is marked `Deferred → nfr-requirements`.** It is a committed conformance
  requirement covering the editing surface, and `team-practices.md` makes it a merge
  gate plus a manual screen-reader pass every release. Deferring it wholesale means
  no story carries the manual pass. It should at minimum name US5.3 and US2.3 as its
  partial home rather than reading as unowned.
- **NFR1.3 is marked `Deferred` while US3.2 lists NFR1.3 among its `Traces`.** One
  of the two is wrong.
- **FR2.2 → US3.2 drops half the requirement.** FR2.2 has two clauses — the coarse
  pass *and* "a full osm2streets pass on the street the user selects". The second is
  US3.1. Trace it to both.

The `reverse` section is honest where it matters: US8.2 and US8.3 genuinely have no
FR behind them and the `N/A` says so and points at OQ-US1. Listing US7.4 and US6.2
there as `OK` is noise, not a defect.

### 6. AC6.2.1 — the comparison is not well-defined against what osm2streets returns

Verified against F5: **osm2streets has no kerb-to-kerb width.** `Road::total_width()`
sums every `LaneSpec` including `Sidewalk`, `Shoulder`, `Footway`, `SharedUse` and
`Buffer(Verge)`. Kerbs appear only as `Buffer(BufferType::Curb)` entries, and only
when `MapConfig.inferred_kerbs` is on and the tagging carries kerb positions —
uncommon. Whether sidewalks are in the list at all is a `MapConfig` flag
(`inferred_sidewalks`). So "the target street's kerb-to-kerb width" names a quantity
the dependency does not return.

Three consequences, in order of severity:

1. **The AC needs an operational definition, and it must be project-owned.** The only
   defensible one available from the data: the sum of `LaneSpec.width` between the two
   `Buffer(Curb)` entries where both exist, and otherwise the sum excluding
   `LaneType::is_walkable()` types and `Buffer(Verge)`. Whichever is chosen, the AC
   must state it — otherwise two developers implement two different checks and the
   fixture suite cannot arbitrate.
2. **On a thinly-tagged street both sides of the comparison are defaults.** Per F4 the
   target's widths fall back to `typical_lane_widths`, so the check compares the
   design against a default table — it tests whether the design has more lanes than the
   table assumed, not whether the street physically fits. `AC6.2.3` treats this as a
   labelling problem ("the warning states that the comparison rests on an inferred
   width") when it is a "this check carries no information here" problem. Better: when
   the target's carriageway width is wholly inferred, say the fit **could not be
   checked** rather than warning about a comparison that means nothing. Warning on a
   default-vs-default comparison is presenting a guess as a finding — R-2 again.
3. **`AC6.2.5` claims more than width-only delivers.** `typical_lane_widths(lt,
   highway_tag)` is parameterised by `MapConfig.country_code` (F5), so a width-only
   check still imports a jurisdiction's default widths — through the adapter rather
   than through a compatibility table. That is compatible with the mandate, because the
   *core model* stays neutral, but the AC as phrased ("so no jurisdiction's
   classifications enter the core model") overstates it, and the fixture must pin
   `country_code` or the check is not reproducible.

Q4's choice of width-only remains the right call for a solo builder; the objection is
to the AC's precision, not to the decision.

### What the draft gets right

Briefly, because these are load-bearing and should not be re-litigated downstream:
`AC5.1.3` (`mapped` → `user-set`, never back) is exactly the right invariant and is
directly testable. `AC4.2.3` and `AC12.2.2` — omit the value rather than render it
unmarked — is the strongest thing in the document and the correct resolution of an
awkward case. `AC5.3.4` (activation target ≥ 44 × 44 CSS px, drawn width unchanged)
is precise and implementable whichever way OQ2 goes. `AC7.1.2` and `AC7.1.3` line up
exactly with the typed-error-boundary rule in `team-practices.md`. Separating US2.3
from US2.2 rather than burying keyboard selection in an AC is right. US8.2 —
browser storage vanishing without a user action — is a genuine catch that most
drafts of this kind miss entirely.

## Positions

AGREE: The two-path split behind US3.1 and US3.2 is real — `osm_to_street_network`
consumes a whole extract and cannot serve a viewport pass, so FR2.2's two layers are
forced by the dependency, not invented.
AGREE: `AC5.1.3`'s one-way `mapped` → `user-set` transition and `AC4.1.1`'s
exactly-one-provenance rule map cleanly onto the `Dimension`/`Provenance` type and are
implementable — `LaneSpec.lane: Option<Lane>` carries the tagged-vs-defaulted signal.
AGREE: `AC4.2.3` and `AC12.2.2` — omit rather than render unmarked — is the right
invariant and should survive downstream unchanged.
AGREE: `AC5.3.4`'s activation-target rule is precise and implementable under either
outcome of OQ2.
AGREE: US8.2 is correctly a story rather than an error dialog, and US2.3 is correctly
a story rather than an AC inside US2.2.
AGREE: Q4's width-only corridor check is the right call for one builder; my objection
below is to `AC6.2.1`'s precision, not to the decision.

OBJECT: `AC7.3.4` — "still applied" is not achievable in general and, implemented
literally, re-applies a stale correction onto a lane OSM has since re-tagged; split it
into fingerprint-match / key-resolves / unresolved, where the third preserves the
correction without applying it.
OBJECT: `AC5.2.4` and the keying rule it rests on — an OSM way id does not identify a
road segment (`split_ways` gives every block of a way the same `osm_ids`), so the key
needs a segment component such as the bounding node-id pair; `RoadID` cannot serve, it
is counter-allocated.
OBJECT: US7.3 and US5.4 — no AC says how an *imported* lane is identified across a
re-import, which `AC7.3.1`, `AC7.3.3` and `AC5.4.3` all require; `AC5.2.4` only covers
the user-added case.
OBJECT: `AC5.4.3` — reverting to "the imported baseline value with its original
provenance" discards a US7.3 correction and restores a `mapped` value the user has
explicitly contradicted; revert must target the corrected baseline.
OBJECT: US5.2 and US7.3 — no AC states whether an added lane lands in the correction
layer or the design layer, and the two stories describe the same mechanism with
opposite meanings.
OBJECT: `AC5.2.1` and `AC5.2.3` — "the same position" has no referent once positional
indices are forbidden; a user-added lane needs a stored anchor.
OBJECT: `AC3.1.4` and `AC4.1.3` — a fixture is not reproducible without pinning the
full `MapConfig` (`inferred_sidewalks` alone changes the lane count) and the
osm2streets `rev`.
OBJECT: `AC3.1.2` — 10 seconds with no stated extract size and no reference device is
not a testable budget while NFR4.6 puts phones in the matrix.
OBJECT: `AC6.2.1` — osm2streets returns no kerb-to-kerb width; `total_width()` includes
sidewalks, footways and verges, and kerbs exist only as `Buffer(Curb)` when
`inferred_kerbs` is on. The AC must state a project-owned definition.
OBJECT: `AC6.2.3` — on a thinly-tagged target both sides of the comparison are
`typical_lane_widths` defaults; the honest outcome is "fit could not be checked", not a
warning about an inferred width.
OBJECT: `AC6.2.5` — width-only still imports country-parameterised default widths via
`MapConfig.country_code`; the claim that no jurisdiction's classifications enter is
overstated as phrased.
OBJECT: US6.1 — it is an epic, `AC6.1.2` does not define what "apply" means when the
target's lane list differs from the source's, and its declared dependency on US3.2 is
wrong: connectivity comes from the network graph, not the coarse display pass.
OBJECT: No story owns building or holding the connected-street graph that `AC6.1.1`
assumes.
OBJECT: US2.2 — no AC requires selection to yield a stable OSM way identity, which
every overlay key in the product depends on and which OQ1/D-4 leave unsettled.
OBJECT: US5.3 — `AC5.3.1` and `AC5.3.2` are universally quantified over a growing set
and can never be closed; it is a merge gate, not a story.
OBJECT: `AC12.1.2` — "suitable for that purpose at the stated size or medium" states no
size or medium anywhere; unimplementable without one named artifact.
OBJECT: The dependency graph contradicts three story bodies — US1.2 is absent, US1.1 is
drawn as a prerequisite of US2.2, and US3.2 is drawn hanging off US3.1.
OBJECT: US7.3 has no path to US8.1 in the graph, so `AC7.3.4` is unsatisfiable in the
order drawn; US7.2 also depends on US5.1, not only US7.1.
OBJECT: The critical path claims equivalence with B-0 while omitting the pinned
osm2streets build (R-1, D-1) and OQ1's unsettled data source.
OBJECT: `traceability.json` — NFR4.1 marked `Deferred` leaves the manual screen-reader
pass unowned; NFR1.3 is `Deferred` while US3.2 claims to trace it; FR2.2 traces only to
US3.2 and drops its second clause.
