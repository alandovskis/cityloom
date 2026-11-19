# One road per block: merging connection junctions

Date: 2026-11-18. Status: draft for approval.

## Intent

osm2streets splits a street into separate roads wherever its OpenStreetMap way is split, and a way is split
wherever a tag changes partway along a block (lane count, speed limit, sidewalk) or just because a mapper split it.
osm2streets calls the point where two such pieces meet a `Connection`. The importer (`citymodel_builder`) turns
each piece into its own `Road` and each `Connection` into its own `Junction` with `junction: false`, so one block
between two real intersections can open in the street editor as two or more separate streets. A person designing a
block should design it as one street.

The importer will instead make one road per block: the pieces between two real ends become one road, each piece
kept as its own zone (its own lanes and speed limit) along that road.

## Goals

1. **Merge in the importer.** At a node where two roads meet and nothing else does, the two become one road
   when they continue the same street. Chains of such nodes collapse into one road from real end to real end.
2. **The editor reads the zones.** A merged road opens in the street editor as one street whose zones are the
   pieces, each with its own section and speed limit. (Today `Layout::from_network`,
   `cityloom_editor/src/city/import.rs`, reads only a road's first zone, so without this goal every piece after
   the first would be silently lost.)
3. **Discard all saved edits.** Every street and junction edit saved before this change stops applying, on every
   device and on the server, so no edit made on an old piece lands on a whole merged block. Decided by the user:
   all saved edits, not just those on the split pieces.

## Non-goals

- Name changes. Where the name changes at a split node (Rue A becomes Boulevard B), the two stay separate
  roads. They already share a corridor: `Network::derive_corridors` joins any roads that meet at a node and serve
  the same user class, and the importer derives corridors last. A test pins this; no corridor code changes.
- Pairing more divided roads. A dual carriageway split on one side only is not paired today and still is not.
- Migrating old edits onto merged streets (the user chose to discard them).
- Removing the legacy-width matching (`legacy_rows_mm`). It becomes unused for old records but stays.

## Anti-goals (a failure even if it works)

- A piece's lanes or speed limit lost or moved: a merged street must show each piece's cross-section where that
  piece was.
- An old edit applied to a merged street, or to anything else, after the change.
- A discarded edit coming back: pulled from the server, pushed by a tab still on the old build, or brought back
  by an archive.
- Merging across a real intersection, a fork, a dead end, the map's edge, or a node with signals or signs.

## The merge (citymodel_builder)

**Where:** a new step in `importer/mod.rs::read_network`, after `merge::merge_dual_carriageways` and before
`lay_out_lanes` and `derive_corridors`. It runs after the dual-carriageway merge because that merge refuses zoned
roads. It lives in its own module beside `merge.rs`, in the same style: pure over `&mut Network`, unit-tested
there.

**A node is merged away when all of these hold:**
- it is not a junction (`Junction::junction == false`: not osm2streets' `Intersection`);
- its control is `Control::None` (a mid-block signalled or signed crossing keeps its node);
- exactly two road ends meet at it, counting the roads left after unnamed roads were dropped and the
  carriageways merged;
- the two roads have the same name and the same `RoadType` (a road has only one type, so a change of class
  keeps two roads in one corridor).

**Chains:** a maximal run of roads joined by such nodes becomes one road. A run that closes on itself with no other
end (a ring of connections) is left alone.

**The merged road:**
- **Id and direction:** those of the run's lowest road id. Pieces running the other way are turned round first
  (points reversed, lanes reversed and their directions swapped, as `merge::reversed` does). Every piece is a
  single zone at this point, so turning one round is safe.
- **Ends:** the run's two outer nodes.
- **Centre line:** the pieces' points in order. Where one piece ends at the point the next begins, the duplicate
  point is dropped; where they differ, both are kept.
- **Zones:** one per piece, in order. Each starts at the length of the centre line before it (its first point's
  distance along the merged line), with the piece's lanes and `speed_kmh`. Neighbouring zones with equal lanes and
  equal speed become one, so a block split for no visible reason is one zone.
- **Provenance:** the union of the pieces' ways, with their versions.
- **Name, type:** the shared ones.

**Around it:** the merged-away nodes are removed from `junctions`. A priority control (`Control::Priority`) at
another node that names a piece names the merged road instead, once.

**Version:** `cityloom_model::IMPORTER_VERSION` goes from 10 to 11. The output changes for the same data. The
browser then reads kept areas again, and CI's tile cache key, which already hashes `src/importer/**`, rebuilds the
tiles.

**Downstream (already handles zones, to be confirmed by tests):** `cityloom_model::cut` (tiles and the browser's
`clip_to`) cuts zoned roads through `zones_between`; `Road::lay_out_lanes` lays out each zone.

## The editor reads zones (cityloom_editor `city/import.rs`)

`Layout::from_network` builds each street's base section from the first zone, as now. For a road of more than one
zone, it also fills the street's zones (`cityloom_model::street::Zone`):
- `from_end` and `length_mm` from `Layout::zone_frame` (see the anchor below);
- each later zone's `from_mm` is its start along the road, scaled to that length and measured from that end;
- each zone's `segments` are `pieces(zone.lanes)`, and its `speed` is `Speed::from_kmh(zone.speed_kmh)`.

My calls (builder check):
- **Anchor:** zones are measured from the end `Layout::zone_frame` anchors on: the first end when it is an OSM
  node, else the last. A road cut at both ends has no frame. It opens with its first zone only (known limit,
  written in `city/CLAUDE.md`).
- **Short zones:** a piece shorter than `MIN_ZONE_MM` (5 m) is still merged and imported as its own zone. That
  minimum governs splits a person makes, and folding the piece would lose its lanes. Tests confirm the editor
  shows and edits such a zone. Boundaries are rounded to whole millimetres, not snapped to 100 mm.
- **Width and the map (decided by the user):** the street's width (`row_mm`) is the first zone's, as an imported
  street's is today. A wider later zone is allowed to show as over that width. The map keeps drawing one band with
  the base section's lanes and marks where zones change; the street editor shows each zone. No map drawing changes.

The result goes through `settle_zones` so it is canonical: a section equal to the base is `None`, and a speed
equal to the street's is the street's. A one-zone road opens exactly as today. The imported street
(`today_streets`) carries the zones, so a street the person has not changed is not saved, and its zones show as
imported.

## Discarding saved edits

Saved edits are the street and junction records of `cityloom-edits` (per project, `cityloom-edits@<id>`), the
same records on the server under the account (`sync`, kinds `street` and `junction`), and the copies inside
archives and milestones.

**Mechanism:** a street or junction record says which import it was made against.
- Every record written from now on carries `"import": <IMPORTER_VERSION>` (the version its place came from).
- A record without the field, or with a number below 11, is stale. The cutoff is a fixed constant
  (`STALE_BEFORE_IMPORT = 11`), not the current `IMPORTER_VERSION`. Later importer changes do not wipe edits
  again; raising the constant is a deliberate decision. (Builder-check point; my call: the user asked for one
  discard, now.)
- A stale record is never applied to a place and never matched by a junction arm.
- **Locally:** on the first write of a project's edits document after the change, every stale record is
  removed. The removal is marked for the sync, so its tombstone goes up.
- **Sync:** a pulled stale record is not written into the document and is answered with a tombstone. A stale
  record pushed later by a tab on the old build is dropped the same way by every new build.
- **Archive:** a stale record in an archive being imported is not put back.
- **Milestones (decided by the user):** milestones are kept as history and can still be compared with now.
  Restoring a milestone that holds any stale street or junction record is refused with a new refusal
  (`Refused::Stale`, worded in `timeline/i18n` in both languages): it would only remove today's work.
- **Notice (decided by the user):** the person is told once, in English and French, that their street and junction
  edits were cleared because the map was rebuilt. No local copy is kept aside. My call on where: the discard
  records, per project, that it removed something (a flag in the project's storage). The next street, junction or
  map page opened in that project shows the notice once, through the shell's existing status area, and then clears
  the flag. A project with nothing stale shows nothing.
- The document's `version` gets a new number for the field (the next free one, per the rule in
  `city/CLAUDE.md`), so an older build leaves a new document alone instead of writing records back without the
  field.

**Not discarded:** restrictions (rules), the project list, the chosen area and the milestones themselves are not
street or junction edits.

## Versioning and docs

- Product version: minor bump (0.92.1 to 0.93.0), with `cargo update -p cityloom_editor`.
- Update `citymodel_builder/CLAUDE.md` (the new step), `city/CLAUDE.md` (zones from the import, stale records
  and the new document version) and `sync/CLAUDE.md` (stale records answered with tombstones).

## Testing (test-first, as the project requires)

- Builder unit tests: two pieces of one street merge into one road of two zones with the right starts, lanes and
  speeds; a chain of three; a piece pointing the other way; equal neighbouring pieces give one zone; a name
  change, a type change, a signalled connection, a fork, an intersection, a dead end and a ring are left alone;
  a priority control naming a piece names the merged road; provenance is the union; a network with nothing to
  merge is untouched; a large network merges quickly, like the existing twenty-thousand-roads test.
- Corridor test: two roads of different names meeting at a connection share a corridor.
- `cut` test: a merged zoned road cut by a tile keeps each piece's zone where it lies.
- Editor tests: a two-zone road opens as one street of two zones with their sections and speeds; a one-zone road
  opens as before; an unchanged zoned street is not saved.
- Discard tests: a stale record is not applied; the first write removes stale records and marks them deleted; a
  pulled stale record is dropped and tombstoned; a restored milestone and an imported archive bring no stale
  record back; a written record carries `import`; restoring a milestone holding a stale record is refused; the
  notice shows once in a project whose edits were discarded and never in one without; both `.ftl` files carry the
  new messages (the parity test).
- `citymodel_builder/tests/importer` (`real_city.rs`, `clip_agrees.rs`): they still pass, with any threshold
  that moves explained.
- Browser specs whose fixtures change (`just e2e-fixtures` makes them from the importer): run all; update an
  expectation only where it counted the pieces this change merges.

## Left to the builder

- The module name and how the chain walk is written, keeping it near-linear in the number of roads.
- How the editor measures the street's length for `length_mm`, as long as each zone starts where its piece
  starts on the drawn street.
- Where in `sync::local` and `city::edits` the stale check lives, as long as there is one rule
  (`is_stale(record)`) used everywhere.
