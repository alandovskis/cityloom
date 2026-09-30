# Intersection editor: a plan sheet for where streets meet

Date: 2026-09-30

## Goal

A second surface beside the street cross-section editor. A resident opens one
junction of three to five streets, sees it in plan at true scale, and changes
how it works: the shape of its corners, where people cross, the lanes each
street brings to it, which turns are allowed, and how it is controlled. The
sheet says what that does to crossing distances, turning speeds and whether the
turns line up with the lanes.

## Decisions taken with the user

- **Scope:** a whole-junction network in plan: three, four or five arms at any
  bearing, offset crossings, and a roundabout as a control type.
- **Levers:** corners and crossings; lanes at the approach; control type; turn
  movements. All four are in the first version.
- **Depth:** design plus a working prototype, built code-led inside the
  existing sheet world (no new visual identity, no comp, no structure
  tournament; the user chose the prototype path).

## What an arm is

Each arm is a street that meets the junction. It takes its cross-section from
one of the sample streets and reads it, in plan, as strips running out from the
junction: sidewalks, planting, parking, bike and driving lanes keep their tint,
hatch and name. **The arm's section is not editable here.** The section editor
owns it; this sheet only reads it. Linking the two (open this arm in the
section editor) needs shared streets in the model and is out of scope.

Left and right of an arm are as seen standing at the junction looking outward.
A lane's direction is the section editor's: "away" leaves the junction,
"toward" enters it. Which half of the street carries entering traffic follows
the region's drive side, which this page shares with the section editor.

## Model (Rust, `src/junction.rs`)

All lengths are integer millimetres, angles integer degrees. Bearings run
clockwise from north (up on the sheet). Y grows downward, as in SVG.

State that is history (one revision per change, gestures collapse a drag):

```
Junction { arms: Vec<Arm>, control: Control, ring_mm: i32 }
Arm {
  uid, street: usize,        // index into SAMPLES
  bearing: i32,              // 0..360, multiple of 5
  offset_mm: i32,            // sideways shift of the axis, multiple of 100
  corner_mm: i32,            // curb radius at the corner clockwise of this arm
  lanes: Vec<Lane>,          // entering lanes, driver's left to right
  crossing: Option<Crossing>,
  bulb: [bool; 2],           // curb extension, left and right
  banned: Vec<u32>,          // uids of arms this arm may not turn into
}
Lane { uses: u8 }            // bit 1 left, bit 2 through, bit 4 right
Crossing { setback_mm, width_mm, island: bool }
Control = Uncontrolled | Priority | AllWayStop | Signal | Roundabout
```

Arms are kept sorted by bearing. The arm profile (edge zones, carriageway,
entering and leaving lane counts, parking beside each curb) is read from the
sample street at the sheet's drive side, so a region change re-reads it.

### Rules

- 3 to 5 arms. Bearings at least 30 degrees apart, and no gap between
  neighbours wider than 180 degrees (a reflex corner has no curb to round).
  A rejected edit returns false and changes nothing.
- Offset is limited to plus or minus half the arm's carriageway, so the two
  sides of an offset crossing still overlap the junction.
- Corner radius 1.0 to 15.0 m in 0.5 m steps, default 6.0 m.
- A crossing sits 2.0 to 8.0 m from the junction mouth, 2.0 to 6.0 m wide,
  default 3.0 and 3.0. A refuge island needs a carriageway of 9.0 m or more.
  A bulb-out needs parking or loading beside that curb.
- **Movements.** For entering arm A and leaving arm B the turn is
  `bearing(B) - (bearing(A) + 180)`, folded to -180..180. Within 35 degrees
  is *through*, more clockwise is *right*, more anticlockwise is *left*.
  Each ordered pair of arms is allowed unless banned. An arm cannot ban every
  pair. A street that only leaves the junction (one-way out) has no entering
  lanes and no movements.
- **Lanes.** `uses` names the classes a lane serves. Editing a lane that
  would leave it serving nothing is rejected. When geometry changes, uses are
  masked to the classes the arm has; a lane left empty takes the default for
  its place (one lane serves everything; two are left and through, through and
  right; three or more are left, through..., right).
- **Control.** Setting `Roundabout` turns the crossing into a ring; any
  other control turns it back. The ring's outer radius is the least that
  keeps neighbouring arms apart (the floor), plus whatever the resident adds,
  up to 40 m. Without a ring, `Priority` calls the two arms nearest to a
  straight line the priority street, the rest give way.
- Region changes are a setting, not history, as in the section editor.

### Geometry (all of it in Rust; the page draws what it is given)

For each arm the model returns, in sheet millimetres:

- its axis frame (origin, outward direction, right-hand normal);
- the lateral position and extent of every piece of its section;
- its mouth (where the pieces stop and the junction starts);
- the corner curbs as lines and fillets between neighbouring arms (a straight
  curb between arms 180 degrees apart), or, for a roundabout, the ring and the
  curb arcs between arm edges;
- crossings (the four corner points), stop lines, bulb-outs, refuge islands;
- the movements: for each ordered pair a centreline path and its class.

The page transforms those into SVG. It contains no geometry rules.

### Checks (synthetic placeholder thresholds, labelled as such)

| Check | Passes when |
|---|---|
| Lanes cover every turn | each allowed movement has a lane of its class |
| Lanes follow the turn bans | each lane class has an allowed movement |
| Crossing distance | every stage is 15.0 m or less |
| Turning speed | a turn across a marked crossing is 25 km/h or less at the corner radius (v = sqrt(127 R 0.3)) |
| Fits the arms | ring (if any) keeps every arm clear of its neighbours |
| Signal has room | a signal junction has no more than 4 arms |

### Numbers

Alongside the checks, the notes show per arm: crossing distance and stages, the
turning speed of its right-hand and left-hand turn, entering lanes and the
movements they serve. **Conflict points** for the whole junction: crossing,
merging and diverging points among allowed movements, counted from the arm
geometry and the control (a signal separates them in time and is shown as
"separated by phase" rather than removed). People moved is not attempted here:
no capacity model exists for junctions and none is invented.

## Page

A new page, `web/intersection.html`, with `web/junction.js`, in the same sheet.

- **Shell.** The top bar, avatar menu (units, region, theme, drawing style),
  notes and inspector toggles, and notes tabs are shared with the street page.
  They move out of `app.js` into `web/shell.js` first, as a
  behaviour-preserving change.
- **Surface switch.** Two text tabs in the top bar after the wordmark, *Street*
  and *Intersection*, on the 1px rule with the 2px ink underline for the page
  you are on. They are links, not a router.
- **Layout.** As the street page: left inspector, plan drawing in the middle,
  notes column on the right, one sheet with the same frame, same breakpoints.
- **Plan drawing.** Drawn in the sheet's own vocabulary: tint plus hatch plus
  name per piece; ink at graded weights; blue pencil for what is selected;
  redline only where a check fails. North is up and marked once. A graphic
  scale bar and one overall dimension (arm to arm) close the drawing. The
  Engineering drawing style redraws the same plan as plain line work.
- **Selection.** A resident selects an arm, a corner, or a crossing (each is a
  target in the drawing and reachable by keyboard); the inspector shows that
  target's controls. Nothing selected: the junction's own controls (control
  type, ring size) plus a short line saying what to select.
- **Direct manipulation.** Drag an arm's end around the junction to change its
  bearing (snaps to 5 degrees; a rejected position shows the ghost stopped at
  the last legal one). Drag a corner along its bisector to change its radius.
  Drag a crossing toward or away from the junction to change its setback.
- **Keyboard.** Tab enters the drawing; left and right select the next arm,
  corner or crossing in clockwise order; plus and minus change the selected
  thing's main number (bearing 5 degrees, corner 0.5 m, setback 0.5 m);
  Delete removes an arm or crossing; Shift plus arrows rotate an arm. Every
  action also has a button in the inspector.
- **Add an arm.** A palette of the sample streets, as ruled rows like Add a
  piece. Drag onto the drawing to place at that bearing, or press to add at the
  widest free gap.
- **Notes tabs.** *Space* (per-arm numbers and the movement table), *Checks*,
  *Changes*. The movement table is a drafting schedule: rows are entering
  arms, columns leaving arms, each cell a toggle (allowed, no turn) with its
  class letter, so it is a schedule you edit, not a picture of one.
- **Inspector.** Arm: name, street, bearing, offset, lanes at the approach
  (each lane, with left/through/right toggles), crossing (on, setback, width,
  island), bulb-outs. Corner: radius. Junction: control type, ring size.
- **Motion.** The plan redraws instantly during a drag. Adding or removing an
  arm and switching to or from a roundabout ease over 220ms unless reduced
  motion is set.
- Responsive as the street page; the plan scales to the width and keeps
  aspect, no horizontal scroll needed since it is a square-ish drawing.

## Sample junctions

Three synthetic starting junctions: a four-way of two avenues and streets, a
T with a lane, and an offset crossing. Each names the sample street per arm.
"Start over" returns to the loaded sample.

## Out of scope

Editing an arm's section here; saving, sharing or exporting; real signal
timing or phasing; slip lanes, turn pockets that add lane length or width, bike
boxes, protected corners; a map, several junctions, or a network of them; any
measured or real traffic data. All rates, thresholds and sample junctions are
synthetic placeholders.

## Verification

`cargo test` for geometry, rules, history and checks. Then build the wasm and
check in the browser: each sample; add, remove and rotate an arm; corner
radius; crossings, bulbs, islands; control types and the roundabout; lane and
turn edits with their checks; undo and redo; keyboard only; light and dark;
both drawing styles; narrow and wide; print.
