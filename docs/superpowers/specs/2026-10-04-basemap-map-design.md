# Map view on a styled OpenStreetMap basemap

Date: 2026-10-04

## Goal

The city map (`web/map.html`) shows a real OpenStreetMap basemap, in a CityLoom
style that follows the light and dark themes, instead of the SVG drawing of the
city layout. The editable streets and junctions stay on the map as an overlay
drawn from Rust: coloured by whether they work, and clickable to open the
street or junction editor.

## Decisions taken with the user

- The map drawing is **replaced**. The SVG city drawing (`map/svg.rs`) goes.
- Tiles are **self-hosted PMTiles**: no API key, nothing sent to a third party.
- The renderer is **MapLibre GL JS**, vendored. It is third-party code and the
  one exception to "JS in `web/` is start-up glue".
- The editable streets and junctions stay as a **Rust-owned overlay**. Lane and
  hatch detail is not drawn on the map; it stays in the street and junction
  editors.
- **Tiles are required.** If the PMTiles file cannot be loaded the map page says
  so and how to build it. There is no flat-background fallback.
- Coverage is the **Montréal metropolitan area**, the same extent as
  `scripts/metro_tiles.py`. A place outside it gets the missing-basemap message
  on the map page; its street and junction editors and the home page's search
  still work.
- The **sample city is removed from the map page**. The map page always shows a
  real area (the default one, the Plateau Mont-Royal, until one is chosen). If
  the area's roads cannot be had, the page says so and shows nothing to draw.
- Tiles are built with **Planetiler**.
- **Print is dropped on the map page.** The shell's Print button is hidden
  there; the other two pages keep it.

## Not decided here (to settle in the plan)

- A road's real shape is not kept today (`city/import.rs` draws straight lines
  between nodes), so on a real basemap the overlay will cut across curved
  roads. Keeping `Road::points` in `Layout` is a separate change. This spec
  accepts the straight overlay and states it in the page's notes; it does not
  fold that change in.

## Data and build

- `scripts/basemap_tiles.py` and `just basemap-tiles quebec-latest.osm.pbf` run
  Planetiler on the same Geofabrik extract as `metro-tiles` and write
  `web/data/basemap/montreal.pmtiles` (gitignored, like `web/data/metro`).
  Layers: water, landuse and parks, buildings, roads by class, and labels.
  Planetiler needs Java; the README says so beside the osmium note.
- `web/map-style.json` is the MapLibre style, with a light and a dark variant
  chosen from `data-theme` (and `prefers-color-scheme` when unset). Colours
  come from the app's CSS tokens, generated from them by a script so the two
  do not drift. Glyphs use the fonts in `web/fonts`, converted to the PBF
  glyph format MapLibre needs.
- `maplibre-gl` and `pmtiles` come from npm at their latest versions, copied
  into `web/vendor` by `scripts/build.sh` (`just setup` runs `npm ci`).
  `credits.html` gains the two libraries' licences and the OpenStreetMap and
  tile-schema attributions; MapLibre's attribution control shows the
  OpenStreetMap credit on the map.
- Missing tiles: the page's start-up glue tries the PMTiles header; on failure
  it calls a Rust export that puts the message on the page (the words are in
  Rust, per the architecture).

## Georeferencing

Overlay and basemap meet at one `Projection`, new in `map/`, pure and tested
natively: layout metres to and from longitude and latitude.

The imported layout is in metres from the south-west corner of osm2streets'
`gps_bounds`, with y flipped (`osm_import/src/lib.rs`, `convert`). The saved
network does not say where that corner is. Two things have to be settled by
reading osm2streets, before any code: the exact projection it uses (my
expectation is a linear scaling with the cosine of the mid latitude, not
Web Mercator, which I have not confirmed), and whether `gps_bounds` is the
clip box or the box of the data read. The answer decides whether
`osm_network::Network` carries its origin and scale (a stored-format change,
with a migration for networks already kept under `cityloom-network:<key>`) or
whether the origin can be taken from `Area::bounds()`. A test must place a
known OSM node on the overlay and on the basemap at the same point; this is the
riskiest part of the work, since an error of a few metres shows.

## Code

- `map/` after the change:
  - `svg.rs`, `camera.rs`, `gestures.rs` and their tests are removed.
  - `vm.rs` keeps the places, checks, search and selection logic. Its camera
    and zoom state go; zoom, pan and fit become commands on the `Mapper` port.
  - `overlay.rs` (new) turns a `CityView` into GeoJSON: each street as a
    polygon of its real width, with `place`, `status` and `label` properties,
    and each junction as a disc.
  - `style.rs` (new) holds the overlay layers' paint rules (the pass and fail
    colours, hover, selection), so the colours stay in Rust.
  - `view.rs` binds the page to the view-model as before, minus the SVG.
- `Mapper` is a new port in `shared/ports.rs`, with a fake for native tests and
  its browser side in `platform.rs`. It takes `set_overlay`, `fit`,
  `zoom_by`, `pan_by`, `highlight` and reports `on_pick`. The view-model reaches
  the map only through it.
- `web/map.js` stays glue: it registers the pmtiles protocol, creates the
  MapLibre map with the style, and hands it to `mount_map`.
- The keyboard shortcuts (`+`, `−`, arrows, `0`) stay in Rust and call the
  port. The Places list keeps its focus order and announcements.
- `web/map.html` loses the print button for this page and gains the MapLibre
  container and attribution.

## Tests

- Native: the projection (round trip, and a known node), the overlay GeoJSON
  (width, status, one feature per place), the view-model against the fake
  `Mapper` (a pick selects a place; a shortcut becomes a command; zoom and fit
  commands), and the missing-basemap message.
- Browser (`e2e/tests/map.spec.ts`): a small committed fixture
  `e2e/fixtures/plateau.pmtiles`, cut from the same area as
  `plateau.osm.pbf`, served with `page.route` as `world.spec.ts` serves its
  tile. It checks that the map mounts with no page error, that a place opens
  its editor, that a failing place is marked, that the dark theme changes the
  style, and that missing tiles show the message. `shell.spec.ts` stops
  expecting the sample city on the map page. Pointer and canvas behaviour is
  covered only here.
- `just test` and `just e2e` run as before; the build copies the vendored
  libraries into `web/vendor`.

## Out of scope

- Keeping a road's real shape, and drawing lanes or hatches on the map.
- A basemap beyond the Montréal metropolitan area.
- Print on the map page.
- A second basemap style or a style editor.
