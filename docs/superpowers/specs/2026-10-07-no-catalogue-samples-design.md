# No catalogue samples: the network is the only source of streets and junctions

Date: 2026-10-07

## Goal

Make the code match `docs/erd.mmd`. The ERD has no `CATALOGUE_STREET`, no
`CATALOGUE_JUNCTION_SAMPLE` and no `NAMES`. A street and a junction exist only
because the network (OpenStreetMap, through `osm_import`) says so. The code
keeps no template catalogue of streets or junctions.

## Where the code stands

The merged `feat/no-sample-city` work already removed the sandbox pages and the
sample city from the product: a page opens the area's city, and the sample city
is built for the tests only. What is left, and what this change removes:

- `SAMPLES` (`shared/catalogue.rs`): four street templates, used as a street's
  class (`EdgeDef.street`, `Street.sample`, `Arm.street`), its fallback name
  and width, and its `freeway` flag.
- `JUNCTION_SAMPLES` (`junction/model.rs`): four junction templates.
- `Names` / `NodeDef.names` (`city/model.rs`): titles cached at import.
- The test-only sample city (`Layout::sample()`, `Street::sample(n, side)`,
  `CityStore::sample`).
- Editor operations built on the templates: `Junction::add_arm(street, bearing)`
  and the inspector's "this arm becomes street X" list.

## Decisions taken with the user

- **Everything goes**: both catalogues, `Names`, and the test-only sample city.
- **A street has a class**: `StreetClass { Motorway, Arterial, Collector, Local }`
  replaces every index into `SAMPLES`. `freeway` is `class == Motorway`.
- **No adding arms.** Arms are what the network has. Editing and removing arms
  stay. (Adding an arm of an existing street, or by class with a default
  section, were rejected.)
- **Stored data is discarded.** There is no migration. A kept city or network
  that does not read under the new shape is dropped and the area is imported
  again.
- The sandbox pages and the sample city in the product are already gone.

## Design

### Data

- `StreetClass` lives in `shared` (it is used by `street`, `junction` and
  `city`; `shared` depends on no slice). It serialises as a lowercase string.
- `Street` (`street/model.rs`): `sample: usize` becomes `class: StreetClass`.
  `name` is always set, so the "template name" fallback and `Street::title`'s
  fallback go. `row_mm`, `side`, `segments` are unchanged.
- `EdgeDef` (`city/model.rs`): `street: usize` becomes `class: StreetClass`;
  `name` is required. `section` stays and always comes from the import.
- `Arm` (`junction/model.rs`): `street: usize` is dropped; an arm reads its
  street's section, name, width and class through its edge uid. An arm with no
  edge no longer exists.
- `Junction`: `sample` is dropped. A junction is built from the arms the
  network gives it, never from a template.
- `Names` and `NodeDef.names` are removed. `node_name` / `end_name` derive from
  the incident edges' names and the node's degree: a junction is the joined
  distinct names, a dead end is "End of X", a connection is "Connection on X".
  The sentence form ("the end of X") is produced in `text.rs`, not stored.

### Import

`class_of(highway)` returns a `StreetClass`: `motorway`, `motorway_link` →
Motorway; `trunk`, `primary` (and their links) → Arterial; `secondary`,
`tertiary` (and their links) → Collector; everything else, `service` included →
Local. An unnamed road is still named "Unnamed <highway>" at import.

### Behaviour that read the template

- Junction footprint (`city/model.rs` `row_mm / 2`): from the street's own
  `row_mm`.
- `freeway` on a place and on measures/checks: from `class == Motorway`.
- An arm may not be a freeway: the check reads the class.
- "Becomes street X": removed with adding arms.
- `junction_catalogue()` and `lib.rs`'s street/sample JSON: the sample and
  street lists go; controls, limits and rules stay.

### Tests

- Test-first: each stage starts with a failing test.
- Rust tests that used `Layout::sample()`, `Street::sample(n, side)` or
  `JUNCTION_SAMPLES` build their places through the importer from a small
  `osm_network` fixture helper under `cfg(test)`. Nothing sample-shaped ships.
- Tests that assert on the templates (`samples_fill_their_right_of_way_exactly`,
  `samples_respect_catalogue_ranges`, the `JUNCTION_SAMPLES` loops) are deleted.
- E2E: no change expected (the specs already start on the Plateau fixture);
  any that touch the arm palette or "becomes" are removed.

### ERD

`docs/erd.mmd`: `STREET.sample` becomes `STREET.class` (a `StreetClass`);
`JUNCTION.sample` goes; the `ARM` edge relationship drops "0 = sample". The
`.svg` and `.png` are regenerated.

## Order of work

One commit each, `just test` green at every step, `just e2e` at the end:

1. Derive node names from the incident streets; remove `Names`.
2. Add `StreetClass`; import maps `highway` to it; `Street.sample` and
   `EdgeDef.street` become `class`; the freeway rule reads it.
3. Remove `add_arm`, the "becomes" list and `Arm.street`; arms read their edge.
4. Remove `JUNCTION_SAMPLES` and `Junction.sample`.
5. Build test data through the importer fixture; remove `Layout::sample()`,
   `Street::sample` and `CityStore::sample`.
6. Delete `SAMPLES` and the JSON exports.
7. ERD: `class` replaces `sample`; regenerate the images.

## Out of scope

- The segment catalogue (`KINDS`, `MATERIALS`, `CURBS`, `REGIONS`): the ERD
  keeps `SEGMENT_KIND`, `MATERIAL`, `CURB`, `MODE`.
- Changing how roads are imported beyond the class mapping.
