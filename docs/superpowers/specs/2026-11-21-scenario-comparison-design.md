# Scenario comparison: design

Date: 2026-11-21. Issues: CLM-9 (GitHub #191, part c of #62), which closes CLM-7 (#62). Slices: street and junction
(`crates/cityloom_editor/src/street/`, `junction/`), a shared frame in `shared/`, composed in `lib.rs`. New page
`web/compare.html`.

## Why

A place (one street or one junction) keeps a baseline design and up to 8 named scenarios beside it (`city::scenario`,
#192), and the street and junction pages can create, switch, rename and delete them (#236). What people cannot yet do
is the point of having alternatives: put two of them next to each other and see what differs. CLM-7's goal is "explore
and compare competing designs for the same street or junction without losing prior work"; its last open acceptance
point is "a comparison identifies changed street/junction allocation and changed design checks".

Success: from a place's editor, a person opens a page that shows two chosen designs of that place side by side, drawn
the same way at the same scale, with a short list of what differs between them, in English and fr-CA, usable from the
keyboard and with a screen reader.

## Goals

1. A page of its own, `compare.html`, for one place of the project: two designs (the baseline or any scenario), chosen
   on the page, drawn side by side.
2. Street: the plan and the 2.5D of each design. Junction: the 2.5D of each (the junction has no flat plan, #140).
3. A difference summary, B against A:
   - **Street:** the width each use has (the width table's design column), the street becoming pedestrian or carrying
     traffic again, the travel lanes in each direction, and pieces added, removed, or changed in kind or width.
   - **Junction:** the control (and turbo layout), per street (arm): entering lanes, leaving lanes, whether it has a
     marked crossing, a bike box; the signal's phase count and cycle.
   - **Both:** the design checks (and, for a street, the area rules) that pass in one design and fail in the other.
   - **Also differs in (both):** every other category the drawings show is compared for equality only and, where it
     differs, named in one line ("Also differs in: stops, planting"). Street: speed, stretches (zones), curbside uses,
     stops, raised elements (calming), green. Junction: roundabout ring, raised junction, bus option, corners. Each
     category is compared after leaving out what the person cannot see (uids, and order where order carries no
     meaning). "The two designs are the same" is said only when every detailed group and every category match.
     (Chosen in review over a detailed diff of every category, or accepting a wrong "identical".)
4. Reached from the scenario panel of the street and junction pages ("Compare"), keeping `project` and `view=1`.
5. English and fr-CA; WCAG 2.2 AA: labelled pickers, text alternatives for each drawing, the summary as real text, a
   change of pick announced in the live region.

## Non-goals

- Editing on the compare page. It only reads; edits stay on the street and junction pages.
- Comparing more than two designs at once, or designs of two different places.
- Merging, copying parts from one design to another, or promoting a scenario to baseline.
- Showing a comparison on the map or in the design-review report.
- Keeping the chosen pair anywhere but the page's URL.

## Anti-goals (a failure even if it works)

- The page writes anything: no record, no scenario, no history step, no sync mark. Opening it must leave storage
  byte-identical.
- The drawings differ from what the editor shows for that design (they come from the same model and renderers, not an
  approximation).
- Two drawings at different scales, so that equal widths look different.
- A summary that disagrees with the drawings, or that lists a difference made only of uids or ordering the person cannot see.
- The street and junction pages change in anything but the added Compare link.

## Approaches

Which slice owns the page decides how much of the dependency rule (a slice uses another's `model` only, never its
`vm`, views or text) bends.

- **A. Street and junction each own their half (chosen).** `street::compare` and `junction::compare` (each with its
  view-model and views), `mount_compare_street` / `mount_compare_junction`; `lib.rs` mounts the one the URL names. The
  page frame and the pair of design pickers are shared (`shared/compare.rs`, beside `scenarios_view`). The street
  half reuses `street::vm::impact` (use widths, class, checks) and `check_label`, its plan (`plan_svg`) and 2.5D
  (`street::view::drawing`) as they are. No change to the dependency rule. Cost: two page bodies; a junction diff
  is new.
- **B. A new `compare/` slice.** One owner, but `impact.rs` and `check_label` would move from `street/vm` to
  `street/model`, or the rule would gain exceptions: churn in two slices for no gain to the person.
- **C. In the map's report.** It already draws both figures, but it reads baselines only and is not a page of its own.

Chosen: A, on a separate page (`compare.html`), not inside the editor pages or the map.

## What the person sees

```
[top bar: Projects  About                                   account]
Compare designs · Rue Saint-Denis                  ‹ Back to the street
 Design A [Baseline        ▾]      Design B [Wider bike lane ▾]
┌──────────── A: Baseline ──────────┐┌──────── B: Wider bike lane ───────┐
│ plan (street only)                ││ plan                               │
│ 2.5D                              ││ 2.5D                               │
└───────────────────────────────────┘└────────────────────────────────────┘
What differs in B
 Space   Cycling: 1.5 m → 2.0 m (0.5 m more) · Parking: 2.4 m → 0 m (2.4 m less)
 Lanes   Northbound: 2 → 1
 Pieces  Removed: parking lane (east) · Changed: bike lane 1.5 m → 2.0 m
 Checks  Passes only in B: Bike lane width · Fails only in B: Loading access
```

- Columns side by side from 900 px wide; stacked (A above B) below that, with no sideways scroll at 280 px.
- Each column is a `section` headed by its design's name; each drawing has an accessible name (the existing
  `alt`/`iso_label` text) and the street's plan the plan's.
- The two drawings of a kind are drawn at one scale: the 2.5D through the existing `figures` pair functions; the plans
  at one millimetres-per-pixel, from the longer street.
- The summary is a heading and lists; an empty group is left out; identical designs say so in one line.
- Changing a picker redraws both columns' scale and the summary, rewrites `a`/`b` in the URL (replace, not push), and
  says the summary's first line in the live region.

## URL and entry

`compare.html?street=<edge>` or `?junction=<node>`, plus `project=<id>`, optional `view=1`, `a=<id>` and `b=<id>`
(`base` or a scenario id). Defaults: A the baseline; B the scenario shown on the page that linked here, else the
first scenario, else the baseline. An `a`/`b` naming no scenario of the place falls back to the baseline and says
`scenario-missing`. No place, or a place the project does not hold: the page says so (as `street.html` does for a
missing street) and offers the Projects link.

The scenario panel gets a "Compare" link (`#scenario-compare`) while the place keeps at least one scenario, its
`b` the scenario shown. `compare.html` joins `shared::project_id::PROJECT_PAGES` so the shell keeps `project` and
`view=1` on its links. A "Back" link returns to the place's editor page.

## Structure

- `shared/compare.rs`: the page frame and `PairPicker` (two labelled `select`s over `ScenarioItem`s, the baseline
  first), a `CompareState` (A, B as `Option<u32>`) and the URL read/write through the ports, with no slice knowledge.
- `street/compare/` and `junction/compare/` following the slice's layout rule (only `mod.rs` at the slice root is
  allowed, so this is `street/vm/compare.rs` + `street/view/compare.rs`, and a pure diff in `street/model/` or
  `street/vm/` as fits; same for junction). Each view-model opens a read-only `CityBinding` and takes
  `street_editor(a)` / `street_editor(b)` (`junction_editor`), keeps their `View`s, and offers presentation-ready
  getters: names, figures/SVG, alt texts, the summary as `Said` groups.
- Street summary: `impact::diff` over `Input::of(view_a)`/`Input::of(view_b)` (with the area findings) for uses, class
  and checks, with a `Cause::Compare` (or a separate entry point) so no "Undone"/edit headline is made; plus a new pure
  diff of lanes per direction and pieces by uid.
- Junction summary: a new pure diff of two `JView`s (control, turbo, arms by uid, signal, checks by id).
- `lib.rs`: `mount_compare` reads `street`/`junction` from the URL and calls the slice's mount; `web/compare.js`
  loads the module and calls it, then `.mount_shell()`.
- Messages: `cmp-*` in the street and junction `i18n` (and the shared frame's in `city/i18n` where `scenario-*`
  live), both languages, `i18n_guard` `MIGRATED`.

## Constraints

- Logic in Rust; `web/compare.js` is start-up glue only.
- View-models tested natively against `test_ports()`; diffs are pure and unit-tested; views get component tests.
- `RUSTFLAGS=-D warnings`, `just format`, version bump (minor) with `cargo update -p cityloom_editor`.
- New spec `e2e/tests/compare.spec.ts` in the UI tier (`e2e/tiers.json`) and the `cross` group of
  `scripts/ci/spec-map.json`; `just test-scripts` must pass. New colour pairs go in `shared/contrast.rs`.
- Documentation: root `CLAUDE.md` (pages list), `city/CLAUDE.md` ("Not yet: comparison"), `shared/CLAUDE.md`, the
  street and junction `CLAUDE.md`, and the user docs in `website/` if they describe scenarios.

## Left to the builder

- Exact wording in both languages (fr-CA by `docs/glossary-fr-ca.md` and the OQLF rules).
- How a zoned street's pieces are compared when the two designs have different stretches (at least: say the
  stretches differ, and compare the base sections).
- Which 2.5D stretch is drawn for a zoned street (the one the report draws).
- Whether the page follows sync updates live or only on load (follow them if the editor pages' mechanism reuses cheaply).

## Settled from the code (the designer's calls)

- **No write on open.** `project::context::open_page` records the project as the one opened last; the compare page
  resolves its project with `context::peek` (or an equivalent that writes nothing), so the anti-goal holds.
- **Junction drawings at one scale.** `junction::view::figure::figures` gives a report `Figure`, not page SVG. A pair
  renderer for the page (both 2.5D diagrams at one scale, like the street's) is added beside `junction/view/iso`.
- **Uses on both sides.** `impact::diff` lists only uses the later side has; the comparison takes the union of both
  sides' uses, so a use that drops to zero ("Parking: 2.4 m → 0 m") is listed. The editor's impact summary gets the
  same fix only if its own tests show the gap matters there; otherwise it is left as it is.
- **Back lands on the baseline.** The editor pages always open on the baseline (`ScenarioState.active` is per page
  load); the Back link says "Back to the street" / "Back to the junction", not to a design.
