# Junction slice (`src/junction/`)

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

## Turbo layouts (`turbo.rs`, `turbo_spiral.rs`, `turbo_ring.rs`)

- Basic, egg, knee, spiral, rotor, in the four Dutch blocks. Figures are placeholders from secondary sources on CROW 2008; unverified ones are marked in doc comments.
- A turbo is a layout of the `ROUNDABOUT` control, not a control: `State.turbo: Option<Turbo>` (layout, block, two main-road arm uids; saved, skipped when `None`, range-checked by `Layout::junction_fits`).
- `turbo.rs`: the layout, not the street, sets entry lanes (an arm cars do not enter gets none) and fixes lane destinations (`set_lane_dest` is refused). Main-road candidates: best opposite pair first. An arm skewed over 20 degrees from its template direction refuses the edit and names the arm; 10 or more fails `turbo-square`.
- `turbo::uses` lane table: main arm first lane left and through, last through and right, middle through only; side arm's outermost lane (right in right-hand traffic, left in left-hand) turns outward only; a lone lane serves every turn.
- `turbo_spiral.rs` is the ring as pure geometry (CETRA 2016 Fig. 3 and 4):
  - Each lane edge is two half circles either side of the translation axis (57.5 degrees from the main road with four arms, 65 with three, mirrored for left-hand traffic), centres shifted along it (island by Δv, other edges by Δu). At the axis a lane's inner edge on one half meets its outer edge on the other within 50 mm (a test pins it for all four blocks).
  - `ring_shape`: one open `RingLane` per half and ring lane (anticlockwise for right-hand traffic), closed island, carriageway and cycle-track outlines, and dividers (each half's R2/R3 arc cut where a street's carriageway meets the ring, `Mouth`; none under 3 m).
  - `spiral_walk`: a movement keeps its lane, steps out a lane at each tip, and in the outermost lane leaves at the tip for the first street after it. `turbo_path` tries a lane further in when the entry's lane cannot reach the street (an egg's or knee's lone lane).
  - A test checks no movement of any layout, block or side crosses a divider (dividers are cut at every street, so an inner lane may leave across the outer lane).
- UNVERIFIED (spec amendment 2026-10-13, J1 to J7): which end of the axis leads (where the tips lie, which arms are side arms; if it flips, main and side swap), where a divider begins and ends, the spiral and rotor blocks (the one-axis block is Egg, Basic and Knee; a third lane is an extension), where a movement leaves at a tip, the egg's and knee's lone lanes.
- `settle_table_and_boxes` drops a turbo when the control, arm count, skew or room for an entry no longer suit it; no bus lane across the island, no queue jump, crossings 6 m from the ring.
- `read_model::layout` widens each entered arm to the layout's lanes (`TURBO_ENTRY_LANE_MM`, 3.5 m, placeholder) and builds the ring on the block. A widened entry past its street's property line is refused as `TurboEntryTooNarrow` (names the arm; a saved turbo with no room is dropped like one past the skew limit); a turbo fitting no block is refused as too big.
- `RingView.lanes` / `.dividers` carry the ring to the 2.5D (dividers open, painted double white line in pieces, `shared::iso::paint`). Its sidewalks stay where the street has them.
- `read_model::choose_turbo` is the one test both `set_turbo` and `turbo_options` use.
- Inspector layout choice: WAI-ARIA radio group (one tab stop; pure `keys::radio_choice` / `radio_tab_stop`); re-choosing the chosen layout is a silent no-op.

## The 2.5D diagram (`junction/iso/`, #140)

- `IsoFrame` (plan turned 30 degrees, depth 0.55, height x3) implements `shared::diagram::Projection` (`Pt` = plan x, height, plan y); affine, so `ground_at` is the one matrix for every texture. `build.rs` makes each part's shapes; `scene.rs` (`Scene`) gathers layers into a `Diagram`; `shared::diagram::render` writes it (defs, then one group `iso iso-junction`).
- Layers bottom to top: `Ground`, `Surface` (laid order, places to press among it: order decides a press), `Raised` (far to near), `Markings`, `Objects`, `Signs`, `Outline` (selection). `Hit` stays empty: a place to press lies with what it is on.
- Renderer shapes: `Shape::Path`, `Shape::Elem`, `Shape::Caption`. A textured top on an affine projection is the top plus a `hatch` polygon over it (a fill attribute would lose to the top's CSS fill).
- The 2.5D is the page's only drawing, pressed to select; no flat plan (`plan_svg.rs`), toggle, `V`, grips or `DrawMode`; a value under `cityloom-junction-view` is ignored. `drawing.rs` is the component plus pure `selected_by` / `pressed`.
- Selectable parts carry `data-role` / `data-uid` / `data-lane` (`iso::hit_attrs`): an arm's pieces, a corner's sidewalk, protected track and island, a crossing's island, a roundabout's cycle track and bus lane. Transparent `press_area`s cover an entering lane (`lane-hit`, name as `<title>`), a crossing and a corner with no sidewalk (`iso-hit`).
- All else is `pointer-events: none` (`#drawing .iso-junction`, `#drawing .iso-junction *`), so a standing sidewalk selects its arm. `[data-role]` has `touch-action: manipulation`.
- Target size (WCAG 2.5.8): a place to press under 24 px gets an unpainted stroke to 24 px (`iso::target_growth_px`, `stroke: transparent` in `editor.css`); a corner's place is drawn after the streets so it wins where grown; where neighbours still leave a part less, `#part-previous` / `#part-next` (`jn-part-*`) step through every part like the arrow keys. `e2e/tests/junction-a11y.spec.ts` measures targets at 1440, 1050, 600 and 280 px and keeps a floor.
- Drawing at least `frame::MIN_WIDTH` (240 px): no sideways scroll at 280 px. A zebra and a stop line are pavement paint (`--iso-paint-white` with its dark edge), 3:1 on every material.
- Selection outline: `iso-sel-casing` under `sel-box iso-sel`, over the solids (a cycle track has two), at the height it stands. Nothing is dragged: inspector fields with steppers (`i-h-bearing`, `i-h-corner`, `i-h-setback`) and a key (Shift+Arrow, `+`, `-`).
- `iso_label` names the junction in 2.5D with its streets, and a roundabout, turbo layout or raised table (`jn-iso-label*`).

### Ground marks (`junction/iso_marks.rs`, #140 stage 2)

- Pure geometry in ground mm, projected through `IsoFrame::at`. Each line lies on a casing (`iso-casing`; `iso-casing warn` in `--iso-sel-casing` under a warning).
- Drawn: `lane-arrow` on entering lanes (`bad` when only banned turns), `out` where traffic leaves; a roundabout's arrows and "Bus only"; turns of the selected street, lane, crossing or corner (`mv`, banned `no` with a cross, no lane `bad`, `indirect`); `stop-pad`, dwell, run, post; `approach-mark`; a virtual queue jump's loop; a bus gate's line; a dead end's cap; `measure-tag` (Atlas code); `iso-queue`; a failing curbside use's outline and failing corner's curb; a turbo entry's widened road.
- A crossing's length: `dim`, `dim-warn`; overlapping figures move apart by whole lines, the longer crossing keeping its place (`iso_marks::spread_labels`).
- Not drawn: north mark, scale bar, the street's width and compass line, a dead-end arm's fade and hatching, conflict points, a lane's destinations.

## Junction approaches (`street/model/beyond.rs`, `junction/beyond.rs`)

- Derived, never saved: no change to `Street`, `State`'s serde or the edits document; `city::edits::VERSION` stays 5.
- `Street::beyond(first_end, frame)` gives an arm `Beyond { boundary_mm, next, zone, taper }`: the first zone boundary going out, the section across it (looking outward), and the `BeyondTaper` (`from_mm`, `to_mm`, `need_mm`, `room_mm`, `merges`, `TaperPiece`s), all from `transition()`; the junction never recomputes a length, host or merge. Distances are from the junction's centre.
- `Arm.beyond: Option<Beyond>` is `#[serde(skip)]` like `Arm.section`, set where `section` is (`City::with_streets_of`, `beyond_from`). Movements, lanes and crossings still read the end zone only.
- `None` for: one zone, no frame, drawn elsewhere (the page's notice), boundary at or past the road's end, same pieces on both sections, or either section wider than `row_mm` (a street failing `fits` draws as before). A section narrower than the street ("left to use") has its beyond: pieces are laid from the left edge, so a mirrored span flips by its own section's total, not `row_mm`.
- `junction/beyond.rs` (pure, mm): end-zone pieces stop at the taper's start (a moving piece) or the boundary; one quadrilateral per moving piece at true scale; then the next zone's strips to `ARM_LENGTH_MM` (36 m); two `.approach-mark` lines at the taper's ends (the boundary alone for a jog), dashed, no press, `var(--ink-2)`, `CanvasText` in forced colours, 3:1 on road, sidewalk and planting (`shared/contrast.rs`).
- A taper cut short by its host zone is drawn over the room there is and ends in a jog. A boundary or taper start at or past `ARM_LENGTH_MM` draws nothing new and is neither judged by `approach` nor described in the arm's sentence (`Drawn.shown`). The 2.5D draws the same pieces and counts them in its frame (`content_points`). Arm text: `jn-sel-arm-approach` / `jn-sel-arm-jog`.
- Check `approach` ("Approaches", `jn-check-approach*`; not `Arm.approach` / `APPROACHES`, the transit measures) fails, naming the arm: a steady end zone (to the taper's start, else the boundary) short of its arrows plus a stop line (`ARROW_CLEAR_MM`, 3 m); a host zone shorter than the taper needs; a lane that drops with nothing to merge into.
- Known limits: one boundary per arm (the first); transit measures (`Arm.approach`) are not moved to the boundary; turbo and pedestrian arms draw as before; the zone's start under the mouth is hidden; marks and taper pieces are all that is drawn beyond.
- `e2e/tests/junction.spec.ts` covers the drawing, the check and its failure on a street divided in two.

## Stops at junctions (`street/model/stop_end.rs`, `junction/stops.rs`, PR 2 of #70)

- Derived, never saved (no change to `Street`, `State`'s serde, the edits document or `VERSION` 5); `Arm.stops` is read from the street.
- `Street::end_stops(first_end, frame, driving)` reads the street under the side `driving` names (`for_side`, as `Editor::from_street` and `profile_of`; an imported street keeps traffic on the right whatever the region, so `enters`, `lane`, `pad` are as the region's junction reads it; `stops_from` and `City::with_streets` take the region's side).
- It returns stops wholly inside the end zone, nearest first, as `EndStop { index, side, near_mm, far_mm, kind, shelter, name, enters, lane, pad }` in the arm's orientation: mm from the junction's centre, `side` swapped at the last end, `enters` = near-side stop, `lane` / `pad` from the street's own `bus_lane` / `stand` (never recomputed). Empty without a frame, for a frame of another length or anchor, or without stops.
- `Arm.stops` is set beside `Arm.section` / `Arm.beyond` (`stops_from` in `city/model.rs`, cleared with them). The read side draws and judges none on a turbo or pedestrian arm, keeps `near_mm < ARM_LENGTH_MM`, clips to the drawn arm, drops one wholly past it.
- `EndInfo.reach_mm` (`city::model::street_end_infos`, via `Editor::with_ends`): how far the junction reaches along the street (`read_model::arm_reaches`: the crossing's far edge, else the mouth; read on `in_force`). 0 for a pedestrian arm and any turbo arm, so the street page shows no `stop-junction` failure where the junction shows none. `arm_at_end` picks an end's arm: a street returning to its junction has one arm, read at its first end, so its last end has no reach. Computed only for a street with stops, read when the street page opens (stale until reload).
- `STOP_CLEAR_MM` (5 m, UNVERIFIED, same as `CROSSING_CLEARANCE_MM`): how far past the reach a stop's near end must stand (`stop_gap_mm`). Both pages measure the far end from the frame's length, not `length_mm` (may differ within `LENGTH_TOLERANCE_MM`). Closer fails; a negative gap says so (`jn-stop-inside`, `check-detail-stop-junction-inside`), never refused.
- Checks: street page row `stop-junction` (`check-label-stop-junction`, only once an end has a reach); junction `stop-clear` (`jn-check-stop-clear*`) and `stop-lane` (`jn-check-stop-lane*`), each naming stop and arm.
- A lane belongs to a stop only through `junction::stops::lane_of`: the section's `travel` piece with direction `toward`, counted from the driver's left. A `bus` piece has no lane or destinations, so a stop in a bus lane is exempt, as is an arm of one entering lane.
- Only an entering stop draws a run (down its lane to the stop line, else the mouth) and is judged for lanes: a lane not going straight on, at a junction with a straight-on movement, fails (`jn-stop-turn-only`); a straight-on lane that also turns toward the kerb fails (`jn-stop-shared-turn`). A transit measure's bus stop (`Arm.stop`, M1/M2) plus a street stop on the same kerb side raise `jn-problem-stop-twice`.
- Drawing: `junction/stops.rs::draw` (pure polygons in mm, cut to the drawn arm and `beyond::Drawn.cuts`): `.stop-pad` (+ `.stop-bulb` / `.stop-island`, so kind shows without colour), `.stop-dwell`, `.stop-run`, `.stop-post` (with `.stop-shelter`), `.stop-bad`; on a casing, no press. Arm text `jn-sel-arm-stop*`. Colours and forced-colours rule in `web/editor.css`, contrast pair in `shared/contrast.rs`.
- Known limits: end zone only; a stop straddling the end zone's boundary is not an `EndStop` (judged on the street page only, against the reach; the junction neither draws nor judges it); no routes; turbo and pedestrian arms skip stops; figures are placeholders.
- `e2e/tests/junction.spec.ts` covers a stop too near (drawn, both checks fail, passes 20 m out) and the turning lane (finds the arm with two entering lanes and the side by the street page's "Measured from" end).

## Signal plans (`signal.rs`)
- The cycle has one phase per street with entering lanes, in bearing order, plus which pedestrian crossings and bike movements run with each phase.
- `Junction::set_phase_pattern` gives a phase a ready-made `Pattern` (through only, all movements, protected left then through, pedestrian scramble); `Custom` is what matches none and is never chosen. Offered only where the lanes support it (`signal::patterns_for`, deduplicated), chosen in the inspector by a radio group per phase.
- A scramble serves no movement and is kept as a stored phase with `arm` (written only then, so no version changes).
- `Arm.timing` was added within `city::edits::VERSION` 3 and is saved only once set; version 4 discards version 3 documents, timing included.
