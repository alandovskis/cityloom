# Engineering standard: lane width limits by road type, speed and volume

Date: 2026-10-01

## Goal

Give the city an **engineering standard**: the minimum and maximum width of
each kind of street piece, depending on the street's **road type**, its
**design speed** and the **traffic volume of the mode that uses that piece**.
Each city has a default standard, and a resident can pick another from a short
list. The standard is advisory. A width outside it is allowed and is reported
as a failed check.

## Decisions taken with the user

- Each street stores its own road type, design speed and peak-hour volume per
  mode. They are inputs the resident can change, not derived from the sample.
- The standard is **advisory**: a "Lane widths" sheet check. It does not clamp
  dragging or typing.
- It covers **every kind** of piece (sidewalk, planting, bike, driving, bus,
  parking, median, loading, shoulder), not only roadway lanes.
- Volume is **of the relevant kind**: a piece is judged against the volume of
  its own mode. Pieces whose mode has no traffic (planting, median) use the
  road type and speed alone.
- The resident **chooses from several** built-in standards. There is no table
  editor. The city names a default.
- The values in every built-in standard are **synthetic placeholders**, labelled
  as such like the rest of the catalogue. They are not any jurisdiction's
  standard (`PRODUCT.md`: do not fabricate data).

## How it sits with the existing limits

`KINDS[k].min_mm` and `max_mm` stay as they are. They clamp dragging, decide
which types a piece can take at other times (`alt_kinds`), and size arranged
measures (`street_measures::arrange`). The standard sits on top of them as a
tighter, advisory range. No existing editing behaviour changes.

The effective range for a piece is the standard's range clamped into its
kind's `[min_mm, max_mm]`. A standard whose range for some piece lies wholly
outside that envelope would be unreachable, so a test rejects it (see Tests).

## Model (Rust)

New module `src/standard.rs`, pure Rust like the others.

### Road types and speed bands

- `ROAD_TYPES`: `local`, `collector`, `arterial`, `freeway`, each with an id and
  a display name.
- Speed bands by upper bound in km/h: up to 30, 31 to 50, 51 to 70, over 70.
  A speed always falls in exactly one band.
- Speed is whole km/h, 5 to 130. It stays in km/h whatever the m/ft toggle
  says, as `junction_view` already does.

### A standard

A `Standard` has an `id`, a `name`, a one-line `note`, and:

1. **Base rows.** One row for each (road type, speed band): 16 per standard.
   A row gives `(min_mm, max_mm)` for every kind.
2. **Volume steps.** For each mode that carries traffic (Foot, Bike, Transit,
   Vehicle), two thresholds that split a volume into low, typical and high,
   and for each of those three bands an adjustment `(min_add_mm, max_add_mm)`
   applied to the base range of every kind of that mode. Green has no volume
   and no step.

How the rows are laid out in the source (literal arrays, or built by a const
helper from a few anchor values) is an implementation detail. The lookup below
is the contract.

### Lookup

`limits(standard, profile, kind) -> (min_mm, max_mm)`:

1. Take the base row for the profile's road type and the band of its speed.
2. Take the kind's `(min, max)` from it.
3. If the kind's mode has a volume, find the volume's band for that mode and
   add that band's adjustment to min and to max.
4. Clamp into the kind's `[KINDS.min_mm, KINDS.max_mm]`, then make sure
   `min <= max` (if the adjustment crossed them, set both to the clamped min).

### Built-in standards

Three, in a fixed order, all synthetic: a compact one (narrower lanes, more
people per street), a balanced one, and a generous one. The balanced one is the
city's default. Names are placeholders to settle in the plan.

### Per-street inputs

`model::Street` gains a `profile`:

- `road_type: usize` (index into `ROAD_TYPES`)
- `speed_kmh: i32`
- `volumes: [i32; 4]`: peak-hour, both directions, in the mode's own unit
  (people walking, people biking, buses, vehicles per hour), indexed by
  Foot, Bike, Transit, Vehicle. 0 to 20000.

Defaults come from the sample (synthetic):

| Sample | Road type | km/h | Foot | Bike | Bus | Vehicle |
|---|---|---|---|---|---|---|
| Street | collector | 40 | 300 | 60 | 6 | 600 |
| Avenue | arterial | 50 | 600 | 100 | 20 | 1500 |
| Lane | local | 30 | 100 | 20 | 0 | 150 |
| Freeway | freeway | 100 | 0 | 0 | 0 | 4000 |

`Street::sample` fills these. `is_sound` rejects an index out of range, a speed
or a volume out of range. `for_side` and `reversed` carry the profile through
unchanged.

The profile is a **setting of the street, not part of its undo history**, like
the region and the time of day. A change to it is not a revision under "Your
changes", but it is saved with the street, and it makes the street count as
edited on the map (`now != today`). "Start over" on a city resets it.

### The check

`model::checks` gains a check `id: "widths"`, label "Lane widths within the
standard". For each piece at the shown time it takes `limits(...)` and compares
the width. It passes when every piece is within range. When it fails, `detail`
names the worst piece and how far out it is ("Driving lane 3.60 m, standard
allows 3.00 to 3.30 m"), and "and 2 more" if there are others. `amount_mm` is
that worst distance. Checks carry no piece ids today and this change does not
add them.

`street_measures::MIN_TRANSIT_LANE_MM` is removed. The transit-lane message
reads the standard's minimum for a bus lane on this street, so there is one
source of truth. The measure logic around it is unchanged.

### Editor

`Editor` takes the standard as a setting, like `region`:
`Editor::from_street(today, now, region, standard)` and a default standard for
the standalone `Editor::new`. `SegView` gains the standard's range for that
piece: `std_min_mm`, `std_max_mm`, and `std_ok`. Setting the profile on the
editor is `set_road_type`, `set_speed`, `set_volume(mode, value)`, each
returning false for an invalid or unchanged value. The view reports the
profile and the per-kind limits table for the Standard tab.

### City

- `City` holds `standard: usize`, an index into the built-in list, and
  `DEFAULT_STANDARD` next to `NAME` names the city's default. `City::new` uses it.
- `City::set_standard(i) -> bool` and `City::standard()`.
- `City::view` passes the standard to each street's editor. A street that
  fails the standard then appears in `failing` like any other failed check, so
  the map shows it with no new mechanism.
- `City::street_profile` and `City::set_street_profile` read and write one
  street's profile, so the street page can store what the editor holds.
- The view reports the active standard and the list of standards (id, name,
  note) for the map's picker.

### Saving

The save stores the standard by **id**, not index, so reordering the list does
not change a saved choice. An unknown id falls back to the city's default.

Streets saved before this change have no profile. They load with the profile
their sample gives, and keep their widths. `SAVE_VERSION` is **not** bumped,
because bumping it would throw away every resident's saved city. The plan
decides the exact serde mechanism; the requirement is that an old save loads
with its streets and junctions intact and a test proves it.

### `lib.rs` (WebAssembly)

- `Sheet`: `set_road_type(index)`, `set_speed(kmh)`, `set_volume(mode, value)`,
  `set_standard(index)`.
- `City`: `set_standard(index)`, `street_profile`, `set_street_profile`.
- A `standards()` export gives the road types, speed bands, volume thresholds
  and the list of standards for the pages. The pages hold no rules.

## Interface

The city is shared across pages, so the standard is chosen on the **map**, and
a street's own inputs are set on the **street** page. The existing drafting-sheet
design (`DESIGN.md`) applies. Redline for a failure, with words and an icon, so
colour is never the only signal.

### Street page

- **A "Standard" tab** in the notes column, after Measures. It shows:
  - the city standard's name and note;
  - this street's inputs: road type (select), design speed in km/h (number),
    and the four volumes (numbers) with their units named in the labels;
  - a table of the allowed range for each kind on this street, `min to max`,
    beside the street's current widths.
  - The sample-numbers note ("not real measurements") covers it.
- **Lane inspector.** The existing "Allowed 2.70 to 4.00 m" line stays, since it
  is the hard limit on the field. Under it a second line, "Standard: 3.00 to
  3.30 m". Outside the range it takes the redline style and says how far out in
  words ("0.20 m over the maximum"). Typing or dragging outside the standard is
  still accepted.
- **Checks tab.** The "Lane widths" check, as above.

### Map page

- A select for the city's standard in the notes panel, with its name and note.
  Changing it re-checks every street at once, and the map's failing count
  follows.

### Not in this change

- A page for reading a whole standard, and any editor for standards.
- Clickable check items that select a piece.
- Junction-specific standards. Junctions read their streets from the city, so
  the check follows the street.
- Real standards or real volumes.

### Input handling

A speed or volume field that is empty, not a number, or out of range keeps the
last valid value and says so in the live region. All controls are labelled and
reachable by keyboard, like the rest of the sheet.

## Tests

Rust, native, in the existing style:

- Lookup: each of the 16 rows is reachable. The band edges (30, 31, 50, 51, 70,
  71 km/h) fall in the right band. A volume on each side of each threshold
  changes the range as the step says. A kind with no mode volume is unaffected
  by volume.
- Every built-in standard, for every road type, speed band, volume band and
  kind: `min <= max` and the effective range lies inside the kind's `KINDS`
  envelope and is not empty.
- The three standards give different answers for at least one lookup, so a
  picker choice is never a no-op.
- The check passes for every sample street under the default standard, and
  fails, naming the lane and the range, for a width outside it.
- A bus lane's minimum in the transit measures comes from the standard.
- Profile: defaults per sample, `is_sound` rejects bad values, `for_side` and
  `reversed` keep it, setting it does not enter history, and it marks the street
  edited.
- City: the default standard is in use after `City::new`. A save round-trips
  the standard id and each profile. A save from before this change loads with
  streets and junctions intact and sample-default profiles. An unknown standard
  id falls back to the default. `set_standard` changes `failing` on the view for
  a street that sits between the two standards.

Browser: the pages are checked by hand and with the existing build
(`scripts/build.sh`), as for the earlier features.

## Open items for the plan

- Names of the three built-in standards, and the synthetic numbers.
- The serde mechanism for loading a street that has no profile.
- Where the Standard tab's range table and the lane inspector line share code.
