# Changing a street along its length, without zones (design)

Date: 2026-11-18. Scope: the street page (`street/`), and the wording that names zones elsewhere (map, junction,
report, timeline, the user docs). The data model (`cityloom_model::street::Zone` and what reads it) stays.

## Why

A zone is how the app stores "from here on, the street has this cross-section and this speed limit". It is an
implementation detail, but the street page puts it in front of the person: a "Zones along the street" strip, Split,
Merge, Name, Reset and "Use this section for the whole street" commands, a Zone level in the details panel's path
(Street › Zone › piece), "Zone 2" in announcements and checks, "3 zones" on the map. To change half a block, someone
has to learn the concept first: split, pick the zone, then edit.

What the person wants is to change the street along a block, seamlessly: point at part of the block, change it, and
the street changes there.

## Goals

- No zone concept anywhere the person sees or hears it: no zone strip or zone commands, no Zone level, no "Zone n" or
  zone names in wording, announcements, checks, labels or the map.
- Everything zones enable today still works: a different cross-section and speed limit per stretch, transitions and
  their checks, the 2.5D joint across a boundary, junction arms reading the street's end, the map's marks and bands,
  the School street template's school stretch.
- A new top-down plan of the whole street, laid out horizontally and drawn to scale, where the person picks a stretch
  by dragging along it.
- Boundaries appear and disappear by themselves as the person edits.

## Non-goals

- Changing the saved form. `Street.zones`, `Zone { from_mm, segments, name, speed }`, `from_end`, `length_mm`, the
  record versions and the sync stay as they are. A saved zone name is kept and round-trips; it is no longer shown.
- Changing what the street is checked against, the transition rules, the taper lengths or the 2.5D drawing itself
  (other than which stretch it shows).
- The junction page's layout and the map's drawing (only their wording changes).
- A plan view on the junction page or the map.

## Anti-goals (a failure even if it works)

- A person meeting the word "zone" (or "Zone 2", or a zone's name) for what is a stretch of street. "School zone" and
  "loading zone" are real-world terms and stay.
- Losing keyboard or screen-reader access to anything the replaced rows give today (choosing, adding, moving,
  resizing and removing curbside uses, stops, calming and green elements; moving a boundary).
- An edit that silently changes a part of the street the person did not pick.
- Slivers: a stretch shorter than the 5 m minimum appearing as a side effect of a pick or an edit.
- A street saved before this change looking or behaving differently once opened (beyond the hidden wording).

## Words

Throughout this document and the new wording:

- **Stretch**: the part of the street the person picked: a start and an end along the street.
- **Run**: a maximal part of the street that looks the same: the same pieces (ignoring uids, as `same_pieces` does)
  and the same speed limit. Internally a run is one or more adjacent zones. Saved streets may hold identical
  neighbouring zones (a Split with no edit after it, or two named zones), and they are not joined on load (see
  **Joining**), so code must not assume one zone per run. No boundary is drawn between zones of the same run.

"Run" is internal; the person sees only "stretch" and positions ("from 49 m to 82 m").

**Positions** (my call): the plan always draws the first end on the left, and every position the person sees or types
counts from the plan's left end, whichever end the street is anchored from (`from_end`). Internally positions stay
measured from the anchor, converted at the view-model's edge. A pick's "start" is its left end; Left/Right on the
keyboard move left/right on the plan; a whole-street pick shows the leftmost run. The "Measured from the first/second
end" caption (`st-zones-from`) goes.

## The page (decided: layout A)

Top to bottom in the column (`.stage-scroll`):

1. The 2.5D drawing, as today. It shows the picked stretch: its first 40 m (`STRETCH_MM`), with the joint across a
   boundary when one is near, exactly as it shows the selected zone today.
2. The piece list (`#pieces-slot`), as today: the pieces of the stretch (see **What the stretch shows**).
3. **The plan** (new), in place of the zone strip, the curbside rows, the stops rows, the calming strip and the green
   strip. Above it, the street's ends ("0 m" and its length, measured from the anchored end as `st-zones-from` says
   today); under it, the picked stretch: "From 49 m to 82 m · 33 m long".

The details panel on the right is unchanged apart from the levels (see **Details panel**).

## The plan

A horizontal, to-scale, top-down drawing of the whole street (all of `length_mm`), first end on the left. Each piece is
a band across its width, in each run's own order and widths, so a change along the street shows where it happens.
Drawn in place along the street:

- curbside uses (parking, loading, racks, timed lanes as a hatch or label) on their side;
- transit stops (curb stop, curb extension, island) on their side;
- traffic calming: raised tables and crossings across the roadway, chicane build-outs, modal filters;
- green elements: trees, beds, rain gardens, bioswales on their host piece;
- a boundary between runs, with the transition's taper drawn at its true length (it is schematic in the 2.5D only);
- an end cut by the area's edge marked as cut. This is new: nothing on the street page shows a cut end today, so it
  needs a new mark and a new message. (`zone-cut-off` is something else: a zone starting beyond the loaded end of an
  elsewhere street; see **Elsewhere**.)

The plan is a pure function returning an SVG string, like the 2.5D (`street/view/plan/`, set with `inner_html`), with
its press targets as a hit layer; pointer and keyboard logic are pure state machines with tests (`keys::`), as the
strips' are today. It fits the column's width; a long street is drawn smaller rather than scrolled sideways, down to a
minimum scale at which it scrolls inside itself (the builder picks that minimum; the zone strip's 24 px cell floor is a
precedent). Colours come from the existing material and kind tokens; any new text or control colour pair goes in
`shared/contrast.rs`.

### Picking a stretch

- **Drag** along the plan, on anything that is not an object (an empty part of a band): picks from where the drag
  started to where it ended, snapped to 100 mm (`snap`). The pick is shown as a tinted band with a handle at each end.
- **Click** (a press without a drag): picks the run under the pointer, the whole run that looks the same.
- **Drag a pick's handle**: changes that end of the pick.
- **Drag a boundary** between runs (when nothing is picked across it, or on its own marker): moves the boundary, one
  undoable edit, kept 5 m from its neighbours (`strip::pair_boundary_mm` today). Refused as today while another drag
  is under way or the street is **elsewhere**.
- **The whole street**: dragging the full length, or choosing Street in the details panel's path, picks the whole
  street. At first (page open) the whole street is picked.
- **Snapping to avoid slivers**: a pick end within 5 m (`MIN_ZONE_MM`) of a street end or an existing boundary snaps to
  it. A pick shorter than 5 m is not made (the previous pick stays, and the reason is said).
- The pick is outside history, like the zone selection today: undo, redo and Start over only clamp it.

### Moving objects

A press on a curbside use, stop, calming element or green element chooses it (its level opens in the details panel, as
the strips do today); a drag on it moves it, with exactly the rules and refusals the rows have today (one undoable
step, Escape cancels, only into a free stretch for curbside uses and stops, drags refuse each other). Adding stays in
the Add a piece menu, at the start of the picked stretch where today it is "at the start of the chosen free stretch"
(the builder maps "chosen free stretch" onto the pick: the free stretch on the relevant side under the pick's start).

### Keyboard and screen reader

The plan replaces listboxes that are fully keyboard-operable today, so it must be too. My call on the shape, the
builder may refine it:

- The plan is one tab stop for the street itself: Left/Right move a point along the street (1 m, Shift 10 m, Home/End
  to the ends), Shift+Left/Right extend the pick from where it started, Enter picks the run at the point.
- Each side's objects (curbside uses and stops, then calming, then green) stay reachable as listboxes, one tab stop
  each, with the arrow and Alt+arrow keys they have today. They may be the same `role="listbox"` markup drawn over the
  plan, or sr-only lists beside it; the e2e and a11y specs that drive the rows today must still pass with ids kept
  where they can be.
- The pick is announced: "From 49 m to 82 m, 33 m long" and, when it is a whole run that differs from its neighbours,
  what differs ("bike lane on the right").

## What the stretch shows and what edits do

A pick covers one or more runs. Every edit made with a pick applies to every run inside it, after splitting at the
pick's ends where they fall inside a run. A split that leaves a run identical to its neighbour joins straight back
(see **Joining**), so picking alone never splits: only an edit that makes the stretch differ does.

- **What is shown**: the piece list, the inspector and the 2.5D show the run at the pick's start (for the whole
  street: the first run, as today with zone 0). When the pick covers several runs, the piece list and inspector say so
  ("Differs along the stretch" where a value is not the same in every run, e.g. a width), my call.
- **Changing a piece** (width, surface, direction, curb, kind, variants): applies to "the same piece" in every run of
  the pick that has it, and only there. A piece in one run only (the middle run's bike lane) is changed only there.
- **The same piece** (my call): matched by uid first, then, for pieces left unmatched, by kind in left-to-right
  order, the rule `transition.rs` already uses to pair two zones' pieces (one shared function, not two). Uids alone
  are not enough: they survive `split_zone` and copy-on-write, but templates (`Template::arrange`), lane measures
  (`measures::arrange`) and the School template give new uids from `next_uid`, so a school stretch shares no uid with
  its neighbours. A whole-street "widen the sidewalk" must still widen it in the school stretch. A shown piece with
  no match in a run leaves that run unchanged, and the announcement says where the change did not apply.
- **Adding a piece**: adds it to every run of the pick, beside the same neighbour (by uid) it is added beside in the
  shown run, or at the same end when that neighbour is missing from a run. The new piece gets one uid shared by every
  run.
- **Removing or moving a piece**: in every run of the pick that has it.
- **Speed limit**: set for the pick: every run in it gets that speed. Shown as the pick's speed, or "Varies along the
  stretch" when it differs (my call). "Same as the street" stays for a stretch following the street's speed.
- **Transform menu** (Make pedestrian, transit priority measures, templates other than School): applied to every run
  of the pick; within the pick the result is one section with one set of uids (the pick becomes one run), since a
  template or measure lays the street out anew.
- **School street template** (my call): as today in the selected zone, but in the pick: it makes its 60 m school
  stretch in the middle of the pick (splitting there, across any boundaries inside), gives only that stretch the
  template's pieces and 30 km/h, and adds the raised table at its middle. The rest of the pick is left as it was.
  When the pick has no 5 m to spare on each side, the pick itself is the school stretch. Its wording says "school
  zone" (real-world term), never "zone n".
- **Refusals**: an edit refused in any run of the pick is refused as a whole, naming where ("not in the stretch from
  49 m to 82 m: …"). The builder confirms per edit kind that the checks of a single run hold run by run.
- Each edit across several runs is one undoable step (`squash_since`, as the School template does).

### Joining

After an edit, adjacent zones with the same pieces (ignoring uids, `same_pieces`) and the same speed join into one
and the boundary disappears, but only the zones the edit touched and their immediate neighbours (my call). This is an
editor step run after an edit (in `street/model/`, called from the edit path), **not** part of `settle_zones` /
`Street::settled()`, which run on load, in fixtures and on the city side: joining there would rewrite saved streets on
open (their boundaries, names, map ticks, junction ends and the next synced record), which the anti-goals forbid. A
saved street with identical neighbouring zones keeps them until an edit touches them. A joined run keeps the first
zone's uids and saved name. No join on an elsewhere street (structure edits are refused there).

### Whole-street and base section

Today one zone means `zones` is empty and edits go to the base section (`Street.segments`); a zone with
`segments: None` follows the base. With a whole-street pick, edits apply to every run, which for runs that follow the
base means editing the base once. The builder keeps the canonical form (`settle_zones`): one run is empty `zones`.

### Elsewhere

A street whose zones were drawn on a road of another length or from its other end (`Editor::elsewhere`) keeps its
runs, cannot have boundaries made, moved or joined, and its runs' sections stay editable. With the zone commands gone,
this means (my calls): the plan is drawn at the loaded length; zones starting at or beyond it (`Editor::is_cut_off`)
are not drawn, cannot be picked and are skipped by every edit, including a whole-street pick, as they cannot be
selected or edited today; a pick may only be a whole run or the whole street (a drag snaps to whole runs), where a run
is the maximal set of identical neighbouring zones and an edit applies to each zone in it; and the notice
(`zones-elsewhere`) is reworded without "zones": "The changes along this street were drawn when it was 128 m long;
here it is 131 m."

### No anchor

A street with both ends cut by the area's edge has no frame (`City::zone_frame` is `None`), so the editor has no
length (`street_length_mm()` is `None`; `st-street-sub-free` leaves it out). My call: no new plumbing. The plan is
drawn schematic and unscaled for such a street: the shown run's bands across the full width, both ends marked cut, no
positions, no objects along it (they need an anchor today too), and only the whole street can be picked. A drag says
the reworded `zone-refused-no-anchor` ("Both ends of this street are cut by the edge of the map, so it can only be
changed along its whole length."). A saved record that already holds zones keeps them (nothing on load removes them);
edits then apply to every run, as with a whole-street pick.

## Details panel

- The Zone level goes (`Level::Zone`, `#p-zone`, `ZoneTools`, `ZoneEntry`, `#zone-entry-slot`). The path is Street;
  Street › piece kind; Street › use / stop / element. Choosing Street picks the whole street.
- The Street level's head: "Local street · 12.3 m wide · 128 m long" (no zone count; width of the shown run). When the
  pick is not the whole street, the head says so: "From 49 m to 82 m · 33 m long". The `.zone-mini` goes.
- `st-speed-zone-hint` becomes "For the stretch from 49 m to 82 m."
- Reset to the street's section and Use this section for the whole street go: the first is undo or editing the
  stretch back (it then joins), the second is picking the whole street and editing.

## Wording

Every message naming a zone is reworded to name a place, both `en.ftl` and `fr.ftl`, with the same ids and variables
where the meaning holds, new ids where it does not (the parity test and `i18n_guard` keep them honest; dead entries
fail):

- `zone-unnamed`, `zone-unnamed-mid`, `zone-selected`, `check-zone-in` ("Room for the transition from 49 m to 82 m"),
  `svg-iso-label-zone`, `svg-iso-label-joint`, `svg-iso-transition`, `check-detail-transition-room`,
  `calming-instance-zone`, `rev-zone-*` (e.g. "Speed limit 30 km/h from 49 m to 82 m"; the split/merge/name revisions
  go, a boundary move becomes "Move a change along the street to 52 m"), `zone-refused-*`, `template-school-*` (keep
  "school zone"), `st-zones-*`, `st-zone-*`, `st-row-zones`, `st-street-sub*` zone counts, `st-path-hint`,
  `st-path-zone-entry`, `transform-templates-hint` ("…the picked stretch…"), `src-applies-transition` ("Applies where
  a street's section changes along it: …"), `zones-elsewhere`, `zone-cut-off`.
- Map: `map-zones`, `map-street-sub-zones*`, `map-label-street-zones*` go (the count is not shown); the ticks and
  bands where a street changes stay. `map-report-place-zones` ("This street changes along its length. The figures show
  it at its first end."), `map-report-place-other`.
- Junction: `jn-check-approach-ok`, `jn-approach-short` and the arm descriptions say "end stretch" for "end zone" and
  name the next stretch by position, not as a zone.
- Timeline: `timeline-aspect-zones` becomes "changes along the street".
- fr-CA: "tronçon" for stretch, per the OQLF rules and `docs/glossary-fr-ca.md` (add the term there).
- The user docs (`website/`) lose their mention of zones.

Internal names (`Zone`, `zones.rs`, `select_zone`, `ZonesView`, CSS classes, element ids) may stay where renaming is
churn; ids the e2e specs read are renamed with the specs.

## What goes

`street/view/strip.rs` (the zone strip), `zone_tools.rs`, the curbside/stops/calming/green strips as separate rows
(their logic moves into the plan; their tools in the details panel stay), `StreetVm`'s `split_zone`,
`begin_merge`/`finish_merge`/`cancel_merge`, `name_zone`, `reset_zone`, `use_for_street` as person-facing commands (the
model's operations may stay as the building blocks of pick edits), `select_zone_relative`/`select_zone_edge`, the
Zone level.

## Tests

Test first, as the project requires:

- Model: pick edits across one and several runs (change, add, remove, move a piece; speed), splitting at the pick's
  ends, joining after an edit, no split from a pick alone, slivers, elsewhere (cut-off zones skipped), no anchor,
  whole street on the base; piece matching by uid then kind (a whole-street edit after the School template reaches
  the school stretch); a saved street with identical neighbouring zones loads unchanged and round-trips; positions on
  a street anchored at its last end.
- View-model (native, `test_ports()`): click picks a run, drag picks a stretch, handles, boundary drag, keyboard pick,
  announcements, details path without a Zone level.
- Plan renderer: pure SVG string tests (bands per run, objects in place, tapers at true length, cut ends), and the a11y
  tree of the plan's listboxes.
- `i18n_guard`, parity, and a test that no English or French message the street page, map, junction, report or
  timeline can show contains "zone" except "school zone" and "loading zone" (and their French forms).
- e2e: the specs listed by `grep -l -i zone e2e/tests` move from the zone strip and tools to the plan
  (`showZoneLevel` goes from `e2e/tests/fixtures.ts`); the street, speed, curb, stops, calming, green, templates, map
  and a11y specs keep passing.

## Left to the builder

- The plan's exact drawing (band colours, object symbols, labels, minimum scale, height), within the existing tokens.
- How the objects' listboxes sit with the plan for keyboard and screen reader, within **Keyboard and screen reader**.
- Splitting the work into pull requests (a sensible order: model pick edits and joining; the plan drawing; the plan's
  interaction replacing the rows; the Zone level and wording; map, junction, report, timeline wording and docs).
- Version bump: minor (a feature) per pull request.
