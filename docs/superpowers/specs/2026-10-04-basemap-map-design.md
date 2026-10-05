# Map view on a styled OpenStreetMap basemap

Date: 2026-10-04

## Goal

The city map (`web/map.html`) shows a real OpenStreetMap basemap, in a CityLoom
style that follows the light and dark themes, instead of the SVG drawing of the
city layout. The editable streets and junctions stay on the map as an overlay
drawn from Rust: coloured by whether they work, and clickable to open the
street or junction editor.

## Decisions taken with the user

- The map page's drawing is **replaced** by the basemap and an overlay.
- Tiles are **self-hosted PMTiles**: no API key, nothing sent to a third party.
- The renderer is **MapLibre GL JS**, vendored. It is third-party code and the
  one exception to "JS in `web/` is start-up glue".
- The editable streets and junctions stay as a **Rust-owned overlay**. Lane and
  hatch detail is not drawn on the map; it stays in the street and junction
  editors.
- **Tiles are required.** If the PMTiles file cannot be loaded the map page says
  so and how to build it. There is no flat-background fallback.
- Coverage is the **Montréal metropolitan area**, the same extent as
  `scripts/metro_tiles.py`. A place outside it gets a "no basemap here" message
  on the map page; its street and junction editors and the home page's search
  still work.
- The **sample city is removed from the map page**. The map page always shows a
  real area (the default one, the Plateau Mont-Royal, until one is chosen). If
  the area's roads cannot be had, the page says so and has nothing to draw. The
  sample city stays on the home page's hero and in the editors' sandbox.
- Tiles are built with **Planetiler**.
- **Print is dropped on the map page.** Its menu has no Print button; the other
  pages keep theirs.
- A street keeps its **actual geometry**. The overlay follows the road's real
  centreline, not a straight line between its two nodes.

## What stays

The home page's hero (`map/home.rs`, `HeroMap`) is a quiet, non-interactive copy
of the SVG map behind the search card. It is not the map page and is not
changed: `map/svg.rs` and `map/camera.rs` stay for it, and it keeps drawing
straight streets. Only the map page stops using them. `map/gestures.rs`, which
only the map page used, goes.

## Data and build

- `scripts/basemap_tiles.py` and `just basemap-tiles quebec-latest.osm.pbf` run
  Planetiler's release jar (OpenMapTiles schema) on the same Geofabrik extract
  as `metro-tiles`, limited to the metro bounds, and write
  `web/data/basemap/montreal.pmtiles` (gitignored, like `web/data/metro`).
  Planetiler needs Java 21 and, with `--download`, fetches about 1 GB of
  Natural Earth and water-polygon sources once. The README says so beside the
  osmium note.
- The basemap style is two files, `web/basemap-light.json` and
  `web/basemap-dark.json`, written by `scripts/make_basemap_style.mjs` from one
  palette object so the two variants do not drift (the generated files are
  committed). Colours follow the app's CSS tokens (`--land`, `--ink`, …). The
  glue picks the variant from `data-theme`, or `prefers-color-scheme` when
  unset, and follows a change.
- Glyphs: the app has no font files (it uses the system's), and MapLibre draws
  labels from glyph PBFs. Noto Sans Regular and Bold glyph ranges for Latin
  (0–255, 256–511) and general punctuation (8192–8447) are taken from
  `openmaptiles/fonts` (SIL Open Font License) into `web/glyphs/`, committed,
  and credited in `credits.html`.
- `maplibre-gl` (ES modules: `maplibre-gl.mjs`, `-shared.mjs`, `-worker.mjs`,
  and the stylesheet) and `pmtiles` (`dist/pmtiles.js`, which defines a global
  `pmtiles`) come from npm at their latest versions, from a new root
  `package.json`, and are copied into `web/vendor` (gitignored) by
  `scripts/build.sh`. `just setup` runs `npm ci` there; CI caches both lock
  files. `credits.html` gains the libraries' licences (BSD-3-Clause) and the
  OpenMapTiles and OpenStreetMap attributions, and the map shows them in
  MapLibre's attribution control.
- PMTiles is read with HTTP range requests. Neither `python3 -m http.server`
  (`just serve`) nor `e2e/serve.mjs` answers them, so `serve.mjs` gains
  `Range` support and `just serve` runs it. The README notes that a host must
  support range requests.
- Missing tiles: the glue reads the PMTiles header and tells Rust, through the
  same event channel as everything else the map says (see Code), whether it
  loaded, and which area it covers. The words on the page are Rust's.

## Street geometry

The imported `Network` already holds each road's centreline (`Road::points`,
metres). `city/import.rs` reads it only to find the headings at the two ends
and then drops it, so `Layout` edges are straight lines between nodes. This
change keeps it:

- An edge gains a `shape`: its centreline as a polyline in layout millimetres,
  from node `a` to node `b` (the order `Road::points` already runs in, as
  `headings` assumes), with its two ends set to the nodes' positions so the
  overlay meets the junction discs. A road with fewer than two points has no
  shape and is the straight line. `Layout` is rebuilt from the stored network on
  every load (`city/store.rs`), so this changes no stored format; the sample
  city's edges stay straight.
- Merged dual carriageways (`osm_network::merge`) already carry one merged
  centreline; that is the shape kept.
- `headings` is derived as now, so the junction editor is unchanged.
- `EdgeView` gains `shape_mm` (never fewer than two points). Its `length_mm`
  becomes the length along the shape instead of the chord. Nothing reads
  `length_mm` today (the checks do not), so no check changes; a test covers a
  curved street longer than its chord.

## Georeferencing

Overlay and basemap meet at one `Projection` in `map/`, pure and tested
natively: layout millimetres to longitude and latitude.

Settled by reading osm2streets and `geom`:

- With a clip box, `gps_bounds` *is* that box (`streets_reader`, `Document::read`),
  and every area import passes `Area::bounds()` as the clip box. So the origin
  comes from the area, and `Network` does not need to carry it: no stored-format
  change and no migration.
- The mapping is linear in longitude and latitude over the box, scaled to the
  box's size in metres: width is the haversine distance along the south edge and
  height along the west edge (`GPSBounds::get_max_world_pt`), with a 6,371,000 m
  earth radius. It is not Web Mercator and does not vary with latitude inside the
  box. `osm_import` then flips y, so `Network` positions are metres east and
  north of the box's south-west corner.
- `Layout::from_network` re-bases the layout to the bounding box of the nodes it
  keeps (`x_mm = (x_m − min_x)·1000`, `y_mm = (max_y − y_m)·1000`), so the
  layout must remember `(min_x, max_y)` — `CityView` carries it as `origin_m`
  (absent for the sample city, which therefore has no map).

A test in `osm_import` checks the `Projection` against `geom`'s own
`GPSBounds::convert_back_xy` over the Plateau box. This is the riskiest part of
the work, since an error of a few metres shows against the basemap.

## Code

- `map/` after the change:
  - `gestures.rs` and the view-model's pointer, wheel and panning state are
    removed.
  - `vm.rs` keeps the places, checks, search and selection logic, and its camera
    only for what the hero uses (size, insets, fit). Zoom, pan and fit on the map
    page become commands on the `Mapper` port. It gains the basemap's state
    (waiting, ready, or unavailable with the words to say) and handles what the
    map reports.
  - `projection.rs` (new) is the `Projection`.
  - `overlay.rs` (new) turns a `CityView` into GeoJSON: each street as a
    `LineString` along its shape, with `hot` (`s-7`), `status` and `width_m`
    properties, and each junction as a `Point` with `hot` (`j-3`), `status` and
    `radius_m`. Widths are drawn in metres by a zoom expression, not as polygons.
    No simplification is done here; MapLibre simplifies GeoJSON itself.
  - `style.rs` (new) holds the overlay layers (MapLibre layer JSON): status
    colours, the minimum drawn width, the hover and selected states. The metres
    to pixels factor uses the area's latitude, so the layers are made in Rust.
  - `view.rs`: `MapView` becomes the behaviour on the basemap element (the
    keyboard, the page's insets, and keeping the map in step with the
    view-model) plus the message when the basemap is unavailable. The "Key to
    the streets" legend, which shows lane tints and hatches the map no longer
    draws, becomes a key to the places' state (works, needs attention,
    changed). The key help text is updated: the places are reached through the
    Places list and the map's own pick, since the canvas has no focusable places.
- `Mapper` is a new port in `shared/ports.rs` with a fake for native tests; the
  browser side in `platform.rs` wraps a JS adapter. It takes `set_places`,
  `fit` (bounds and the insets), `zoom_by`, `pan_by`, `highlight` and
  `set_imperial` (for the scale control), and reports `MapEvent`s: `Ready`
  (with the tile coverage), `Failed`, `Pick`, `Hover` and `Moved`. Pages that
  have no map get a `NoMapper`.
- `web/basemap.js` is the adapter, the one place MapLibre is touched: it
  registers the pmtiles protocol, builds the map from the style for the current
  theme (north up, no rotation, no built-in keyboard handling so the shortcuts
  stay in Rust), re-adds the places after a theme change, and turns MapLibre's
  events into the JSON `MapEvent`s. `web/map.js` creates it and hands it to
  `mount_map`.
- The keyboard shortcuts (`+`, `−`, arrows, `0`) stay in Rust and call the
  port. The Places list keeps its focus order and announcements.
- `web/map.html` gains the basemap element and loses the print button and the
  hatch definitions it no longer uses.

## Tests

- Native: the projection (corners, centre, against `geom` in `osm_import`), the
  kept shape (end order and snapping, a road with one point, length along a
  curve), the overlay GeoJSON (one feature per place, status, width, a curved
  street follows its centreline), the layers, and the view-model against the fake
  `Mapper` (a pick opens a place, a hover is hot, a shortcut becomes a command,
  fit uses the insets, coverage outside the area and a failure say so, the
  sample city says it has no map). Replaced: the view-model tests of the camera
  commands, the wheel and the drag, and the map view's SVG component tests.
- Browser: a small committed fixture `e2e/fixtures/plateau.pmtiles`, cut from the
  same area as `plateau.osm.pbf`, served with `page.route` and range requests as
  `world.spec.ts` serves its tile. `map.spec.ts`, `search.spec.ts` and the map
  case of `shell.spec.ts` move to the area from the sample city, because the map
  page no longer shows the sample. They check that the map mounts with no page
  error, that the basemap loads, that a place opens its editor, that a failing
  place is marked, that the theme changes the style, and that missing tiles, a
  place outside coverage, and an area that did not load each say so. Pointer and
  canvas behaviour is covered only here. WebGL in the headless browser is
  proved first, before the rest is built on it.

## Out of scope

- Drawing lanes or hatches on the map.
- Drawing the real shape in the street and junction editors and in the home
  hero, which still show straight streets; only the map page follows the
  centreline.
- A basemap beyond the Montréal metropolitan area.
- Print on the map page.
- A second basemap style or a style editor.
