# Map view on a styled OpenStreetMap basemap — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The city map page shows a self-hosted OpenStreetMap basemap (PMTiles, MapLibre GL, a CityLoom style in light and dark) with the editable streets and junctions drawn over it from Rust, following each road's real centreline.

**Architecture:** A new `Mapper` port (`shared/ports.rs`) is all the map view-model knows of the map; `web/basemap.js` is the one adapter that touches MapLibre and talks to Rust in JSON events. Rust owns the projection (layout millimetres to lon/lat, taken from the area's bounds), the overlay GeoJSON and the overlay's layer definitions. `Layout` edges keep the road's centreline, and the layout remembers its origin so it can be placed on the earth. The home hero keeps the SVG drawing.

**Tech Stack:** Rust 2024 + Leptos 0.8 (wasm), `maplibre-gl` 6.x and `pmtiles` 4.x (vendored from npm), Planetiler (OpenMapTiles schema) for the tiles, Playwright.

**Spec:** `docs/superpowers/specs/2026-10-04-basemap-map-design.md` (commit `af85ca7`)

## Global Constraints

- Logic belongs in Rust; JS in `web/` is start-up glue. The one exception is `web/basemap.js`, which wraps MapLibre and contains no decisions.
- `shared` must not depend on any slice. A slice may use another slice's `model`, `city::store`/`city::binding`, `shell::Target` and `shared`; never another slice's `vm`, views or text. (`map` using `place::Bounds` and `place::area::Area` follows how `city::store` already uses `Area`.)
- View-models reach the browser only through the ports and are tested natively against fakes (`test_ports()`).
- CI runs with `RUSTFLAGS=-D warnings`: no new warning. Run `just format` before committing.
- Test-driven: write the failing test first. Commit each task separately once its tests pass. End every commit message with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Always use the latest version of any dependency (`maplibre-gl`, `pmtiles`, Planetiler's `releases/latest`).
- Tiles are required: a missing or non-covering basemap shows a message on the map page; there is no flat-background fallback. The sample city has no map.
- osm2streets' mapping is linear in lon/lat over the area's box, scaled by the haversine width (south edge) and height (west edge) with a 6,371,000 m radius; `Network` positions are metres east and north of the box's south-west corner. `Layout` positions are re-based: `x_mm = (x_m − min_x)·1000`, `y_mm = (max_y − y_m)·1000`.
- PMTiles needs HTTP range requests; every server used (`e2e/serve.mjs`, `just serve`, test routes) must answer them.
- The Mapper's pan contract: `pan_by(dx, dy)` in pixels, positive `dx` moves the view east (right), positive `dy` moves it south (down).

## Review Focus

Failure modes the spec implies that no single task's headline test covers. Each has a test in the task named.

1. **A road with one point, no points, or a zero-length shape** must not panic or draw nothing: it is the straight line between its nodes (Task 1).
2. **A place whose area is outside the tiles' coverage, or whose tiles 404**, must say so on the page, not leave a blank map or throw a page error (Tasks 5 and 9).
3. **A theme change while places and a highlight are showing** must not lose them: `setStyle` drops every layer (Task 9).
4. **Keyboard-only use:** the canvas has no focusable places, so every place must stay reachable and openable through the Places list, and the map's own shortcuts must work with focus on the map (Tasks 8 and 9).
5. **The overlay must sit on the basemap's roads**, not merely on a self-consistent projection: checked against rendered tile features (Task 9).

---

### Task 1: Streets keep the shape of their road, and the layout remembers its origin

**Files:**
- Modify: `src/city/model.rs` (`EdgeDef`, `street()`, `Layout`, `Layout::sample`, `Layout::dist_mm`, `City::view`, `EdgeView`, `CityView`)
- Modify: `src/city/import.rs` (`from_network`, new `shape_mm`, tests)

**Interfaces:**
- Produces: `EdgeView.shape_mm: Vec<[i32; 2]>` (layout millimetres, from node `a` to node `b`, at least two points; ends equal the nodes' `x_mm`/`y_mm`); `EdgeView.length_mm` is the length along it; `CityView.origin_m: Option<[f64; 2]>` = network metres `[min_x, max_y]` of the layout's origin (`None` for the sample city).

- [ ] **Step 1: Write the failing tests**

Append inside `mod tests` of `src/city/import.rs` (it already has `curving_t()` and `crossing()`; `curving_t` is a T whose roads have 3, 3 and 4 points and whose far nodes lie at (−100,100), (100,100), (60,50)):

```rust
    #[test]
    fn a_street_keeps_its_roads_shape_with_its_ends_on_its_nodes() {
        let v = City::from_network(&curving_t(), "Curves").view(0);
        let node = |uid: u32| v.nodes.iter().find(|n| n.uid == uid).unwrap();
        let mut lens: Vec<usize> = v.edges.iter().map(|e| e.shape_mm.len()).collect();
        lens.sort_unstable();
        assert_eq!(lens, vec![3, 3, 4], "a point for each point the road has");
        for e in &v.edges {
            assert_eq!(e.shape_mm[0], [node(e.a).x_mm, node(e.a).y_mm], "starts on node a");
            assert_eq!(*e.shape_mm.last().unwrap(), [node(e.b).x_mm, node(e.b).y_mm], "ends on node b");
        }
    }

    #[test]
    fn a_curved_street_is_longer_than_the_line_between_its_ends() {
        let v = City::from_network(&curving_t(), "Curves").view(0);
        let e = &v.edges[0]; // (0,0) -> (-30,0) -> (-100,100): 30 m, then about 122.07 m
        let (a, b) = (e.shape_mm[0], e.shape_mm[2]);
        let chord = ((a[0] - b[0]) as f64).hypot((a[1] - b[1]) as f64);
        assert!((e.length_mm - 152_066).abs() <= 2, "{}", e.length_mm);
        assert!(e.length_mm as f64 > chord + 5_000.0);
    }

    #[test]
    fn a_road_with_no_centreline_or_one_point_is_the_straight_line_between_its_nodes() {
        for points in [vec![], vec![(0.0, 0.0)]] {
            let mut net = curving_t();
            net.roads[0].points = points;
            let v = City::from_network(&net, "Bare").view(0);
            let e = &v.edges[0];
            assert_eq!(e.shape_mm.len(), 2);
            let node = |uid: u32| v.nodes.iter().find(|n| n.uid == uid).unwrap();
            assert_eq!(e.shape_mm[0], [node(e.a).x_mm, node(e.a).y_mm]);
            assert_eq!(e.shape_mm[1], [node(e.b).x_mm, node(e.b).y_mm]);
        }
    }

    #[test]
    fn the_layout_remembers_where_its_origin_lies_in_the_network() {
        assert_eq!(City::from_network(&curving_t(), "Curves").view(0).origin_m, Some([-100.0, 100.0]));
        let sample = City::new().view(0);
        assert_eq!(sample.origin_m, None);
        assert!(sample.edges.iter().all(|e| e.shape_mm.len() == 2), "the sample's streets are straight");
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test city::import 2>&1 | tail -20`
Expected: FAIL to compile — `no field shape_mm on type EdgeView` / `origin_m`.

- [ ] **Step 3: Implement in `src/city/model.rs`**

In `pub(super) struct EdgeDef`, after `headings`:

```rust
    /// The street's centreline in layout millimetres, from node `a` to node `b`, where its real shape is
    /// known. Otherwise it runs along the straight line between them.
    pub(super) shape: Option<Vec<(i32, i32)>>,
```

`const fn street(...)`: `EdgeDef { a, b, street, name: None, section: None, headings: None, shape: None }`.

In `pub struct Layout`, after `edges`:

```rust
    /// Where the layout's origin lies in the network it was made from, in metres east and north of the
    /// network's own origin. The sample city is on no network, so has none.
    pub(super) origin_m: Option<(f64, f64)>,
```

`Layout::sample()`: add `origin_m: None`.

Replace `fn dist_mm` in `impl Layout` with:

```rust
    /// The street's centreline from node `a` to node `b`, in millimetres: its own shape, or the straight line.
    fn shape_of(&self, edge: usize) -> Vec<(i32, i32)> {
        let e = &self.edges[edge];
        e.shape.clone().unwrap_or_else(|| {
            let (a, b) = (&self.nodes[e.a], &self.nodes[e.b]);
            vec![(a.x_mm, a.y_mm), (b.x_mm, b.y_mm)]
        })
    }
```

Add a free function near `impl Layout`:

```rust
/// How long a line through these points is, in millimetres.
fn path_mm(points: &[(i32, i32)]) -> f64 {
    points.windows(2).map(|w| ((w[1].0 - w[0].0) as f64).hypot((w[1].1 - w[0].1) as f64)).sum()
}
```

In `City::view`, inside `for (i, e) in self.layout.edges.iter().enumerate()` add `let shape = self.layout.shape_of(i);` before `edges.push(EdgeView {`, and replace the `length_mm:` line with:

```rust
                length_mm: path_mm(&shape).round() as i32,
                shape_mm: shape.iter().map(|&(x, y)| [x, y]).collect(),
```

In `pub struct EdgeView`, after `length_mm`:

```rust
    /// The street's centreline from node `a` to node `b`, in millimetres: two points or more.
    pub shape_mm: Vec<[i32; 2]>,
```

In `pub struct CityView` add `/// Where the layout's origin lies in the network, in metres east and north of its corner; None for the sample city.` `pub origin_m: Option<[f64; 2]>,` and in the construction at the end of `view()`:

```rust
        CityView {
            name: self.layout.name.clone(),
            nodes,
            edges,
            bounds_mm: [x0, y0, x1, y1],
            places,
            failing,
            edited,
            origin_m: self.layout.origin_m.map(|(x, y)| [x, y]),
        }
```

- [ ] **Step 4: Implement in `src/city/import.rs`**

Add above `impl City`:

```rust
/// A road's centreline in layout millimetres with its two ends set to where its nodes are, so that a street
/// meets its junctions; None for a road with fewer than two points.
fn shape_mm(points: &[(f64, f64)], from: (i32, i32), to: (i32, i32), mm_of: &impl Fn(f64, f64) -> (i32, i32)) -> Option<Vec<(i32, i32)>> {
    if points.len() < 2 {
        return None;
    }
    let mut line: Vec<(i32, i32)> = points.iter().map(|&(x, y)| mm_of(x, y)).collect();
    let last = line.len() - 1;
    line[0] = from;
    line[last] = to;
    Some(line)
}
```

In `Layout::from_network`, after `let (min_x, max_y) = ...;` add:

```rust
        let origin_m = (!nodes.is_empty()).then_some((min_x, max_y));
        let mm_of = |x_m: f64, y_m: f64| (((x_m - min_x) * 1000.0).round() as i32, ((max_y - y_m) * 1000.0).round() as i32);
        let node_mm = |id: u32| {
            let n = nodes[index[&id]];
            mm_of(n.x_m, n.y_m)
        };
```

In the `EdgeDef { .. }` literal add `shape: shape_mm(&r.points, node_mm(r.from), node_mm(r.to), &mm_of),` after `headings`. In the final `Layout { .. }` literal add `origin_m,`.

- [ ] **Step 5: Run the tests**

Run: `cargo test city 2>&1 | tail -15`
Expected: PASS, including every existing `city::` test. If `a_street_keeps_its_roads_shape…` reports lengths other than `[3, 3, 4]`, `star()` in the test module places its far nodes differently from what `curving_t` assumes; read `star()` and fix the expectation, not the code.

- [ ] **Step 6: Commit**

```bash
git add src/city/model.rs src/city/import.rs
git commit -m "feat(city): a street keeps the shape of its road, and the layout remembers its origin

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The projection from the layout to the earth

**Files:**
- Create: `src/map/projection.rs`
- Modify: `src/map/mod.rs` (add `pub mod projection;`)
- Create: `osm_import/tests/projection.rs`

**Interfaces:**
- Consumes: `place::Bounds { south, west, north, east }` (degrees), `CityView.origin_m` (Task 1).
- Produces:
  - `Projection::new(bounds: Bounds, origin_m: [f64; 2]) -> Projection`
  - `Projection::width_m(&self) -> f64`, `height_m(&self) -> f64`
  - `Projection::network_lon_lat(&self, x_m: f64, y_m: f64) -> [f64; 2]` (`[lon, lat]`, metres east/north of the box's south-west corner)
  - `Projection::lon_lat(&self, x_mm: i32, y_mm: i32) -> [f64; 2]` (layout millimetres)

- [ ] **Step 1: Write the failing unit tests**

Create `src/map/projection.rs` with the tests only first:

```rust
//! Layout millimetres to longitude and latitude, the way osm2streets lays a box of the earth out in
//! metres: linearly over the box, scaled to its size on the ground.

use crate::place::Bounds;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::place::area::Area;

    fn plateau() -> Bounds {
        Area::new("Plateau", 45.5261, -73.5978).bounds()
    }

    #[test]
    fn the_box_is_as_big_on_the_ground_as_the_area_says() {
        let p = Projection::new(plateau(), [0.0, 0.0]);
        assert!((p.width_m() - 800.0).abs() < 2.0, "{}", p.width_m());
        assert!((p.height_m() - 800.0).abs() < 2.0, "{}", p.height_m());
    }

    #[test]
    fn the_south_west_corner_is_the_network_origin_and_the_far_corner_is_the_north_east() {
        let b = plateau();
        let p = Projection::new(b, [0.0, 0.0]);
        assert_eq!(p.network_lon_lat(0.0, 0.0), [b.west, b.south]);
        let [lon, lat] = p.network_lon_lat(p.width_m(), p.height_m());
        assert!((lon - b.east).abs() < 1e-12 && (lat - b.north).abs() < 1e-12);
    }

    #[test]
    fn layout_positions_are_offset_by_the_origin_and_run_south_as_y_grows() {
        let p = Projection::new(plateau(), [10.0, 20.0]);
        assert_eq!(p.lon_lat(0, 0), p.network_lon_lat(10.0, 20.0));
        assert_eq!(p.lon_lat(2_000, 5_000), p.network_lon_lat(12.0, 15.0));
    }
}
```

Add `pub mod projection;` to `src/map/mod.rs` (alphabetical, after `pub mod home;`).

- [ ] **Step 2: Run to see them fail**

Run: `cargo test map::projection 2>&1 | tail -10`
Expected: FAIL to compile — `cannot find type Projection`.

- [ ] **Step 3: Implement**

Insert above `#[cfg(test)]` in `src/map/projection.rs`:

```rust
/// The earth's radius `geom` uses for distances, in metres.
const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// The distance between two points on the earth, in metres, by the haversine formula.
fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let (dp, dl) = ((lat2 - lat1).to_radians(), (lon2 - lon1).to_radians());
    let a = (dp / 2.0).sin().powi(2) + (dl / 2.0).sin().powi(2) * p1.cos() * p2.cos();
    2.0 * EARTH_RADIUS_M * a.sqrt().asin()
}

/// Where a layout lies on the earth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    bounds: Bounds,
    width_m: f64,
    height_m: f64,
    /// Network metres (east, north of the box's south-west corner) of the layout's origin.
    origin_m: (f64, f64),
}

impl Projection {
    /// The projection of a layout whose origin is `origin_m` in a network read over `bounds`.
    pub fn new(bounds: Bounds, origin_m: [f64; 2]) -> Projection {
        let width_m = haversine_m(bounds.south, bounds.west, bounds.south, bounds.east);
        let height_m = haversine_m(bounds.south, bounds.west, bounds.north, bounds.west);
        Projection { bounds, width_m, height_m, origin_m: (origin_m[0], origin_m[1]) }
    }

    /// The box's size on the ground: along its south edge, and along its west edge.
    pub fn width_m(&self) -> f64 {
        self.width_m
    }

    pub fn height_m(&self) -> f64 {
        self.height_m
    }

    /// `[lon, lat]` of a position in metres east and north of the box's south-west corner.
    pub fn network_lon_lat(&self, x_m: f64, y_m: f64) -> [f64; 2] {
        let b = &self.bounds;
        [b.west + x_m / self.width_m * (b.east - b.west), b.south + y_m / self.height_m * (b.north - b.south)]
    }

    /// `[lon, lat]` of a position of the layout, in millimetres (x east, y south).
    pub fn lon_lat(&self, x_mm: i32, y_mm: i32) -> [f64; 2] {
        self.network_lon_lat(self.origin_m.0 + x_mm as f64 / 1000.0, self.origin_m.1 - y_mm as f64 / 1000.0)
    }
}
```

- [ ] **Step 4: Run the unit tests**

Run: `cargo test map::projection 2>&1 | tail -10`
Expected: PASS (3 tests).

- [ ] **Step 5: Write the cross-check against `geom`, the library osm2streets uses**

Create `osm_import/tests/projection.rs`:

```rust
//! The editor's projection must agree with the one osm2streets laid the network out with.

use cityloom_editor::map::projection::Projection;
use cityloom_editor::place::area::Area;
use geom::{GPSBounds, LonLat};

#[test]
fn the_projection_is_the_inverse_of_geoms_over_the_plateau() {
    let b = Area::new("Plateau", 45.5261, -73.5978).bounds();
    let gps = GPSBounds::from(vec![LonLat::new(b.west, b.south), LonLat::new(b.east, b.north)]);
    let size = gps.get_max_world_pt();
    let (w, h) = (size.x(), size.y());
    let p = Projection::new(b, [0.0, 0.0]);
    assert!((p.width_m() - w).abs() < 1e-6 && (p.height_m() - h).abs() < 1e-6, "size {} x {} against {w} x {h}", p.width_m(), p.height_m());
    for (fx, fy) in [(0.0, 0.0), (1.0, 1.0), (0.5, 0.5), (0.2, 0.9), (0.9, 0.1)] {
        // the importer measures y up from the south; geom measures it down from the north
        let (x, y_north) = (fx * w, fy * h);
        let want = gps.convert_back_xy(x, h - y_north);
        let got = p.network_lon_lat(x, y_north);
        assert!((got[0] - want.x()).abs() < 1e-9 && (got[1] - want.y()).abs() < 1e-9, "{fx},{fy}: {got:?} against {} {}", want.x(), want.y());
    }
}
```

- [ ] **Step 6: Run it**

Run: `cargo test -p osm_import --test projection 2>&1 | tail -15`
Expected: PASS. If the size assertion fails, `geom`'s `gps_dist` differs from the haversine written here (read `~/.cargo/git/checkouts/geom-*/*/src/gps.rs`) and `haversine_m` must be made to match it, because the whole overlay depends on that agreement.

- [ ] **Step 7: Commit**

```bash
git add src/map/projection.rs src/map/mod.rs osm_import/tests/projection.rs
git commit -m "feat(map): the projection from the layout to the earth, checked against geom

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The `Mapper` port and what the map says back

**Files:**
- Modify: `src/shared/ports.rs`
- Modify: `src/shared/platform.rs` (`browser_ports`)

**Interfaces:**
- Produces (all in `crate::shared::ports`):
  - `enum MapEvent { Ready { bounds: [f64; 4] }, Failed, Pick { hot: String }, Hover { hot: Option<String> }, Moved }` (`Deserialize`, internally tagged by `kind`, snake_case; `bounds` is `[west, south, east, north]`)
  - `trait Mapper { fn set_places(&self, layers: &str, data: &str); fn fit(&self, bounds: [f64; 4], insets: [f64; 4]); fn zoom_by(&self, factor: f64); fn pan_by(&self, dx: f64, dy: f64); fn highlight(&self, hot: Option<&str>); fn set_imperial(&self, imperial: bool); fn listen(&self, on: Box<dyn Fn(MapEvent)>); }` (`insets` is `[top, right, bottom, left]` in pixels)
  - `struct NoMapper` (every call does nothing)
  - `enum MapCall { Places { layers: String, data: String }, Fit { bounds: [f64; 4], insets: [f64; 4] }, Zoom(f64), Pan(f64, f64), Highlight(Option<String>), Imperial(bool) }`
  - `struct FakeMapper` with `take(&self) -> Vec<MapCall>` and `say(&self, event: MapEvent)` (delivers an event to the listener)
  - `Ports.mapper: Rc<dyn Mapper>`

- [ ] **Step 1: Write the failing tests**

Append to the end of `src/shared/ports.rs` (it has no test module today; add one):

```rust
#[cfg(test)]
mod map_tests {
    use super::*;

    #[test]
    fn the_adapters_events_are_read_from_json() {
        let read = |s: &str| serde_json::from_str::<MapEvent>(s).unwrap();
        assert_eq!(read(r#"{"kind":"ready","bounds":[1,2,3,4]}"#), MapEvent::Ready { bounds: [1.0, 2.0, 3.0, 4.0] });
        assert_eq!(read(r#"{"kind":"failed"}"#), MapEvent::Failed);
        assert_eq!(read(r#"{"kind":"pick","hot":"s-7"}"#), MapEvent::Pick { hot: "s-7".into() });
        assert_eq!(read(r#"{"kind":"hover","hot":"j-3"}"#), MapEvent::Hover { hot: Some("j-3".into()) });
        assert_eq!(read(r#"{"kind":"hover","hot":null}"#), MapEvent::Hover { hot: None });
        assert_eq!(read(r#"{"kind":"moved"}"#), MapEvent::Moved);
    }

    #[test]
    fn the_fake_records_what_it_is_asked_and_passes_on_what_it_is_told() {
        let fake = FakeMapper::default();
        fake.zoom_by(1.4);
        fake.pan_by(-80.0, 0.0);
        fake.highlight(Some("s-1"));
        fake.set_imperial(true);
        fake.fit([1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0]);
        fake.set_places("[]", "{}");
        assert_eq!(
            fake.take(),
            vec![
                MapCall::Zoom(1.4),
                MapCall::Pan(-80.0, 0.0),
                MapCall::Highlight(Some("s-1".into())),
                MapCall::Imperial(true),
                MapCall::Fit { bounds: [1.0, 2.0, 3.0, 4.0], insets: [5.0, 6.0, 7.0, 8.0] },
                MapCall::Places { layers: "[]".into(), data: "{}".into() },
            ]
        );
        assert!(fake.take().is_empty(), "taken once");

        let heard = Rc::new(RefCell::new(Vec::new()));
        let sink = heard.clone();
        fake.listen(Box::new(move |e| sink.borrow_mut().push(e)));
        fake.say(MapEvent::Moved);
        assert_eq!(*heard.borrow(), vec![MapEvent::Moved]);
    }
}
```

- [ ] **Step 2: Run to see them fail**

Run: `cargo test shared::ports 2>&1 | tail -10`
Expected: FAIL to compile — `MapEvent`, `FakeMapper` not found.

- [ ] **Step 3: Implement**

In `src/shared/ports.rs`, add `use serde::Deserialize;` to the imports, and after the `Navigator` trait:

```rust
/// What the map tells the page.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MapEvent {
    /// The tiles are there. `bounds` is the area they cover: west, south, east, north, in degrees.
    Ready { bounds: [f64; 4] },
    /// The tiles could not be had.
    Failed,
    /// A place on the map was pressed: `s-7` for a street, `j-3` for a junction.
    Pick { hot: String },
    /// The pointer is over a place, or is no longer.
    Hover { hot: Option<String> },
    /// The person moved the map themselves, with the pointer or the wheel.
    Moved,
}

/// The map the page shows the city on. It answers by telling `listen`'s function what happened.
pub trait Mapper {
    /// Shows the places: `layers` is a MapLibre layer list over a GeoJSON source named `places`, whose
    /// features `data` holds. Called again whenever the places change.
    fn set_places(&self, layers: &str, data: &str);
    /// Moves the view to hold `bounds` (west, south, east, north in degrees), clear of `insets`
    /// (top, right, bottom, left, in pixels).
    fn fit(&self, bounds: [f64; 4], insets: [f64; 4]);
    /// Zooms in by `factor` (more than 1), or out (less than 1).
    fn zoom_by(&self, factor: f64);
    /// Moves the view by pixels: a positive `dx` moves it east, a positive `dy` south.
    fn pan_by(&self, dx: f64, dy: f64);
    /// Marks the place `hot` (as in `s-7`) as the one the pointer or focus is on, and clears any other.
    fn highlight(&self, hot: Option<&str>);
    /// Whether the scale is shown in feet and miles.
    fn set_imperial(&self, imperial: bool);
    /// Has `on` called with what the map tells the page, from now on.
    fn listen(&self, on: Box<dyn Fn(MapEvent)>);
}

/// The map of a page that has none.
pub struct NoMapper;

impl Mapper for NoMapper {
    fn set_places(&self, _: &str, _: &str) {}
    fn fit(&self, _: [f64; 4], _: [f64; 4]) {}
    fn zoom_by(&self, _: f64) {}
    fn pan_by(&self, _: f64, _: f64) {}
    fn highlight(&self, _: Option<&str>) {}
    fn set_imperial(&self, _: bool) {}
    fn listen(&self, _: Box<dyn Fn(MapEvent)>) {}
}
```

Add the field to `Ports` (after `scheduler`): `pub mapper: Rc<dyn Mapper>,`.

After `RecordingNavigator`'s impl, add the fake:

```rust
/// What a `FakeMapper` was asked.
#[derive(Clone, Debug, PartialEq)]
pub enum MapCall {
    Places { layers: String, data: String },
    Fit { bounds: [f64; 4], insets: [f64; 4] },
    Zoom(f64),
    Pan(f64, f64),
    Highlight(Option<String>),
    Imperial(bool),
}

/// A map that keeps what it is asked, and passes on what a test has it say.
#[derive(Default)]
pub struct FakeMapper {
    calls: RefCell<Vec<MapCall>>,
    listener: RefCell<Option<Box<dyn Fn(MapEvent)>>>,
}

impl FakeMapper {
    /// What it was asked since this was last called.
    pub fn take(&self) -> Vec<MapCall> {
        std::mem::take(&mut *self.calls.borrow_mut())
    }

    /// The map tells the page something.
    pub fn say(&self, event: MapEvent) {
        if let Some(on) = &*self.listener.borrow() {
            on(event);
        }
    }
}

impl Mapper for FakeMapper {
    fn set_places(&self, layers: &str, data: &str) {
        self.calls.borrow_mut().push(MapCall::Places { layers: layers.into(), data: data.into() });
    }
    fn fit(&self, bounds: [f64; 4], insets: [f64; 4]) {
        self.calls.borrow_mut().push(MapCall::Fit { bounds, insets });
    }
    fn zoom_by(&self, factor: f64) {
        self.calls.borrow_mut().push(MapCall::Zoom(factor));
    }
    fn pan_by(&self, dx: f64, dy: f64) {
        self.calls.borrow_mut().push(MapCall::Pan(dx, dy));
    }
    fn highlight(&self, hot: Option<&str>) {
        self.calls.borrow_mut().push(MapCall::Highlight(hot.map(String::from)));
    }
    fn set_imperial(&self, imperial: bool) {
        self.calls.borrow_mut().push(MapCall::Imperial(imperial));
    }
    fn listen(&self, on: Box<dyn Fn(MapEvent)>) {
        *self.listener.borrow_mut() = Some(on);
    }
}
```

In `test_ports_with_time`, add `mapper: Rc::new(NoMapper),` to the `Ports { .. }` literal. In `src/shared/platform.rs` `browser_ports()` add `mapper: Rc::new(crate::shared::ports::NoMapper),`. Then run `grep -rn "Ports {" src` and add `mapper` to any other literal the compiler reports.

- [ ] **Step 4: Run the tests**

Run: `cargo test 2>&1 | tail -8 && cargo build --target wasm32-unknown-unknown --workspace 2>&1 | tail -3`
Expected: all tests PASS; the wasm build finishes with no warning.

- [ ] **Step 5: Commit**

```bash
git add src/shared/ports.rs src/shared/platform.rs
git commit -m "feat(shared): the Mapper port, with its fake and what the map says back

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The places as GeoJSON, and the layers that draw them

**Files:**
- Create: `src/map/overlay.rs`
- Create: `src/map/style.rs`
- Modify: `src/map/mod.rs` (`pub mod overlay;` and `pub mod style;`)

**Interfaces:**
- Consumes: `CityView`, `EdgeView.shape_mm`, `NodeView` (`x_mm`, `y_mm`, `radius_mm`, `junction`, `uid`, `ok`, `edited`), `Projection` (Task 2).
- Produces:
  - `overlay::places(v: &CityView, p: &Projection) -> String` — a GeoJSON `FeatureCollection`: a `LineString` per street with properties `hot` (`"s-<uid>"`), `status` (`"ok" | "changed" | "bad"`), `width_m`; a `Point` per junction (nodes with `junction == true`) with `hot` (`"j-<uid>"`), `status`, `radius_m`. Coordinates are `[lon, lat]` rounded to 6 decimals.
  - `overlay::bounds(v: &CityView, p: &Projection) -> Option<[f64; 4]>` — `[west, south, east, north]` over every street point and junction; `None` for a city with neither.
  - `style::SOURCE: &str` (`"places"`), `style::layers(lat: f64) -> String` — a JSON array of MapLibre layers with ids `places-casing`, `places-street`, `places-junction`, in that order, all on source `places`.
  - `style::OK`, `style::CHANGED`, `style::BAD`: the status colours (CSS colour strings), reused by the page's key.

- [ ] **Step 1: Write the failing tests for the overlay**

Create `src/map/overlay.rs` with only this test module for now (add `use` lines as you implement):

```rust
//! The city's places as GeoJSON for the map: each street along its real centreline, each junction a point.

#[cfg(test)]
mod tests {
    use osm_network::{Control, Lane, LaneKind, Network, Node, Road, Way};
    use serde_json::Value;

    use super::*;
    use crate::city::model::City;
    use crate::map::projection::Projection;
    use crate::place::area::Area;

    /// One street that bends: east for 80 m, then north-east to a point 100 m east and 50 m north.
    fn bent_road() -> Network {
        let node = |id, x_m, y_m| Node { id, osm_nodes: vec![id as i64], x_m, y_m, junction: false, control: Control::None };
        let lane = |way| Lane { kind: LaneKind::Driving, way, width_m: 3.0 };
        Network {
            left_hand: false,
            nodes: vec![node(1, 0.0, 0.0), node(2, 100.0, 50.0)],
            roads: vec![Road {
                id: 1,
                osm_ways: vec![7],
                name: Some("Bend Street".into()),
                highway: "residential".into(),
                from: 1,
                to: 2,
                lanes: vec![lane(Way::Forward), lane(Way::Backward)],
                points: vec![(0.0, 0.0), (80.0, 0.0), (100.0, 50.0)],
            }],
        }
    }

    fn projection(v: &crate::city::model::CityView) -> Projection {
        Projection::new(Area::new("Bendville", 45.5, -73.6).bounds(), v.origin_m.unwrap())
    }

    fn parse(json: &str) -> Value {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_street_is_a_line_along_its_centreline_with_the_place_it_opens_and_how_it_stands() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let fc = parse(&places(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 1, "one street and no junction");
        let f = &features[0];
        assert_eq!(f["geometry"]["type"], "LineString");
        assert_eq!(f["properties"]["hot"], "s-1");
        assert_eq!(f["properties"]["status"], "ok");
        assert_eq!(f["properties"]["width_m"].as_f64().unwrap(), v.edges[0].row_mm as f64 / 1000.0);
        let line = f["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(line.len(), 3, "the bend is kept");
        let round = |x: f64| (x * 1e6).round() / 1e6;
        let first = p.lon_lat(v.edges[0].shape_mm[0][0], v.edges[0].shape_mm[0][1]);
        assert_eq!(line[0], serde_json::json!([round(first[0]), round(first[1])]));
        // the middle point is off the chord between the ends: the street is not drawn straight
        let (a, m, b) = (&line[0], &line[1], &line[2]);
        let lerp = a[1].as_f64().unwrap() + (b[1].as_f64().unwrap() - a[1].as_f64().unwrap()) * (m[0].as_f64().unwrap() - a[0].as_f64().unwrap()) / (b[0].as_f64().unwrap() - a[0].as_f64().unwrap());
        assert!((m[1].as_f64().unwrap() - lerp).abs() > 1e-5, "{m} on the chord from {a} to {b}");
    }

    #[test]
    fn a_street_that_fails_or_has_been_changed_says_so() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let status = |v: &crate::city::model::CityView| parse(&places(v, &p))["features"][0]["properties"]["status"].as_str().unwrap().to_string();
        assert_eq!(status(&v), "ok");
        v.edges[0].edited = true;
        assert_eq!(status(&v), "changed");
        v.edges[0].ok = false;
        assert_eq!(status(&v), "bad", "failing wins over changed");
    }

    #[test]
    fn a_junction_is_a_point_with_its_radius_and_a_street_end_is_not_drawn() {
        let v = City::new().view(0); // the sample city has junctions and gates
        let p = Projection::new(Area::new("Sample", 45.5, -73.6).bounds(), [0.0, 0.0]);
        let fc = parse(&places(&v, &p));
        let points: Vec<&Value> = fc["features"].as_array().unwrap().iter().filter(|f| f["geometry"]["type"] == "Point").collect();
        assert_eq!(points.len(), v.nodes.iter().filter(|n| n.junction).count());
        assert!(points.iter().all(|f| f["properties"]["hot"].as_str().unwrap().starts_with("j-") && f["properties"]["radius_m"].as_f64().unwrap() > 0.0));
        let streets = fc["features"].as_array().unwrap().iter().filter(|f| f["geometry"]["type"] == "LineString").count();
        assert_eq!(streets, v.edges.len());
    }

    #[test]
    fn the_bounds_hold_every_point_of_every_street() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let [w, s, e, n] = bounds(&v, &p).unwrap();
        for c in parse(&places(&v, &p))["features"][0]["geometry"]["coordinates"].as_array().unwrap() {
            let (lon, lat) = (c[0].as_f64().unwrap(), c[1].as_f64().unwrap());
            assert!(w - 1e-6 <= lon && lon <= e + 1e-6 && s - 1e-6 <= lat && lat <= n + 1e-6, "{c} outside {w} {s} {e} {n}");
        }
        assert!(w < e && s < n);
    }
}
```

- [ ] **Step 2: Write the failing tests for the layers**

Create `src/map/style.rs` with only this test module for now:

```rust
//! The layers the places are drawn with on the map. Widths are in metres on the ground, so the layers
//! depend on the latitude of the place; the colours are the page's own.

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn layers(lat: f64) -> Vec<Value> {
        serde_json::from_str(&super::layers(lat)).unwrap()
    }

    #[test]
    fn the_places_are_drawn_in_three_layers_on_one_source() {
        let l = layers(45.5);
        let ids: Vec<&str> = l.iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["places-casing", "places-street", "places-junction"]);
        assert!(l.iter().all(|l| l["source"] == SOURCE));
        assert_eq!(SOURCE, "places");
    }

    #[test]
    fn the_colour_follows_the_status_and_the_highlight() {
        let street = &layers(45.5)[1];
        let colour = street["paint"]["line-color"].to_string();
        assert!(colour.contains("feature-state") && colour.contains("\"status\""));
        for c in [OK, CHANGED, BAD] {
            assert!(colour.contains(c), "{c} in {colour}");
        }
    }

    #[test]
    fn a_width_in_metres_doubles_with_each_zoom_and_is_a_pixel_scale_at_the_latitude() {
        let street = &layers(0.0)[1];
        let width = &street["paint"]["line-width"];
        let text = width.to_string();
        assert!(text.contains("\"width_m\"") && text.contains("\"exponential\""), "{text}");
        // pull the two stops of the zoom interpolation out of the expression
        fn stops(v: &Value, out: &mut Vec<(f64, f64)>) {
            if let Some(a) = v.as_array() {
                if a.first().is_some_and(|f| f == "interpolate") {
                    let rest = &a[3..];
                    for pair in rest.chunks(2) {
                        out.push((pair[0].as_f64().unwrap(), pair[1].as_f64().unwrap()));
                    }
                }
                a.iter().for_each(|x| stops(x, out));
            }
        }
        let mut s = Vec::new();
        stops(width, &mut s);
        assert_eq!(s.len(), 2, "{text}");
        assert_eq!(s[0].0, 0.0);
        assert_eq!(s[1].0, 24.0);
        assert_eq!(s[1].1 / s[0].1, 2f64.powi(24), "doubling with each zoom");
        // at the equator, at zoom 17, a metre is about 1.675 pixels on 512-pixel tiles
        let at_17 = s[0].1 * 2f64.powi(17);
        assert!((at_17 - 1.675).abs() < 0.01, "{at_17}");
        // and fewer pixels to the metre away from the equator... no: more, because a degree is shorter
        assert!(layers(60.0)[1]["paint"]["line-width"].to_string() != text);
    }

    #[test]
    fn a_junction_is_a_circle_with_a_radius_in_metres_and_a_minimum() {
        let junction = &layers(45.5)[2];
        assert_eq!(junction["type"], "circle");
        let r = junction["paint"]["circle-radius"].to_string();
        assert!(r.contains("\"radius_m\"") && r.contains("\"max\""), "{r}");
    }
}
```

- [ ] **Step 3: Run to see them fail**

Add `pub mod overlay;` and `pub mod style;` to `src/map/mod.rs`, then run: `cargo test map::overlay map::style 2>&1 | tail -10`
Expected: FAIL to compile — `places`, `bounds`, `layers`, `SOURCE` not found.

- [ ] **Step 4: Implement `overlay.rs`**

Insert above `#[cfg(test)]` in `src/map/overlay.rs`:

```rust
use serde_json::{Value, json};

use crate::city::model::CityView;
use crate::map::projection::Projection;

/// How a place stands: needing attention beats having been changed.
fn status(ok: bool, edited: bool) -> &'static str {
    if !ok {
        "bad"
    } else if edited {
        "changed"
    } else {
        "ok"
    }
}

/// A coordinate to six decimals of a degree: about a tenth of a metre.
fn r6(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

fn position(p: &Projection, x_mm: i32, y_mm: i32) -> [f64; 2] {
    let [lon, lat] = p.lon_lat(x_mm, y_mm);
    [r6(lon), r6(lat)]
}

/// The streets and junctions of the city as a GeoJSON feature collection.
pub fn places(v: &CityView, p: &Projection) -> String {
    let mut features: Vec<Value> = Vec::with_capacity(v.edges.len() + v.nodes.len());
    for e in &v.edges {
        features.push(json!({
            "type": "Feature",
            "properties": { "hot": format!("s-{}", e.uid), "status": status(e.ok, e.edited), "width_m": e.row_mm as f64 / 1000.0 },
            "geometry": { "type": "LineString", "coordinates": e.shape_mm.iter().map(|q| position(p, q[0], q[1])).collect::<Vec<_>>() },
        }));
    }
    for n in v.nodes.iter().filter(|n| n.junction) {
        features.push(json!({
            "type": "Feature",
            "properties": { "hot": format!("j-{}", n.uid), "status": status(n.ok, n.edited), "radius_m": n.radius_mm as f64 / 1000.0 },
            "geometry": { "type": "Point", "coordinates": position(p, n.x_mm, n.y_mm) },
        }));
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// West, south, east and north of everything the places draw, in degrees.
pub fn bounds(v: &CityView, p: &Projection) -> Option<[f64; 4]> {
    let streets = v.edges.iter().flat_map(|e| e.shape_mm.iter().map(|q| (q[0], q[1])));
    let junctions = v.nodes.iter().filter(|n| n.junction).map(|n| (n.x_mm, n.y_mm));
    streets.chain(junctions).map(|(x, y)| p.lon_lat(x, y)).fold(None, |b, [lon, lat]| match b {
        None => Some([lon, lat, lon, lat]),
        Some([w, s, e, n]) => Some([w.min(lon), s.min(lat), e.max(lon), n.max(lat)]),
    })
}
```

- [ ] **Step 5: Implement `style.rs`**

Insert above `#[cfg(test)]` in `src/map/style.rs`:

```rust
use serde_json::{Value, json};

/// The name of the GeoJSON source the layers draw.
pub const SOURCE: &str = "places";

/// What the colours say: a place that works, one that has been changed, one that needs attention.
pub const OK: &str = "#4b6b8a";
pub const CHANGED: &str = "#2563eb";
pub const BAD: &str = "#dc2626";
/// The place the pointer or the focus is on.
const HOT: &str = "#f59e0b";

/// MapLibre's zoom 0 shows the earth on one 512-pixel tile: this many pixels to a metre there, at this
/// latitude. A width in metres is that times two to the zoom.
fn pixels_per_metre_at_zoom_0(lat: f64) -> f64 {
    512.0 / (40_075_016.686 * lat.to_radians().cos())
}

/// A property in metres, drawn at least `min_px` wide, growing with the zoom as the ground does.
fn metres(property: &str, lat: f64, min_px: f64) -> Value {
    let k = pixels_per_metre_at_zoom_0(lat);
    json!(["max", min_px, ["*", ["get", property], ["interpolate", ["exponential", 2], ["zoom"], 0, k, 24, k * 2f64.powi(24)]]])
}

/// The colour of a place: the highlight where the pointer is, else the one its status says.
fn colour() -> Value {
    json!(["case", ["boolean", ["feature-state", "hot"], false], HOT, ["match", ["get", "status"], "bad", BAD, "changed", CHANGED, OK]])
}

/// The layers, as JSON, for a place at this latitude.
pub fn layers(lat: f64) -> String {
    json!([
        {
            "id": "places-casing", "type": "line", "source": SOURCE,
            "filter": ["==", ["geometry-type"], "LineString"],
            "layout": { "line-cap": "butt", "line-join": "round" },
            "paint": { "line-color": "#ffffff", "line-opacity": 0.9, "line-width": ["+", metres("width_m", lat, 3.0), 3] }
        },
        {
            "id": "places-street", "type": "line", "source": SOURCE,
            "filter": ["==", ["geometry-type"], "LineString"],
            "layout": { "line-cap": "butt", "line-join": "round" },
            "paint": { "line-color": colour(), "line-width": metres("width_m", lat, 2.5) }
        },
        {
            "id": "places-junction", "type": "circle", "source": SOURCE,
            "filter": ["==", ["geometry-type"], "Point"],
            "paint": { "circle-color": colour(), "circle-radius": metres("radius_m", lat, 5.0), "circle-stroke-color": "#ffffff", "circle-stroke-width": 2 }
        }
    ])
    .to_string()
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test map::overlay map::style 2>&1 | tail -15`
Expected: PASS (8 tests). If `the_bounds_hold…` fails on `w < e`, the bent road's projected points are degenerate; check `Projection::new` got `v.origin_m`.

- [ ] **Step 7: Commit**

```bash
git add src/map/overlay.rs src/map/style.rs src/map/mod.rs
git commit -m "feat(map): the places as GeoJSON along their real centrelines, and the layers that draw them

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The map view-model works the map through the port

**Files:**
- Modify: `src/map/vm.rs`
- Delete: `src/map/gestures.rs`; remove `pub mod gestures;` from `src/map/mod.rs`
- Modify: `src/map/svg.rs`, `src/map/camera.rs` (remove what only the map page used, see Step 6)

**Interfaces:**
- Consumes: `Mapper`, `MapEvent`, `MapCall`, `FakeMapper`, `Ports.mapper` (Task 3); `Projection` (Task 2); `overlay::{places, bounds}`, `style::layers` (Task 4); `CityStore::area()`, `CityView.origin_m`.
- Produces on `MapVm`:
  - `pub enum BasemapState { Waiting, Ready, Unavailable(&'static str) }` and consts `MISSING`, `OUTSIDE`, `NO_ROADS` (the words said)
  - `attach(&self)` — subscribes to the port's events and sets the scale's units; call once on the map page
  - `basemap_state(&self) -> BasemapState` (tracked)
  - `projection(&self) -> Option<Projection>`
  - `sync_places(&self)` (untracked command: sends the places to the map when it is `Ready`)
  - `sync_highlight(&self)` (untracked command: sends `hot` to the map)
  - `fit(&self)` (untracked command: fits the places in what the panels leave)
  - `status_key(&self) -> Vec<(&'static str, &'static str)>` — `[("ok", "Works"), ("changed", "Changed"), ("bad", "Needs attention")]`
  - Unchanged names kept: `zoom_in`, `zoom_out`, `fit_camera`, `press_key`, `set_insets`, `resize`, `camera`, `view_box`, `set_units`, `set_hot`, `hot`.
  - Removed: `wheel`, `pointer_down`, `pointer_move`, `pointer_up`, `swallow_click`, `panning`, `scale_bar`, `legend`.

- [ ] **Step 1: Write the failing tests**

In `src/map/vm.rs`'s `mod tests`, add imports `use crate::map::camera::Insets;`, `use crate::place::area::Area;`, `use crate::shared::ports::{FakeMapper, MapCall, MapEvent, Ports, RecordingNavigator};`, `use osm_network::{Control, Lane, LaneKind, Network, Node, Road, Way};`, and add these helpers and tests:

```rust
    /// A city of one street that bends, kept for an area that the page opens.
    fn area_vm() -> (Rc<MapVm>, Rc<FakeMapper>, Rc<RecordingNavigator>, Rc<MemoryStorage>) {
        let (ports, _, storage) = test_ports();
        let area = Area::new("Bendville", 45.5, -73.6);
        let node = |id, x_m, y_m| Node { id, osm_nodes: vec![id as i64], x_m, y_m, junction: false, control: Control::None };
        let lane = |way| Lane { kind: LaneKind::Driving, way, width_m: 3.0 };
        let network = Network {
            left_hand: false,
            nodes: vec![node(1, 0.0, 0.0), node(2, 100.0, 50.0)],
            roads: vec![Road {
                id: 1,
                osm_ways: vec![7],
                name: Some("Bend Street".into()),
                highway: "residential".into(),
                from: 1,
                to: 2,
                lanes: vec![lane(Way::Forward), lane(Way::Backward)],
                points: vec![(0.0, 0.0), (80.0, 0.0), (100.0, 50.0)],
            }],
        };
        assert!(CityStore::for_area(storage.clone(), area.clone()).keep_network(&network));
        assert!(CityStore::choose(&*storage, &area));
        let (mapper, navigator) = (Rc::new(FakeMapper::default()), Rc::new(RecordingNavigator::default()));
        let ports = Ports { mapper: mapper.clone(), navigator: navigator.clone(), ..ports };
        let vm = MapVm::new(ports);
        vm.attach();
        mapper.take(); // the scale's units, said when it attached
        (vm, mapper, navigator, storage)
    }

    /// Tiles that cover the area `area_vm` opens.
    const COVERING: MapEvent = MapEvent::Ready { bounds: [-74.0, 45.0, -73.0, 46.0] };

    #[test]
    fn a_page_on_the_sample_city_has_no_map_and_says_why() {
        let (vm, ..) = vm();
        assert_eq!(vm.basemap_state(), BasemapState::Unavailable(NO_ROADS));
        assert!(vm.projection().is_none());
    }

    #[test]
    fn the_map_waits_for_the_tiles_and_then_shows_the_places_and_fits_them() {
        let (vm, mapper, ..) = area_vm();
        assert_eq!(vm.basemap_state(), BasemapState::Waiting);
        assert!(mapper.take().is_empty(), "nothing is shown before the tiles are there");
        mapper.say(COVERING);
        assert_eq!(vm.basemap_state(), BasemapState::Ready);
        let calls = mapper.take();
        assert!(matches!(&calls[0], MapCall::Places { layers, data } if layers.contains("places-street") && data.contains("\"s-1\"")), "{calls:?}");
        assert!(matches!(&calls[1], MapCall::Fit { bounds, insets } if bounds[0] < bounds[2] && bounds[1] < bounds[3] && *insets == [0.0; 4]), "{calls:?}");
    }

    #[test]
    fn tiles_that_do_not_cover_the_area_say_so_and_show_nothing() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(MapEvent::Ready { bounds: [10.0, 50.0, 11.0, 51.0] });
        assert_eq!(vm.basemap_state(), BasemapState::Unavailable(OUTSIDE));
        assert!(mapper.take().is_empty());
        vm.sync_places();
        assert!(mapper.take().is_empty(), "still nothing, though asked");
    }

    #[test]
    fn tiles_that_could_not_be_had_say_so() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(MapEvent::Failed);
        assert_eq!(vm.basemap_state(), BasemapState::Unavailable(MISSING));
    }

    #[test]
    fn pressing_a_place_opens_it_and_the_pointer_over_one_makes_it_hot() {
        let (vm, mapper, navigator, _) = area_vm();
        mapper.say(MapEvent::Pick { hot: "s-1".into() });
        mapper.say(MapEvent::Pick { hot: "j-3".into() });
        mapper.say(MapEvent::Pick { hot: "nonsense".into() });
        assert_eq!(navigator.take(), vec!["street.html?street=1".to_string(), "intersection.html?junction=3".to_string()]);
        mapper.say(MapEvent::Hover { hot: Some("s-1".into()) });
        assert_eq!(vm.hot().as_deref(), Some("s-1"));
        mapper.say(MapEvent::Hover { hot: None });
        assert_eq!(vm.hot(), None);
    }

    #[test]
    fn the_hot_place_is_sent_to_the_map() {
        let (vm, mapper, ..) = area_vm();
        vm.set_hot(Some("s-1".into()));
        vm.sync_highlight();
        vm.set_hot(None);
        vm.sync_highlight();
        assert_eq!(mapper.take(), vec![MapCall::Highlight(Some("s-1".into())), MapCall::Highlight(None)]);
    }

    #[test]
    fn the_buttons_and_keys_move_the_map_through_the_port() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        mapper.take();
        vm.zoom_in();
        vm.zoom_out();
        assert!(vm.press_key("+") && vm.press_key("=") && vm.press_key("-") && vm.press_key("_"));
        assert!(vm.press_key("ArrowLeft") && vm.press_key("ArrowRight") && vm.press_key("ArrowUp") && vm.press_key("ArrowDown"));
        assert!(!vm.press_key("a"));
        assert!(vm.press_key("0"));
        let calls = mapper.take();
        let (zin, zout) = (1.4, 1.0 / 1.4);
        assert_eq!(
            calls[..10],
            [
                MapCall::Zoom(zin),
                MapCall::Zoom(zout),
                MapCall::Zoom(zin),
                MapCall::Zoom(zin),
                MapCall::Zoom(zout),
                MapCall::Zoom(zout),
                MapCall::Pan(-80.0, 0.0),
                MapCall::Pan(80.0, 0.0),
                MapCall::Pan(0.0, -80.0),
                MapCall::Pan(0.0, 80.0),
            ]
        );
        assert!(matches!(calls[10], MapCall::Fit { .. }), "{calls:?}");
    }

    #[test]
    fn the_places_are_fitted_in_what_floats_over_the_map_until_the_person_moves_it() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        mapper.take();
        vm.set_insets(Insets { left: 100.0, top: 50.0, right: 20.0, bottom: 10.0 });
        assert!(matches!(mapper.take()[..], [MapCall::Fit { insets, .. }] if insets == [50.0, 20.0, 10.0, 100.0]), "top, right, bottom, left");
        mapper.say(MapEvent::Moved);
        vm.set_insets(Insets { left: 200.0, ..Insets::default() });
        assert!(mapper.take().is_empty(), "the person's view is kept");
        vm.fit_camera();
        assert!(matches!(mapper.take()[..], [MapCall::Fit { insets, .. }] if insets == [0.0, 0.0, 0.0, 200.0]));
        vm.set_insets(Insets::default());
        assert_eq!(mapper.take().len(), 1, "fitting gave the view back to the page");
    }

    #[test]
    fn the_units_are_told_to_the_scale() {
        let (vm, mapper, ..) = area_vm();
        vm.set_units(Units::Feet);
        assert_eq!(mapper.take(), vec![MapCall::Imperial(true)]);
        vm.set_units(Units::Meters);
        assert_eq!(mapper.take(), vec![MapCall::Imperial(false)]);
    }

    #[test]
    fn the_key_says_what_the_colours_of_the_places_mean() {
        let (vm, ..) = vm();
        assert_eq!(vm.status_key(), vec![("ok", "Works"), ("changed", "Changed"), ("bad", "Needs attention")]);
    }
```

(`Units::Meters`: use whatever the metric variant is called in `shared::units`; the file's existing tests use `Units::Feet`.)

Also **delete** these tests from `vm.rs` (their behaviour moves to the port or is gone): `the_camera_follows_the_window_and_the_buttons_and_keys`, `the_wheel_zooms_about_the_pointer_and_finer_with_control`, `the_view_box_and_scale_bar_follow_the_camera_and_the_units`, `a_drag_pans_the_map_and_the_click_that_ends_it_is_swallowed`, `the_legend_lists_the_kinds_the_streets_are_made_of_in_catalogue_order`; and add this replacement for the hero's camera, which stays:

```rust
    #[test]
    fn the_hero_camera_follows_the_window_and_the_view_box_follows_it() {
        let (vm, ..) = vm();
        let o = Owner::new();
        o.set();
        vm.resize(1000.0, 600.0);
        let fit = vm.camera();
        assert_eq!((fit.width, fit.height), (1000.0, 600.0));
        let before = vm.view_box();
        vm.resize(500.0, 600.0);
        assert_ne!(vm.view_box(), before);
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `cargo test map::vm 2>&1 | tail -15`
Expected: FAIL to compile — `BasemapState`, `attach`, `projection` etc. not found.

- [ ] **Step 3: Implement in `src/map/vm.rs`**

Imports: replace `use crate::map::camera::{Camera, Insets, ScaleBar, World};` with `use crate::map::camera::{Camera, Insets, World};`; delete `use crate::map::gestures::{MapGestures, Moved};`; add `use crate::map::overlay;`, `use crate::map::projection::Projection;`, `use crate::map::style;`, `use crate::shared::ports::{MapEvent, Ports};` (replacing the `Ports`-only import), `use std::cell::Cell;`, `use std::rc::{Rc, Weak};`. Remove `RefCell` if the compiler reports it unused.

Add under the existing consts:

```rust
/// What the map page says where it has no map to show.
pub const MISSING: &str = "The basemap could not be loaded. Build it with `just basemap-tiles`, then reload the page.";
pub const OUTSIDE: &str = "There is no basemap for this place. The map covers the Montréal area.";
pub const NO_ROADS: &str = "The roads of this place could not be loaded, so there is no map to show.";

/// Whether the basemap is there to draw the places on.
#[derive(Clone, Debug, PartialEq)]
pub enum BasemapState {
    /// Asked for; the tiles have not answered.
    Waiting,
    Ready,
    /// There will be no map, for the reason in these words.
    Unavailable(&'static str),
}

/// The place a `hot` name (`s-7`, `j-3`) opens.
fn href_of(hot: &str) -> Option<String> {
    let (kind, uid) = hot.split_once('-')?;
    let uid: u32 = uid.parse().ok()?;
    match kind {
        "s" => Some(street_href(uid)),
        "j" => Some(junction_href(uid)),
        _ => None,
    }
}
```

In `pub struct MapVm`: remove `gestures` and `panning`; add `me: Weak<MapVm>,`, `basemap: ArcRwSignal<BasemapState>,`, `moved: Cell<bool>,`.

Replace `MapVm::new`:

```rust
    pub fn new(ports: Ports) -> Rc<MapVm> {
        let store = CityStore::current(ports.storage.clone());
        let model = CityModel { city: store.open(), region: store.region() };
        let core = Core::new(model);
        let world = World::round(core.view_now().bounds_mm);
        let basemap = if store.area().is_some() { BasemapState::Waiting } else { BasemapState::Unavailable(NO_ROADS) };
        Rc::new_cyclic(|me| MapVm {
            me: me.clone(),
            core,
            store,
            ports,
            camera: ArcRwSignal::new(Camera::new(world, 800.0, 520.0)),
            basemap: ArcRwSignal::new(basemap),
            moved: Cell::new(false),
            hot: ArcRwSignal::new(None),
            armed: ArcRwSignal::new(false),
            search: ArcRwSignal::new(String::new()),
            active: ArcRwSignal::new(None),
        })
    }
```

Delete `legend`, `scale_bar`, `panning`, `wheel`, `pointer_down`, `pointer_move`, `pointer_up`, `swallow_click`. Replace `zoom_in`, `zoom_out`, `fit_camera`, `press_key`, `set_insets`, `set_units` and add the new members:

```rust
    // ---- the map ----

    pub fn basemap_state(&self) -> BasemapState {
        self.basemap.get()
    }

    /// Where the city lies on the earth: known once its area's roads are, never for the sample city.
    pub fn projection(&self) -> Option<Projection> {
        let area = self.store.area()?;
        let origin = self.core.view_now().origin_m?;
        Some(Projection::new(area.bounds(), origin))
    }

    /// Listens to what the map says, and tells it the units. Called once, by the page that has a map.
    pub fn attach(&self) {
        let me = self.me.clone();
        self.ports.mapper.listen(Box::new(move |event| {
            if let Some(vm) = me.upgrade() {
                vm.on_map_event(event);
            }
        }));
        self.ports.mapper.set_imperial(matches!(self.core.units(), Units::Feet));
    }

    fn on_map_event(&self, event: MapEvent) {
        match event {
            MapEvent::Ready { bounds } => self.tiles_ready(bounds),
            MapEvent::Failed => self.give_up(MISSING),
            MapEvent::Pick { hot } => {
                if let Some(href) = href_of(&hot) {
                    self.ports.navigator.go(&href);
                }
            }
            MapEvent::Hover { hot } => self.set_hot(hot),
            MapEvent::Moved => self.moved.set(true),
        }
    }

    /// There will be no map, unless the page never had one to wait for (which has said why already).
    fn give_up(&self, words: &'static str) {
        if self.store.area().is_some() {
            self.basemap.set(BasemapState::Unavailable(words));
        }
    }

    fn tiles_ready(&self, [west, south, east, north]: [f64; 4]) {
        let Some(area) = self.store.area() else { return };
        if !(west..=east).contains(&area.lon) || !(south..=north).contains(&area.lat) {
            return self.give_up(OUTSIDE);
        }
        self.basemap.set(BasemapState::Ready);
        self.sync_places();
        self.fit();
    }

    /// Sends the places to the map, once there is a map to show them on.
    pub fn sync_places(&self) {
        let (Some(projection), Some(area)) = (self.projection(), self.store.area()) else { return };
        if self.basemap.get_untracked() != BasemapState::Ready {
            return;
        }
        self.ports.mapper.set_places(&style::layers(area.lat), &overlay::places(&self.core.view_now(), &projection));
    }

    /// Sends the place the pointer or the focus is on to the map.
    pub fn sync_highlight(&self) {
        self.ports.mapper.highlight(self.hot.get_untracked().as_deref());
    }

    /// Fits the places in what the panels leave of the map, and gives the view back to the page.
    pub fn fit(&self) {
        self.moved.set(false);
        let Some(projection) = self.projection() else { return };
        if self.basemap.get_untracked() != BasemapState::Ready {
            return;
        }
        let Some(bounds) = overlay::bounds(&self.core.view_now(), &projection) else { return };
        let i = self.camera.get_untracked().insets;
        self.ports.mapper.fit(bounds, [i.top, i.right, i.bottom, i.left]);
    }

    /// The colours the places are drawn in, and what each says.
    pub fn status_key(&self) -> Vec<(&'static str, &'static str)> {
        vec![("ok", "Works"), ("changed", "Changed"), ("bad", "Needs attention")]
    }

    pub fn zoom_in(&self) {
        self.moved.set(true);
        self.ports.mapper.zoom_by(ZOOM_STEP);
    }

    pub fn zoom_out(&self) {
        self.moved.set(true);
        self.ports.mapper.zoom_by(1.0 / ZOOM_STEP);
    }

    fn pan(&self, dx: f64, dy: f64) {
        self.moved.set(true);
        self.ports.mapper.pan_by(dx, dy);
    }

    /// The whole city fits the window again: the hero's camera, and the map.
    pub fn fit_camera(&self) {
        self.update_camera(|c| c.fit());
        self.fit();
    }

    /// What a key does to the map; says whether the key was one of its own.
    pub fn press_key(&self, key: &str) -> bool {
        match key {
            "ArrowLeft" => self.pan(-PAN_PX, 0.0),
            "ArrowRight" => self.pan(PAN_PX, 0.0),
            "ArrowUp" => self.pan(0.0, -PAN_PX),
            "ArrowDown" => self.pan(0.0, PAN_PX),
            "+" | "=" => self.zoom_in(),
            "-" | "_" => self.zoom_out(),
            "0" => self.fit_camera(),
            _ => return false,
        }
        true
    }
```

Replace `set_insets` and `set_units`:

```rust
    /// What floats over the map, in pixels from each edge: the whole city is fitted in what is left, until
    /// the person moves the map themselves.
    pub fn set_insets(&self, insets: Insets) {
        self.update_camera(|c| c.set_insets(insets));
        if !self.moved.get() {
            self.fit();
        }
    }

    pub fn set_units(&self, units: Units) {
        self.core.set_units(units);
        self.ports.mapper.set_imperial(matches!(units, Units::Feet));
    }
```

(Delete the previous `set_insets` and `set_units`. Keep `resize`, `camera`, `view_box`, `update_camera`, `set_hot`, `reload`.)

- [ ] **Step 4: Run the view-model tests**

Run: `cargo test map::vm 2>&1 | tail -20`
Expected: PASS. Fix anything the compiler reports (unused imports, `Units` variant name). The test `the_places_are_fitted_in_what_floats…` pins a subtle rule: `fit_camera` resets `moved`, so the later `set_insets` fits again.

- [ ] **Step 5: Delete the gestures and what only they used**

```bash
git rm src/map/gestures.rs
```

Remove `pub mod gestures;` from `src/map/mod.rs`. Then remove what nothing but the map page's SVG view used, as the compiler and `grep` show: in `src/map/svg.rs` delete `hot_layer` and `overlay_svg` and their tests (the hero keeps `map_svg` and `map_hatch`); in `src/map/camera.rs` delete `ScaleBar` and `Camera::scale_bar` and their tests. Keep every other `Camera` method: the hero and `Camera`'s own tests use them.

`src/map/view.rs` still calls the removed `pointer_down`, `wheel`, `legend` and the rest, so the crate does not compile until its `MapView` and `Legend` are rewritten. Do that now, in this task: carry out Task 8 Step 3 (the `MapView` and `Legend` code there is complete) and delete the four obsolete component tests and add the two new ones from Task 8 Step 1. Task 8 then has only the adapter, the page and the browser's `Mapper` left to do, and its Step 1 to 4 are already done.

Run: `cargo test 2>&1 | tail -15 && cargo build --target wasm32-unknown-unknown --workspace 2>&1 | tail -5`
Expected: all tests PASS (including `map::vm`, `map::svg`, `map::camera`, `map::overlay`, `map::style`, `map::projection`, `map::tests`) and the wasm build has no warning. Remove the imports and helpers (`window_of`, `hot_of`, `PointerEvent`, `WheelEvent`, `RefCell`) the compiler reports unused.

- [ ] **Step 6: Commit**

```bash
git add -A src/map
git commit -m "feat(map): the map view-model works the map through the Mapper port

The pointer, wheel and drag go: MapLibre handles them and tells the page when
the person moves the map. The hero keeps its camera. The map view binds to the
port instead of drawing SVG.

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 6: A server that answers range requests, and the libraries and glyphs the map needs

**Files:**
- Modify: `e2e/serve.mjs`, `justfile`, `scripts/build.sh`, `.gitignore`, `.github/workflows/ci.yml`, `.github/dependabot.yml`
- Create: `package.json`, `package-lock.json` (repository root), `e2e/tests/serve.spec.ts`
- Create: `web/glyphs/Noto Sans Regular/{0-255,256-511,8192-8447}.pbf`, the same for `Noto Sans Bold`, `web/glyphs/LICENSE-noto-sans.txt`

**Interfaces:**
- Produces: `web/vendor/maplibre-gl.mjs`, `maplibre-gl-shared.mjs`, `maplibre-gl-worker.mjs`, `maplibre-gl.css`, `pmtiles.js` (built, gitignored); glyph PBFs at `web/glyphs/<font stack>/<range>.pbf` for font stacks `Noto Sans Regular` and `Noto Sans Bold`; a static server that answers `Range: bytes=…` with `206`.

- [ ] **Step 1: Write the failing test for range requests**

Create `e2e/tests/serve.spec.ts`:

```ts
import { expect, test } from "./fixtures";

// PMTiles is read with range requests, so the server the pages are tested on has to answer them.
test.describe("the static server", () => {
  test("answers a range with just those bytes and says where they lie", async ({ request }) => {
    const whole = await (await request.get("/map.html")).body();
    const part = await request.get("/map.html", { headers: { range: "bytes=10-29" } });
    expect(part.status()).toBe(206);
    expect(part.headers()["content-range"]).toBe(`bytes 10-29/${whole.length}`);
    expect(part.headers()["accept-ranges"]).toBe("bytes");
    expect(await part.body()).toEqual(whole.subarray(10, 30));
  });

  test("answers an open-ended range and a range of the last bytes", async ({ request }) => {
    const whole = await (await request.get("/map.html")).body();
    const from = await request.get("/map.html", { headers: { range: "bytes=100-" } });
    expect(from.status()).toBe(206);
    expect(await from.body()).toEqual(whole.subarray(100));
    const last = await request.get("/map.html", { headers: { range: "bytes=-16" } });
    expect(last.status()).toBe(206);
    expect(await last.body()).toEqual(whole.subarray(whole.length - 16));
  });

  test("refuses a range past the end, and serves the whole file when there is no range", async ({ request }) => {
    const whole = await request.get("/map.html");
    expect(whole.status()).toBe(200);
    expect(whole.headers()["accept-ranges"]).toBe("bytes");
    const past = await request.get("/map.html", { headers: { range: "bytes=99999999-" } });
    expect(past.status()).toBe(416);
  });

  test("serves ES modules as JavaScript, so that a module script runs", async ({ request }) => {
    expect((await request.get("/basemap-light.json")).headers()["content-type"]).toContain("application/json");
  });
});
```

(The last test needs `web/basemap-light.json`, which Task 7 creates; if running this task alone, skip that one test until Task 7 with `test.fixme`, and change it to `test` in Task 7's commit. The `.mjs` check is done in Step 6 with `curl`.)

- [ ] **Step 2: Run to see them fail**

Run: `cd e2e && npx playwright test tests/serve.spec.ts 2>&1 | tail -15`
Expected: FAIL — status 200 instead of 206.

- [ ] **Step 3: Implement range support in `e2e/serve.mjs`**

Replace the file's `types` and handler with:

```js
const types = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
  ".woff2": "font/woff2",
  ".pbf": "application/x-protobuf",
  ".pmtiles": "application/octet-stream",
};

/** The bytes a `Range` header asks of a file `size` long: [start, end], or null when it cannot be had. */
function rangeOf(header, size) {
  const m = /^bytes=(\d*)-(\d*)$/.exec(header);
  if (!m || (m[1] === "" && m[2] === "")) return undefined;
  const start = m[1] === "" ? Math.max(0, size - Number(m[2])) : Number(m[1]);
  const end = m[1] === "" || m[2] === "" ? size - 1 : Math.min(Number(m[2]), size - 1);
  return start < size && start <= end ? [start, end] : null;
}

createServer(async (req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname));
  const file = join(root, path.endsWith("/") ? path + "index.html" : path);
  if (!file.startsWith(root)) return res.writeHead(403).end();
  try {
    const body = await readFile(file);
    const headers = {
      "content-type": types[extname(file)] ?? "application/octet-stream",
      "cache-control": "no-store",
      "accept-ranges": "bytes",
    };
    const range = rangeOf(req.headers.range ?? "", body.length);
    if (range === null) return res.writeHead(416, { "content-range": `bytes */${body.length}` }).end();
    if (range === undefined) return res.writeHead(200, headers).end(body);
    const [start, end] = range;
    res.writeHead(206, { ...headers, "content-range": `bytes ${start}-${end}/${body.length}` }).end(body.subarray(start, end + 1));
  } catch {
    res.writeHead(404).end("not found");
  }
}).listen(8137, "127.0.0.1");
```

(Keep the file's existing imports and `root`.)

- [ ] **Step 4: Run the tests**

Run: `cd e2e && npx playwright test tests/serve.spec.ts 2>&1 | tail -15`
Expected: the first three PASS (the fourth is `fixme`).

- [ ] **Step 5: Vendor the libraries**

Create the repository-root `package.json`:

```json
{
  "name": "cityloom-web",
  "private": true,
  "description": "What the pages load at run time from npm: the map's libraries, copied into web/vendor by scripts/build.sh.",
  "dependencies": {
    "maplibre-gl": "latest",
    "pmtiles": "latest"
  }
}
```

Run: `npm install` at the repository root, then replace each `"latest"` in `package.json` with the caret range npm recorded (`^<version>` from `package-lock.json`), keeping the lock file. Verify: `ls node_modules/maplibre-gl/dist/maplibre-gl.mjs node_modules/maplibre-gl/dist/maplibre-gl-shared.mjs node_modules/maplibre-gl/dist/maplibre-gl-worker.mjs node_modules/maplibre-gl/dist/maplibre-gl.css node_modules/pmtiles/dist/pmtiles.js` lists all five.

In `scripts/build.sh`, after the two `wasm-bindgen` calls, append:

```sh
# The map's libraries, from npm (`npm ci` at the repository root) into web/vendor. MapLibre is ES modules that
# find each other beside themselves; pmtiles.js defines a global `pmtiles`.
[ -d node_modules/maplibre-gl ] || { echo "node_modules/maplibre-gl is missing: run 'just setup' first" >&2; exit 1; }
mkdir -p web/vendor
cp node_modules/maplibre-gl/dist/maplibre-gl.mjs node_modules/maplibre-gl/dist/maplibre-gl-shared.mjs \
  node_modules/maplibre-gl/dist/maplibre-gl-worker.mjs node_modules/maplibre-gl/dist/maplibre-gl.css \
  node_modules/pmtiles/dist/pmtiles.js web/vendor/
```

In `justfile`: the `setup` recipe gets a first line `npm ci` (before the `cargo install` line), and `serve` becomes:

```
# Serve the pages at http://127.0.0.1:8137/ (build first). The server answers range requests, as the map's tiles need.
serve:
    node e2e/serve.mjs
```

`.gitignore`: add

```
# What scripts/build.sh copies from node_modules, and the basemap's tiles, which scripts/basemap_tiles.py makes.
/node_modules/
/web/vendor/
/web/data/basemap/*.pmtiles
```

`.github/workflows/ci.yml`: in the `e2e` job's `actions/setup-node` step change `cache-dependency-path: e2e/package-lock.json` to

```yaml
          cache-dependency-path: |
            e2e/package-lock.json
            package-lock.json
```

(`just setup` already runs `npm ci` at the root now.) `.github/dependabot.yml`: add

```yaml
  - package-ecosystem: npm
    directory: /
    schedule:
      interval: weekly
```

- [ ] **Step 6: Fetch the glyphs**

```bash
S=$(mktemp -d) && gh release download v2.0 -R openmaptiles/fonts -p noto-sans.zip -D "$S" && unzip -q "$S/noto-sans.zip" -d "$S/fonts"
for stack in "Noto Sans Regular" "Noto Sans Bold"; do
  mkdir -p "web/glyphs/$stack"
  for r in 0-255 256-511 8192-8447; do cp "$S/fonts/$stack/$r.pbf" "web/glyphs/$stack/"; done
done
ls -la web/glyphs/*/ && du -sh web/glyphs
```

Expected: six `.pbf` files, about a megabyte in all. If the archive's folder layout differs, `find "$S/fonts" -name 0-255.pbf` shows where the stacks are. The Noto Sans licence (SIL OFL 1.1) text is in the archive or at `https://github.com/notofonts/latin-greek-cyrillic/blob/main/OFL.txt`: save it as `web/glyphs/LICENSE-noto-sans.txt`.

Run the build and the module-MIME check:

```bash
just build && ls web/vendor && (node e2e/serve.mjs & sleep 1; curl -sI http://127.0.0.1:8137/vendor/maplibre-gl.mjs | grep -i content-type; kill %1)
```

Expected: five files in `web/vendor`; `content-type: text/javascript; charset=utf-8`.

- [ ] **Step 7: Commit**

```bash
git add e2e/serve.mjs e2e/tests/serve.spec.ts justfile scripts/build.sh .gitignore .github package.json package-lock.json web/glyphs
git commit -m "build(map): range requests in the test server, the map's libraries from npm, and Noto Sans glyphs

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 7: The tiles: build script, fixture and the style

**Files:**
- Create: `scripts/basemap_tiles.py`
- Create: `scripts/make_basemap_style.mjs`
- Create (generated, committed): `web/basemap-light.json`, `web/basemap-dark.json`
- Create (generated, committed): `e2e/fixtures/plateau.pmtiles`
- Modify: `justfile`, `README.md`, `e2e/tests/serve.spec.ts` (turn the `fixme` into a test)

**Interfaces:**
- Produces: `just basemap-tiles quebec-latest.osm.pbf` writes `web/data/basemap/montreal.pmtiles`; `web/basemap-{light,dark}.json` are MapLibre styles with `version: 8`, one vector source named `basemap` (its `url` is a placeholder the adapter replaces), `glyphs` placeholder, layers over the OpenMapTiles schema using font stacks `Noto Sans Regular` and `Noto Sans Bold`.

- [ ] **Step 1: Write the tile build script**

Create `scripts/basemap_tiles.py` (mode 755):

```python
#!/usr/bin/env python3
"""Builds the map's basemap tiles of the Montréal metropolitan area from a Quebec OSM extract.

    scripts/basemap_tiles.py quebec-latest.osm.pbf [output.pmtiles] [--minzoom N] [--maxzoom N] [--bounds W,S,E,N]

Needs Java 21 or later and the same Geofabrik extract as scripts/metro_tiles.py
(https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf, about 1.2 GB).
Planetiler (OpenMapTiles schema) is downloaded to .tools/planetiler.jar on first use, and with --download
it fetches about 1 GB of Natural Earth and water-polygon sources once, into data/sources.
Writes web/data/basemap/montreal.pmtiles, which the map page reads with range requests.
"""

import argparse
import subprocess
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
JAR = ROOT / ".tools" / "planetiler.jar"
JAR_URL = "https://github.com/onthegomap/planetiler/releases/latest/download/planetiler.jar"
# The grid scripts/metro_tiles.py cuts: west, south, east, north.
METRO_BOUNDS = "-74.40,45.20,-73.09,45.81"
OUT = ROOT / "web" / "data" / "basemap" / "montreal.pmtiles"


def planetiler():
    if not JAR.exists():
        JAR.parent.mkdir(parents=True, exist_ok=True)
        print(f"downloading {JAR_URL}", file=sys.stderr)
        urllib.request.urlretrieve(JAR_URL, JAR)
    return JAR


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("extract", type=Path)
    ap.add_argument("output", type=Path, nargs="?", default=OUT)
    ap.add_argument("--bounds", default=METRO_BOUNDS, help="west,south,east,north in degrees")
    ap.add_argument("--minzoom", type=int, default=0)
    ap.add_argument("--maxzoom", type=int, default=14, help="MapLibre draws deeper zooms from this one")
    ap.add_argument("--memory", default="4g")
    args = ap.parse_args()
    if not args.extract.exists():
        sys.exit(f"{args.extract} is not there: get it from https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "java", f"-Xmx{args.memory}", "-jar", str(planetiler()),
            f"--osm-path={args.extract}", f"--output={args.output}",
            f"--bounds={args.bounds}", f"--minzoom={args.minzoom}", f"--maxzoom={args.maxzoom}",
            "--download", "--force", "--tmpdir=" + str(ROOT / ".tools" / "planetiler-tmp"),
        ],
        check=True,
        cwd=ROOT,
    )
    print(f"{args.output}: {args.output.stat().st_size / 1e6:.1f} MB")


if __name__ == "__main__":
    main()
```

In `justfile` after `metro-tiles`:

```
# Build the basemap's tiles of the Montréal metropolitan area in web/data/basemap, from the same extract. Needs Java 21+.
basemap-tiles quebec:
    ./scripts/basemap_tiles.py {{quebec}}
```

- [ ] **Step 2: Build the fixture**

The e2e tests need a tile file of the Plateau (the area of `e2e/fixtures/plateau.osm.pbf`). With the Quebec extract at hand:

```bash
chmod +x scripts/basemap_tiles.py
./scripts/basemap_tiles.py quebec-latest.osm.pbf e2e/fixtures/plateau.pmtiles --bounds=-73.6150,45.5170,-73.5800,45.5350 --minzoom 10 --maxzoom 14 --memory 4g
ls -la e2e/fixtures/plateau.pmtiles
```

Expected: a file of one to a few megabytes. If it is over 5 MB, raise `--minzoom` to 12 (the map is never shown below zoom 13 here). Check the header with `node -e 'import("pmtiles").then(async ({PMTiles}) => { const p = new PMTiles("file:///"); })'` is not needed; the adapter's header read is tested in Task 9. If the extract is not on this machine, stop and ask for it: the fixture cannot be made without it.

- [ ] **Step 3: Write the style generator**

Create `scripts/make_basemap_style.mjs`:

```js
#!/usr/bin/env node
// Writes web/basemap-light.json and web/basemap-dark.json: the basemap's MapLibre styles over the OpenMapTiles
// schema, from one palette per theme so that the two do not drift. The colours follow web/app.css's tokens.
// Run `node scripts/make_basemap_style.mjs` after changing the palettes or the layers, and commit the result.

import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const palettes = {
  light: {
    land: "#e2e7ec", water: "#b8d2e6", park: "#cde2c8", wood: "#bcd8b6", building: "#d3d9df", buildingLine: "#c2c9d1",
    road: "#ffffff", minor: "#f7f9fb", casing: "#c4ccd4", major: "#fde3b4", majorCasing: "#d9b574", path: "#aab3bd",
    label: "#4b5563", halo: "#ffffff", place: "#111827", waterLabel: "#4a77a1",
  },
  dark: {
    land: "#0f1317", water: "#142230", park: "#15261b", wood: "#122015", building: "#192027", buildingLine: "#222b33",
    road: "#303941", minor: "#262e35", casing: "#0b0f13", major: "#5d4d30", majorCasing: "#2a2418", path: "#3b454f",
    label: "#9aa6b2", halo: "#0f1317", place: "#e8edf2", waterLabel: "#6f9bc2",
  },
};

const REGULAR = ["Noto Sans Regular"];
const BOLD = ["Noto Sans Bold"];
const ROADS = ["motorway", "trunk", "primary", "secondary", "tertiary", "minor", "service"];
const BIG = ["motorway", "trunk", "primary"];

/** A width that grows with the zoom, by the class of the road. */
const widths = (z0, z1, byClass, fallback) => [
  "interpolate", ["exponential", 1.5], ["zoom"],
  z0, ["match", ["get", "class"], ...Object.entries(byClass[0]).flat(), fallback[0]],
  z1, ["match", ["get", "class"], ...Object.entries(byClass[1]).flat(), fallback[1]],
];

const roadWidth = (extra) =>
  widths(10, 20, [
    { motorway: 1.5 + extra, trunk: 1.5 + extra, primary: 1.2 + extra, secondary: 1 + extra, tertiary: 0.8 + extra },
    { motorway: 22 + extra, trunk: 22 + extra, primary: 20 + extra, secondary: 18 + extra, tertiary: 16 + extra, minor: 12 + extra, service: 6 + extra },
  ], [0.4 + extra, 12 + extra]);

function layers(c) {
  const isRoad = ["in", ["get", "class"], ["literal", ROADS]];
  return [
    { id: "background", type: "background", paint: { "background-color": c.land } },
    { id: "landcover", type: "fill", source: "basemap", "source-layer": "landcover", filter: ["in", ["get", "class"], ["literal", ["grass", "wood"]]],
      paint: { "fill-color": ["match", ["get", "class"], "wood", c.wood, c.park], "fill-opacity": 0.7 } },
    { id: "park", type: "fill", source: "basemap", "source-layer": "park", paint: { "fill-color": c.park, "fill-opacity": 0.8 } },
    { id: "water", type: "fill", source: "basemap", "source-layer": "water", paint: { "fill-color": c.water } },
    { id: "waterway", type: "line", source: "basemap", "source-layer": "waterway",
      paint: { "line-color": c.water, "line-width": ["interpolate", ["linear"], ["zoom"], 8, 0.5, 16, 3] } },
    { id: "building", type: "fill", source: "basemap", "source-layer": "building", minzoom: 14,
      paint: { "fill-color": c.building, "fill-outline-color": c.buildingLine } },
    { id: "path", type: "line", source: "basemap", "source-layer": "transportation", minzoom: 14,
      filter: ["in", ["get", "class"], ["literal", ["path", "pedestrian", "track"]]],
      paint: { "line-color": c.path, "line-width": ["interpolate", ["linear"], ["zoom"], 14, 0.6, 20, 2], "line-dasharray": [2, 2] } },
    { id: "road-casing", type: "line", source: "basemap", "source-layer": "transportation", filter: isRoad,
      layout: { "line-cap": "round", "line-join": "round" },
      paint: { "line-color": ["match", ["get", "class"], ...BIG.flatMap((k) => [k, c.majorCasing]), c.casing], "line-width": roadWidth(1.2) } },
    { id: "road", type: "line", source: "basemap", "source-layer": "transportation", filter: isRoad,
      layout: { "line-cap": "round", "line-join": "round" },
      paint: { "line-color": ["match", ["get", "class"], "minor", c.minor, "service", c.minor, ...BIG.flatMap((k) => [k, c.major]), c.road], "line-width": roadWidth(0) } },
    { id: "road-name", type: "symbol", source: "basemap", "source-layer": "transportation_name", minzoom: 14,
      layout: { "symbol-placement": "line", "text-field": ["get", "name"], "text-font": REGULAR, "text-size": 11, "text-letter-spacing": 0.02 },
      paint: { "text-color": c.label, "text-halo-color": c.halo, "text-halo-width": 1.5 } },
    { id: "water-name", type: "symbol", source: "basemap", "source-layer": "water_name",
      layout: { "text-field": ["get", "name"], "text-font": REGULAR, "text-size": 12 },
      paint: { "text-color": c.waterLabel, "text-halo-color": c.halo, "text-halo-width": 1.2 } },
    { id: "place", type: "symbol", source: "basemap", "source-layer": "place", minzoom: 10,
      layout: { "text-field": ["get", "name"], "text-font": BOLD, "text-size": ["match", ["get", "class"], "city", 16, "suburb", 13, 12] },
      paint: { "text-color": c.place, "text-halo-color": c.halo, "text-halo-width": 1.5 } },
  ];
}

const style = (theme) => ({
  version: 8,
  name: `CityLoom ${theme}`,
  // The adapter replaces these two with absolute addresses: MapLibre needs them absolute.
  glyphs: "GLYPHS",
  sources: {
    basemap: {
      type: "vector",
      url: "pmtiles://TILES",
      attribution: '© <a href="https://openmaptiles.org/">OpenMapTiles</a> © <a href="https://www.openstreetmap.org/copyright">OpenStreetMap contributors</a>',
    },
  },
  layers: layers(palettes[theme]),
});

for (const theme of Object.keys(palettes)) {
  const file = fileURLToPath(new URL(`../web/basemap-${theme}.json`, import.meta.url));
  writeFileSync(file, JSON.stringify(style(theme), null, 2) + "\n");
  console.log(file);
}
```

- [ ] **Step 4: Generate and validate the styles**

```bash
node scripts/make_basemap_style.mjs
node -e '
const { validateStyleMin } = require("@maplibre/maplibre-gl-style-spec");
' 2>/dev/null || npx --yes @maplibre/maplibre-gl-style-spec@latest validate web/basemap-light.json web/basemap-dark.json
```

Expected: no errors from the validator (`gl-style-validate` prints nothing on success). If it reports an expression error, fix the generator, not the JSON.

- [ ] **Step 5: Enable the server's JSON test and commit**

In `e2e/tests/serve.spec.ts` change the `test.fixme` (if you used it) back to `test`. Run: `cd e2e && npx playwright test tests/serve.spec.ts` — expected: 4 PASS.

In `README.md`, after the "Montréal metro tiles" section, add:

```markdown
## The basemap

The map page draws the city over a basemap of the Montréal metropolitan area, from a file of vector tiles that is served with the pages and read with HTTP range requests (any host that serves static files with range support will do; `just serve` does):

```sh
brew install openjdk          # Planetiler needs Java 21 or later
just basemap-tiles quebec-latest.osm.pbf   # the same extract as above; about 1 GB of sources are downloaded once; writes web/data/basemap/montreal.pmtiles
```

The tiles are not kept in git. Without them the map page says the basemap could not be loaded and shows no map, and a place outside the metropolitan area says there is no basemap for it; the street and junction editors work either way. The style is `web/basemap-light.json` and `web/basemap-dark.json`, written by `node scripts/make_basemap_style.mjs`.
```

```bash
git add scripts/basemap_tiles.py scripts/make_basemap_style.mjs web/basemap-light.json web/basemap-dark.json e2e/fixtures/plateau.pmtiles e2e/tests/serve.spec.ts justfile README.md
git commit -m "feat(map): the basemap's tiles, a fixture of the Plateau, and its light and dark styles

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 8: The browser adapter and the page

**Files:**
- Create: `web/basemap.js`
- Modify: `web/map.js`, `web/map.html`, `web/app.css`
- Modify: `src/shared/platform.rs` (the browser's `Mapper`), `src/map/mod.rs` (`mount_map`), `src/map/view.rs` (`MapView`, `Legend`), `src/map/tests.rs`

**Interfaces:**
- Consumes: `Mapper`, `MapEvent` (Task 3); `MapVm::{attach, basemap_state, sync_places, sync_highlight, set_insets, press_key, status_key}` (Task 5); `web/vendor/*` (Task 6); `web/basemap-*.json`, `web/glyphs` (Tasks 6 and 7).
- Produces: `createBasemap(containerId: string): Promise<Adapter>` where `Adapter` has the methods `setPlaces(layers, data)`, `fit(w, s, e, n, top, right, bottom, left)`, `zoomBy(factor)`, `panBy(dx, dy)`, `highlight(hot|null)`, `setImperial(bool)`, `listen(fn)` (the function receives a JSON string of a `MapEvent`); `window.cityloomMap` is the MapLibre map (for the browser tests). `mount_map(basemap: Basemap) -> MapPage` in the wasm module.

- [ ] **Step 1: Write the component tests that must change** (done in Task 5 Step 5; check they are there)

In `src/map/tests.rs`, delete these tests (they test the SVG map the page no longer has): `the_map_is_a_labelled_group_holding_every_place_as_a_link_and_a_scale_bar`, `the_place_the_pointer_is_on_is_outlined_on_the_map`, `the_map_follows_the_zoom_and_the_units`, `the_key_lists_what_the_streets_are_made_of`. Add:

```rust
#[test]
fn a_map_page_with_nothing_to_show_says_why() {
    let (vm, _) = map_vm(); // the sample city: no roads of a place, so no map
    let h = map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    assert!(h.contains("role=\"status\"") && h.contains("The roads of this place could not be loaded"), "{h}");
    assert!(!h.contains("hidden"), "{h}");
}

#[test]
fn the_key_says_what_the_colours_of_the_places_mean() {
    let (vm, _) = map_vm();
    let h = map_html(|| view! { <map_ui::Legend vm=vm.clone()/> });
    assert_eq!(count(&h, "<li>"), 3);
    for words in ["Works", "Changed", "Needs attention"] {
        assert!(h.contains(words), "{words}");
    }
    assert!(h.contains("dot-bad") && h.contains("dot-ok") && h.contains("dot-changed"));
}
```

Keep `the_map_tools_zoom_in_out_and_to_the_whole_city` and `the_hero_map_draws_every_junction_but_cannot_be_reached_or_read_out` as they are.

- [ ] **Step 2: Run to see them fail**

Run: `cargo test map::tests 2>&1 | tail -15`
Expected: FAIL — the old `MapView`/`Legend` still refer to removed view-model methods, or the new assertions do not hold.

- [ ] **Step 3: Rewrite `MapView` and `Legend` in `src/map/view.rs`**

Replace the `use` for `svg` with nothing (the hero imports `map_svg` itself) and delete `hot_of`, `window_of` if unused, and the `PointerEvent`/`WheelEvent` imports. Replace `MapView` and `Legend` with:

```rust
/// What binds the page to the map: the keyboard on the map, what floats over it, and keeping the map in step
/// with the city. The map itself is drawn by MapLibre into `#basemap`, which the page holds; this shows only
/// why there is none when there is none.
#[component]
pub fn MapView(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    // The places follow the city and whether the map is ready; the highlight follows the pointer.
    Effect::new(move |_| {
        vm.with(|v| {
            v.view();
            v.basemap_state();
        });
        vm.with(|v| v.sync_places());
    });
    Effect::new(move |_| {
        vm.with(|v| v.hot());
        vm.with(|v| v.sync_highlight());
    });

    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::closure::Closure;
        Effect::new(move |_| {
            let Some(el) = leptos::prelude::document().get_element_by_id("basemap") else { return };
            let key = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
                if e.meta_key() || e.ctrl_key() || e.alt_key() {
                    return;
                }
                if vm.with(|v| v.press_key(&e.key())) {
                    e.prevent_default();
                }
            });
            let _ = el.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref());
            key.forget();
            // What floats over the map is kept clear of when the places are fitted.
            let fit = {
                let el = el.clone();
                move || {
                    let r = el.get_bounding_client_rect();
                    if r.width() > 0.0 && r.height() > 0.0 {
                        vm.with(|v| v.set_insets(insets_of(&el)));
                    }
                }
            };
            fit();
            let observer = Closure::<dyn FnMut(leptos::web_sys::js_sys::Array)>::new(move |_| fit());
            if let Ok(o) = leptos::web_sys::ResizeObserver::new(observer.as_ref().unchecked_ref()) {
                o.observe(&el);
                observe_covers(&o);
            }
            observer.forget();
        });
        // What the other pages wrote is read again when the page is shown, and when another tab writes.
        window_event_listener(leptos::ev::storage, move |e: leptos::web_sys::StorageEvent| {
            if e.key().is_none_or(|k| k == crate::city::store::CITY_KEY) {
                vm.with(|v| v.reload());
            }
        });
        window_event_listener(leptos::ev::pageshow, move |e: leptos::web_sys::PageTransitionEvent| {
            if e.persisted() {
                vm.with(|v| v.reload());
            }
        });
    }

    view! {
        <p
            class="basemap-note"
            role="status"
            hidden=move || !matches!(vm.with(|v| v.basemap_state()), BasemapState::Unavailable(_))
        >
            {move || match vm.with(|v| v.basemap_state()) {
                BasemapState::Unavailable(words) => words,
                _ => "",
            }}
        </p>
    }
}

/// A key to what the colours of the places on the map say.
#[component]
pub fn Legend(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    move || {
        vm.with(|v| v.status_key())
            .into_iter()
            .map(|(id, name)| {
                view! {
                    <li>
                        <span class=format!("dot dot-{id}") aria-hidden="true"></span>
                        {name}
                    </li>
                }
            })
            .collect_view()
    }
}
```

Add `use crate::map::vm::BasemapState;` to the imports (alongside `MapVm, NoteItem, PlaceRow, ResetOutcome`).

- [ ] **Step 4: Run the component tests**

Run: `cargo test 2>&1 | tail -12 && cargo build --target wasm32-unknown-unknown --workspace 2>&1 | tail -5`
Expected: all PASS; the wasm build is clean (unused `window_of`/`hot_of` and imports are the likely warnings: delete them).

- [ ] **Step 5: The browser's `Mapper` and `mount_map`**

In `src/shared/platform.rs` add (with the module's existing `wasm_bindgen` imports; add `use wasm_bindgen::prelude::*;` and `use wasm_bindgen::closure::Closure;` if absent):

```rust
#[wasm_bindgen]
extern "C" {
    /// The map of the page: the adapter `createBasemap` makes in `web/basemap.js`.
    pub type Basemap;
    #[wasm_bindgen(method, js_name = setPlaces)]
    fn set_places(this: &Basemap, layers: &str, data: &str);
    #[wasm_bindgen(method)]
    #[allow(clippy::too_many_arguments)]
    fn fit(this: &Basemap, west: f64, south: f64, east: f64, north: f64, top: f64, right: f64, bottom: f64, left: f64);
    #[wasm_bindgen(method, js_name = zoomBy)]
    fn zoom_by(this: &Basemap, factor: f64);
    #[wasm_bindgen(method, js_name = panBy)]
    fn pan_by(this: &Basemap, dx: f64, dy: f64);
    #[wasm_bindgen(method)]
    fn highlight(this: &Basemap, hot: Option<String>);
    #[wasm_bindgen(method, js_name = setImperial)]
    fn set_imperial(this: &Basemap, imperial: bool);
    #[wasm_bindgen(method)]
    fn listen(this: &Basemap, on: &js_sys::Function);
}

/// The map on the page, as a `Mapper`.
pub struct BrowserMapper(Basemap);

impl BrowserMapper {
    pub fn new(basemap: Basemap) -> BrowserMapper {
        BrowserMapper(basemap)
    }
}

impl crate::shared::ports::Mapper for BrowserMapper {
    fn set_places(&self, layers: &str, data: &str) {
        self.0.set_places(layers, data);
    }
    fn fit(&self, [w, s, e, n]: [f64; 4], [top, right, bottom, left]: [f64; 4]) {
        self.0.fit(w, s, e, n, top, right, bottom, left);
    }
    fn zoom_by(&self, factor: f64) {
        self.0.zoom_by(factor);
    }
    fn pan_by(&self, dx: f64, dy: f64) {
        self.0.pan_by(dx, dy);
    }
    fn highlight(&self, hot: Option<&str>) {
        self.0.highlight(hot.map(String::from));
    }
    fn set_imperial(&self, imperial: bool) {
        self.0.set_imperial(imperial);
    }
    fn listen(&self, on: Box<dyn Fn(crate::shared::ports::MapEvent)>) {
        let callback = Closure::<dyn Fn(String)>::new(move |json: String| {
            if let Ok(event) = serde_json::from_str(&json) {
                on(event);
            }
        });
        self.0.listen(callback.as_ref().unchecked_ref());
        callback.forget();
    }
}
```

(Remove the `#[allow(clippy::too_many_arguments)]` line: there is no clippy configuration, and an unknown lint name would not warn but is noise.)

In `src/map/mod.rs` change `mount_map`:

```rust
/// Draws the city map page into the elements it keeps for it, over the basemap the script made, and hands
/// back what the script needs to reach it.
#[wasm_bindgen]
pub fn mount_map(basemap: crate::shared::platform::Basemap) -> MapPage {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let ports = crate::shared::ports::Ports { mapper: Rc::new(crate::shared::platform::BrowserMapper::new(basemap)), ..browser_ports() };
    let vm = vm::MapVm::new(ports);
    vm.attach();
```

(The rest of `mount_map` is unchanged: the `mount(...)` calls, including `map-slot` for `MapView` and `plan-key` for `Legend`.) Update the module's doc comment: it no longer holds gestures.

- [ ] **Step 6: Write the adapter**

Create `web/basemap.js`:

```js
// The map on the city map page: MapLibre over the PMTiles basemap. What the page decides is decided in the
// WebAssembly module. This turns what MapLibre does into the JSON events the module listens for, and what the
// module asks into calls on MapLibre.

import { AttributionControl, Map as MapLibreMap, ScaleControl, addProtocol } from "./vendor/maplibre-gl.mjs";

const TILES = "data/basemap/montreal.pmtiles";
// MapLibre needs absolute addresses for what a style names.
const here = (path) => new URL(path, document.baseURI).href;

const themeNow = () => {
  const set = document.documentElement.dataset.theme;
  if (set === "light" || set === "dark") return set;
  return matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
};

async function styleFor(theme) {
  const style = await (await fetch(`basemap-${theme}.json`, { cache: "no-cache" })).json();
  style.glyphs = here("glyphs/") + "{fontstack}/{range}.pbf";
  style.sources.basemap.url = "pmtiles://" + here(TILES);
  return style;
}

export async function createBasemap(container) {
  const protocol = new pmtiles.Protocol();
  addProtocol("pmtiles", protocol.tile);
  const archive = new pmtiles.PMTiles(here(TILES));
  protocol.add(archive);

  // What is said before the module listens waits for it.
  let listener = null;
  const queue = [];
  const emit = (event) => {
    const json = JSON.stringify(event);
    if (listener) listener(json);
    else queue.push(json);
  };
  archive.getHeader().then(
    (h) => emit({ kind: "ready", bounds: [h.minLon, h.minLat, h.maxLon, h.maxLat] }),
    () => emit({ kind: "failed" }),
  );

  let theme = themeNow();
  const map = new MapLibreMap({
    container,
    style: await styleFor(theme),
    center: [0, 0],
    zoom: 1,
    maxZoom: 19,
    attributionControl: false,
    dragRotate: false,
    pitchWithRotate: false,
    touchPitch: false,
    keyboard: false, // the shortcuts are the page's
  });
  map.touchZoomRotate.disableRotation();
  map.addControl(new AttributionControl({ compact: true }));
  let scale = new ScaleControl({ unit: "metric" });
  map.addControl(scale, "bottom-left");
  window.cityloomMap = map;

  // The places: kept, so that a new style (a new theme) can have them put back.
  let places = null;
  let hot = null;
  let styled = false;
  const showHot = () => {
    if (styled && map.getSource("places") && hot) map.setFeatureState({ source: "places", id: hot }, { hot: true });
  };
  const showPlaces = () => {
    if (!styled || !places) return;
    const data = JSON.parse(places.data);
    const source = map.getSource("places");
    if (source) {
      source.setData(data);
    } else {
      map.addSource("places", { type: "geojson", data, promoteId: "hot" });
      for (const layer of JSON.parse(places.layers)) map.addLayer(layer);
    }
    showHot();
  };
  map.on("style.load", () => {
    styled = true;
    showPlaces();
  });

  const retheme = async () => {
    const next = themeNow();
    if (next === theme) return;
    theme = next;
    styled = false;
    map.setStyle(await styleFor(theme), { diff: false });
  };
  new MutationObserver(retheme).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", retheme);

  // What is under the pointer: a junction before a street.
  const hotAt = (point) => {
    const layers = ["places-junction", "places-street"].filter((id) => map.getLayer(id));
    if (!layers.length) return null;
    const box = [
      [point.x - 6, point.y - 6],
      [point.x + 6, point.y + 6],
    ];
    const found = map.queryRenderedFeatures(box, { layers });
    const feature = found.find((f) => f.layer.id === "places-junction") ?? found[0];
    return feature ? feature.properties.hot : null;
  };
  let over = null;
  map.on("mousemove", (e) => {
    const now = hotAt(e.point);
    if (now === over) return;
    over = now;
    map.getCanvas().style.cursor = now ? "pointer" : "";
    emit({ kind: "hover", hot: now });
  });
  map.getCanvas().addEventListener("mouseleave", () => {
    over = null;
    emit({ kind: "hover", hot: null });
  });
  map.on("click", (e) => {
    const now = hotAt(e.point);
    if (now) emit({ kind: "pick", hot: now });
  });
  // A move that came from the person's own pointer or wheel, not from the page.
  map.on("movestart", (e) => {
    if (e.originalEvent) emit({ kind: "moved" });
  });

  return {
    setPlaces(layers, data) {
      places = { layers, data };
      showPlaces();
    },
    fit(west, south, east, north, top, right, bottom, left) {
      map.fitBounds(
        [
          [west, south],
          [east, north],
        ],
        { padding: { top, right, bottom, left }, duration: 300 },
      );
    },
    zoomBy(factor) {
      map.easeTo({ zoom: map.getZoom() + Math.log2(factor), duration: 200 });
    },
    // A positive dx moves the view east, a positive dy south.
    panBy(dx, dy) {
      map.panBy([dx, dy], { duration: 150 });
    },
    highlight(next) {
      if (styled && map.getSource("places") && hot) map.setFeatureState({ source: "places", id: hot }, { hot: false });
      hot = next;
      showHot();
    },
    setImperial(imperial) {
      map.removeControl(scale);
      scale = new ScaleControl({ unit: imperial ? "imperial" : "metric" });
      map.addControl(scale, "bottom-left");
    },
    listen(on) {
      listener = on;
      queue.splice(0).forEach(on);
    },
  };
}
```

- [ ] **Step 7: Wire the page**

`web/map.js` becomes:

```js
// Starts the city map page. The page is drawn and driven by the WebAssembly module (`mount_map`); the map itself
// is MapLibre, made by basemap.js; this sets up what every page shares, which the module binds (`mount_shell`).

import init, { mount_map, prepare_city } from "./pkg/cityloom_editor.js";
import { createBasemap } from "./basemap.js";
import "./city.js";

await init();
// The roads of the area being worked in, kept before the city is opened; if they cannot be got, the page says
// so and has no map to show.
await prepare_city();

mount_map(await createBasemap("basemap")).mount_shell();
```

`web/map.html`: in `<head>` after the `app.css` link add `<link rel="stylesheet" href="vendor/maplibre-gl.css">`; before `<script type="module" src="map.js">` add `<script src="vendor/pmtiles.js"></script>`; delete the `<svg id="defs" …>` line; delete the whole `<p class="menu-act"><button … id="print" …>Print this sheet</button></p>` line; in the `.drawing` section replace the `#map-slot` line with

```html
      <div id="basemap" class="basemap" tabindex="0" role="group" aria-label="Map of the city, north up" aria-describedby="keys"></div>
      <div id="map-slot" style="display: contents"></div>
```

and change the `<h2 id="h-key">Key to the streets</h2>` text and the `ul`'s `aria-label` to `Key to the map` / `Key to the map`.

`web/app.css`: find the rules for `.map-view` (and `.map-view:focus-visible`) and give `.basemap` the same box (fill the `.drawing`, same focus ring), then add:

```css
/* The map is MapLibre's; a place that cannot be shown says why over it. */
.basemap { position: absolute; inset: 0; }
.basemap-note { position: absolute; inset: auto 50% 50% auto; transform: translate(50%, 50%); max-width: 36ch; margin: 0; padding: 12px 16px; background: var(--panel); color: var(--ink); border: 1px solid var(--rule); border-radius: var(--r-ctl); box-shadow: var(--shadow-soft); z-index: 2; }
.basemap-note[hidden] { display: none; }
.dot { display: inline-block; width: 12px; height: 12px; margin-right: 8px; border-radius: 50%; border: 2px solid var(--panel); box-shadow: 0 0 0 1px var(--field-line); vertical-align: -1px; }
.dot-ok { background: #4b6b8a; }
.dot-changed { background: #2563eb; }
.dot-bad { background: #dc2626; }
```

(The three colours repeat `style::OK`, `CHANGED` and `BAD`; keep them equal. Adjust the `.basemap` positioning to whatever the old `.map-view` used, since it sits where that did.) Remove CSS for classes only the old SVG map used on this page (`#map` rules under `.map-view`, `.map-overlay`, `.m-hl`) if nothing else uses them; the hero's `#map .m-*` rules in `home.css` stay.

- [ ] **Step 8: Build and look at it**

```bash
just build
node e2e/serve.mjs &
```

In Chromium (the `claude-in-chrome` tools, or `cd e2e && npx playwright test --headed` with a throw-away spec), with the fixture tiles copied for a look: `mkdir -p web/data/basemap && cp e2e/fixtures/plateau.pmtiles web/data/basemap/montreal.pmtiles`, and the Plateau's roads from Overpass or a metro tile, open `http://127.0.0.1:8137/map.html`. Expected: the basemap draws (water, parks, roads, labels), the city's streets sit on its roads in blue or red, junction discs have white rims, panels float over it, and the note is hidden. Check by eye in both themes (Settings, Theme). Hidden tabs throttle timers and `requestAnimationFrame` (CLAUDE.md): keep the tab visible. If WebGL does not start in the headless Chromium Playwright uses, see Task 9 Step 1 before going on; do not paper over it.

Kill the server (`kill %1`).

- [ ] **Step 9: Format and commit**

```bash
just format && just test
git add web/basemap.js web/map.js web/map.html web/app.css src/shared/platform.rs src/map
git commit -m "feat(map): the map page draws the city over the basemap through MapLibre

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```


---

### Task 9: End-to-end tests

**Files:**
- Modify: `e2e/tests/fixtures.ts`, `e2e/tests/map.spec.ts` (rewrite), `e2e/tests/search.spec.ts`, `e2e/tests/home.spec.ts`, `e2e/tests/world.spec.ts`

**Interfaces:**
- Consumes: `window.cityloomMap` (the MapLibre map), `#basemap`, `.basemap-note`, `a.place-row`, the Places list; `e2e/fixtures/plateau.pmtiles` (Task 7).
- Produces from `fixtures.ts`: `serveBasemap(context, file?)` (serves the tiles with range support), `serveWorld(page)` (the metro index and tile of the Plateau, Overpass refused), `mapReady(page)` (waits until the places are on the map), and a default `serveBasemap` for every test.

- [ ] **Step 1: Prove WebGL in the headless browser, first**

Create `e2e/tests/webgl.spec.ts` (kept, small, and the first thing to fail if the CI browser cannot draw):

```ts
import { expect, test } from "./fixtures";

test("the browser can draw the map: MapLibre starts and its canvas has a WebGL context", async ({ page }) => {
  await page.goto("/map.html");
  await expect(page.locator("#basemap canvas")).toBeVisible();
  const gl = await page.evaluate(() => {
    const c = document.querySelector<HTMLCanvasElement>("#basemap canvas")!;
    return !!(c.getContext("webgl2") || c.getContext("webgl"));
  });
  expect(gl).toBe(true);
});
```

Run `just e2e tests/webgl.spec.ts`. Expected: PASS. If it fails with no WebGL context, add `launchOptions: { args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] }` to the Chromium project's `use` in `e2e/playwright.config.ts`, and run it again; if that does not work either, stop and report: the rest of this task cannot be tested without it.

- [ ] **Step 2: Shared helpers in `fixtures.ts`**

Add to `e2e/tests/fixtures.ts` (with `import { readFileSync } from "node:fs"; import { fileURLToPath } from "node:url";` and `type BrowserContext` from Playwright):

```ts
export const PLATEAU_PBF = fileURLToPath(new URL("../fixtures/plateau.osm.pbf", import.meta.url));
export const PLATEAU_TILES = fileURLToPath(new URL("../fixtures/plateau.pmtiles", import.meta.url));

/** Serves the basemap's tiles as a static host with range support does: PMTiles reads them in pieces. */
export async function serveBasemap(context: BrowserContext, file = PLATEAU_TILES) {
  const body = readFileSync(file);
  await context.route("**/data/basemap/montreal.pmtiles", async (route) => {
    const range = /bytes=(\d+)-(\d*)/.exec(route.request().headers()["range"] ?? "");
    if (!range) return route.fulfill({ body, headers: { "accept-ranges": "bytes" } });
    const start = Number(range[1]);
    const end = Math.min(range[2] ? Number(range[2]) : body.length - 1, body.length - 1);
    await route.fulfill({
      status: 206,
      body: body.subarray(start, end + 1),
      headers: { "content-range": `bytes ${start}-${end}/${body.length}`, "accept-ranges": "bytes" },
    });
  });
}

/** What the Plateau needs where there is no network: a metro tile of it, and no Overpass. */
export async function serveWorld(page: Page) {
  await page.route("**/data/metro/index.json", (route) =>
    route.fulfill({ json: { lon0: -73.6078, lat0: 45.5161, dlon: 0.0257, dlat: 0.018, tiles: ["0_0"] } }),
  );
  await page.route("**/data/metro/0_0.osm.pbf", (route) => route.fulfill({ body: readFileSync(PLATEAU_PBF) }));
  await page.route("https://overpass-api.de/**", (route) => route.abort());
}

/** Waits until the places are on the map and its tiles are drawn. */
export async function mapReady(page: Page) {
  await page.waitForFunction(() => {
    const m = (window as any).cityloomMap;
    return !!m && !!m.getSource("places") && m.areTilesLoaded() && m.querySourceFeatures("places").length > 0;
  });
}
```

In the `context` fixture, before `await use(context)` add `await serveBasemap(context);` (every test then has tiles, so a map page on the sample city, which has no roads and ignores them, causes no failed request).

- [ ] **Step 3: Rewrite `e2e/tests/map.spec.ts`**

```ts
import { expect, mapReady, serveBasemap, serveWorld, test } from "./fixtures";

// The map page works on a real area: the default one, the Plateau Mont-Royal, from a metro tile.
test.use({ area: "world" });

const zoom = (page: import("@playwright/test").Page) => page.evaluate(() => (window as any).cityloomMap.getZoom());
const centre = (page: import("@playwright/test").Page) =>
  page.evaluate(() => {
    const c = (window as any).cityloomMap.getCenter();
    return { lng: c.lng, lat: c.lat };
  });

test.describe("the city map", () => {
  test.beforeEach(async ({ page }) => {
    await serveWorld(page);
    await page.goto("/map.html");
    await mapReady(page);
  });

  test("shows the area over a basemap, with its streets and junctions as places", async ({ page }) => {
    await expect(page.locator("#title-block")).toContainText("Plateau");
    await expect(page.locator("#basemap canvas")).toBeVisible();
    await expect(page.locator("#basemap")).toHaveAttribute("aria-label", "Map of the city, north up");
    await expect(page.locator(".basemap-note")).toBeHidden();
    expect(await page.locator("a.place-row[href^='street.html']").count()).toBeGreaterThan(50);
    expect(await page.locator("a.place-row[href^='intersection.html']").count()).toBeGreaterThan(10);
    const kinds = await page.evaluate(() =>
      [...new Set((window as any).cityloomMap.querySourceFeatures("places").map((f: any) => f.geometry.type))],
    );
    expect(kinds.sort()).toEqual(["LineString", "Point"]);
  });

  test("the streets lie on the basemap's roads", async ({ page }) => {
    // At a street's middle, the basemap draws a road within a few pixels: the overlay is where the earth is.
    await page.evaluate(() => (window as any).cityloomMap.jumpTo({ zoom: 16 }));
    await mapReady(page);
    const result = await page.evaluate(() => {
      const map = (window as any).cityloomMap;
      const lines = map.querySourceFeatures("places").filter((f: any) => f.geometry.type === "LineString" && f.properties.width_m >= 6);
      const roads = ["road", "road-casing"];
      let on = 0;
      let seen = 0;
      for (const f of lines.slice(0, 40)) {
        const c = f.geometry.coordinates;
        const mid = c[Math.floor(c.length / 2)];
        const p = map.project(mid);
        if (p.x < 0 || p.y < 0 || p.x > map.getCanvas().clientWidth || p.y > map.getCanvas().clientHeight) continue;
        seen++;
        if (map.queryRenderedFeatures([[p.x - 8, p.y - 8], [p.x + 8, p.y + 8]], { layers: roads }).length > 0) on++;
      }
      return { on, seen };
    });
    expect(result.seen).toBeGreaterThan(3);
    expect(result.on / result.seen).toBeGreaterThan(0.8);
  });

  test("a place in the list opens its editor", async ({ page }) => {
    const href = await page.locator("a.place-row[href^='street.html']").first().getAttribute("href");
    await page.locator(`a.place-row[href="${href}"]`).click();
    await expect(page).toHaveURL(new RegExp(href!.replace("?", "\\?") + "$"));
  });

  test("pressing a place on the map opens it", async ({ page }) => {
    const spot = await page.evaluate(() => {
      const map = (window as any).cityloomMap;
      const f = map.querySourceFeatures("places").find((f: any) => f.geometry.type === "Point");
      const p = map.project(f.geometry.coordinates);
      const r = map.getCanvas().getBoundingClientRect();
      return { x: r.left + p.x, y: r.top + p.y, hot: f.properties.hot };
    });
    await page.mouse.move(spot.x, spot.y);
    await page.mouse.click(spot.x, spot.y);
    await expect(page).toHaveURL(new RegExp(`intersection\\.html\\?junction=${spot.hot.slice(2)}$`));
  });

  test("the pointer over a place on the map highlights it in the list too", async ({ page }) => {
    const spot = await page.evaluate(() => {
      const map = (window as any).cityloomMap;
      const f = map.querySourceFeatures("places").find((f: any) => f.geometry.type === "Point");
      const p = map.project(f.geometry.coordinates);
      const r = map.getCanvas().getBoundingClientRect();
      return { x: r.left + p.x, y: r.top + p.y, hot: f.properties.hot };
    });
    await page.mouse.move(spot.x, spot.y);
    await expect(page.locator("a.place-row.on")).toHaveCount(1);
    await page.mouse.move(2, 400);
    await expect(page.locator("a.place-row.on")).toHaveCount(0);
  });

  test("zooming in and out moves the view and the whole city button brings it back", async ({ page }) => {
    const fitted = await zoom(page);
    await page.locator("#zoom-in").click();
    await expect.poll(() => zoom(page)).toBeGreaterThan(fitted + 0.3);
    await page.locator("#zoom-out").click();
    await page.locator("#zoom-fit").click();
    await expect.poll(async () => Math.abs((await zoom(page)) - fitted)).toBeLessThan(0.05);
  });

  test("the keys move the map when it has focus: zoom and the four directions", async ({ page }) => {
    const fitted = await zoom(page);
    const start = await centre(page);
    await page.locator("#basemap").focus();
    await page.keyboard.press("+");
    await expect.poll(() => zoom(page)).toBeGreaterThan(fitted + 0.3);
    await page.keyboard.press("0");
    await expect.poll(async () => Math.abs((await zoom(page)) - fitted)).toBeLessThan(0.05);
    await page.keyboard.press("ArrowRight");
    await expect.poll(async () => (await centre(page)).lng).toBeGreaterThan(start.lng);
    await page.keyboard.press("ArrowUp");
    await expect.poll(async () => (await centre(page)).lat).toBeGreaterThan(start.lat);
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect.poll(async () => (await centre(page)).lat).toBeLessThan(start.lat);
  });

  test("the units change the scale", async ({ page }) => {
    await expect(page.locator(".maplibregl-ctrl-scale")).toContainText(/\d\s*m$/);
    await page.locator("#account-btn").click();
    await page.locator('[data-unit="ft"]').click();
    await expect(page.locator(".maplibregl-ctrl-scale")).toContainText(/(ft|mi)$/);
  });

  test("a theme changes the basemap and keeps the places and the highlight", async ({ page }) => {
    const land = () => page.evaluate(() => (window as any).cityloomMap.getPaintProperty("background", "background-color"));
    const light = await land();
    await page.locator("#account-btn").click();
    await page.locator('[data-theme-set="dark"]').click();
    await expect.poll(land).not.toBe(light);
    await mapReady(page); // the places came back with the new style
    const spot = await page.evaluate(() => {
      const map = (window as any).cityloomMap;
      const f = map.querySourceFeatures("places").find((f: any) => f.geometry.type === "Point");
      const p = map.project(f.geometry.coordinates);
      const r = map.getCanvas().getBoundingClientRect();
      return { x: r.left + p.x, y: r.top + p.y };
    });
    await page.mouse.move(spot.x, spot.y);
    await expect(page.locator("a.place-row.on")).toHaveCount(1);
  });

  test("a change made in a street's editor shows on the map and in start over", async ({ page }) => {
    await expect(page.locator("#reset")).toBeDisabled();
    const href = (await page.locator("a.place-row[href^='street.html']").first().getAttribute("href"))!;
    await page.goto("/" + href);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("+");
    await page.goto("/map.html");
    await mapReady(page);
    await expect(page.locator("#reset")).toBeEnabled();
    const marked = await page.evaluate(() =>
      (window as any).cityloomMap.querySourceFeatures("places").some((f: any) => f.properties.status !== "ok"),
    );
    expect(marked).toBe(true);
    await page.locator("#t-changes").click();
    await expect(page.locator("#changes")).not.toBeEmpty();
  });

  test("start over asks to be pressed twice, and then puts the city back as first laid out", async ({ page }) => {
    const href = (await page.locator("a.place-row[href^='street.html']").first().getAttribute("href"))!;
    await page.goto("/" + href);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("+");
    await page.goto("/map.html");
    await page.locator("#reset").click();
    await expect(page.locator("#reset-label")).toHaveText("Press again to start over");
    await page.locator("#reset").click();
    await expect(page.locator("#reset")).toBeDisabled();
  });
});

test.describe("when there is no basemap", () => {
  test("tiles that cannot be had are said, and the page and its places still work", async ({ page, context, errors }) => {
    await serveWorld(page);
    await context.unroute("**/data/basemap/montreal.pmtiles");
    await page.route("**/data/basemap/montreal.pmtiles", (route) => route.fulfill({ status: 404, body: "" }));
    await page.goto("/map.html");
    await expect(page.locator(".basemap-note")).toContainText("The basemap could not be loaded");
    await expect(page.locator(".basemap-note")).toContainText("just basemap-tiles");
    expect(await page.locator("a.place-row[href^='street.html']").count()).toBeGreaterThan(50);
    errors.length = 0; // the failed request is the point of the test
  });

  test("an area whose roads could not be got says so", async ({ page }) => {
    await page.route("**/data/metro/index.json", (route) => route.fulfill({ json: { lon0: 0, lat0: 0, dlon: 1, dlat: 1, tiles: [] } }));
    await page.route("https://overpass-api.de/**", (route) => route.abort());
    await page.goto("/map.html");
    await expect(page.locator(".basemap-note")).toContainText("The roads of this place could not be loaded");
  });
});

test.describe("on the sample city, which has no map", () => {
  test.use({ area: "sample" });
  test("says there is no map, and lists the places", async ({ page }) => {
    await page.goto("/map.html");
    await expect(page.locator(".basemap-note")).toContainText("could not be loaded");
    await expect(page.locator("a.place-row")).toHaveCount(32);
  });
});
```

(`serveBasemap` is imported for symmetry with other specs; drop the import if unused. In the "keys" test, if `ArrowRight` moves the view the wrong way, flip the sign in `basemap.js`'s `panBy` (`map.panBy([-dx, -dy])`): the contract in the Global Constraints is that a positive `dx` moves the view east, and this test is what decides which MapLibre convention that is.)

- [ ] **Step 4: Run it**

Run: `just e2e tests/map.spec.ts 2>&1 | tail -40`
Expected: all PASS. Investigate any failure before editing the test; "the streets lie on the basemap's roads" is the one that proves the projection against real tiles, and if it fails for a reason other than a bug in the code (for example, the road layers' ids in the style), say so in the report rather than loosening the threshold.

- [ ] **Step 5: The search spec and the other specs that named the old map**

`e2e/tests/search.spec.ts`: in the first `beforeEach` replace `await expect(page.locator("#map")).toBeVisible();` with `await expect(page.locator("#basemap canvas")).toBeVisible();` (these tests run on the sample city, which has the Places list and search but no map). Replace the whole `test.describe("the map under the floating panels", …)` with a version on the real area:

```ts
test.describe("the map under the floating panels", () => {
  test.use({ area: "world" });
  test.beforeEach(async ({ page }) => {
    await serveWorld(page);
    await page.goto("/map.html");
    await mapReady(page);
  });

  /** The box the places occupy on the screen, in page pixels. */
  const placesBox = (page: import("@playwright/test").Page) =>
    page.evaluate(() => {
      const map = (window as any).cityloomMap;
      const r = map.getCanvas().getBoundingClientRect();
      let l = Infinity, t = Infinity, rt = -Infinity, b = -Infinity;
      for (const f of map.querySourceFeatures("places")) {
        const pts = f.geometry.type === "Point" ? [f.geometry.coordinates] : f.geometry.coordinates;
        for (const c of pts) {
          const p = map.project(c);
          l = Math.min(l, r.left + p.x); rt = Math.max(rt, r.left + p.x);
          t = Math.min(t, r.top + p.y); b = Math.max(b, r.top + p.y);
        }
      }
      return { l, r: rt, t, b };
    });

  test("the whole city is fitted between the panels and under the bar, and centred there", async ({ page }) => {
    const left = (await page.locator(".panel.left").boundingBox())!;
    const right = (await page.locator(".panel.right").boundingBox())!;
    const bar = (await page.locator(".bar").boundingBox())!;
    await expect.poll(async () => (await placesBox(page)).l).toBeGreaterThan(left.x + left.width - 1);
    const c = await placesBox(page);
    expect(c.r).toBeLessThan(right.x + 1);
    expect(c.t).toBeGreaterThan(bar.y + bar.height - 1);
    const open = { l: left.x + left.width, r: right.x };
    expect(Math.abs((c.l + c.r) / 2 - (open.l + open.r) / 2)).toBeLessThan(60);
  });

  test("hiding the panels gives the map the room back and Whole city centres it again", async ({ page }) => {
    const before = await placesBox(page);
    await page.locator("#inspector-toggle").click();
    await page.locator("#notes-toggle").click();
    await page.locator("#zoom-fit").click();
    await expect.poll(async () => (await placesBox(page)).r - (await placesBox(page)).l).toBeGreaterThan(before.r - before.l);
    const after = await placesBox(page);
    expect(Math.abs((after.l + after.r) / 2 - page.viewportSize()!.width / 2)).toBeLessThan(60);
  });
});
```

(import `mapReady, serveWorld` at the top.) The test about streets being "named at the zoom that fits" described labels and widths of the SVG drawing and is gone; the basemap's own labels are the style's.

`e2e/tests/home.spec.ts`: replace the two `page.locator("#map-slot a[href^='street.html']")` with `page.locator("a.place-row[href^='street.html']")` (lines 79 and 124). `e2e/tests/world.spec.ts`: replace the three `#map-slot` locators the same way, and its own `beforeEach` with `serveWorld(page)` from the fixtures, deleting the now-duplicate `PLATEAU` constant. `shell.spec.ts`, `editors.spec.ts`, `junction.spec.ts` and `street.spec.ts` should pass unchanged; run them to see.

- [ ] **Step 6: Run the whole browser suite**

Run: `just e2e 2>&1 | tail -30`
Expected: all PASS. A failure in a spec this task did not touch is a real regression from earlier tasks: find it, do not skip it.

- [ ] **Step 7: Format and commit**

```bash
just format
git add e2e
git commit -m "test(e2e): the map page on a real area over the basemap

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 10: Documents

**Files:**
- Modify: `CLAUDE.md`, `web/credits.html`, `README.md` (the section Task 7 added is checked, not rewritten)

- [ ] **Step 1: `CLAUDE.md`**

Update the project instructions to what is now true: the `map/` bullet in Architecture (the map page draws no SVG; `projection.rs`, `overlay.rs`, `style.rs`; `svg.rs`/`camera.rs` remain for the home hero only; no `gestures.rs`), the ports list (add `Mapper`; "browser side wraps `web/basemap.js`"), the entry-point line (`mount_map(basemap)`; `web/map.js` creates the basemap with `createBasemap`), the commands (`just basemap-tiles`, `just serve` runs the Node server because PMTiles needs range requests, `just setup` runs `npm ci` at the root and in `e2e`), the OpenStreetMap paragraph (edges keep the road's centreline in `shape`; the layout remembers its `origin_m`; "Edges are drawn as straight lines…" now says only the editors and the hero still do), and a Conventions bullet: the e2e fixtures serve `e2e/fixtures/plateau.pmtiles` for every test, `mapReady(page)` waits for the places, and the map is MapLibre in a WebGL canvas (`window.cityloomMap`), so pointer behaviour is only covered there. State that the JS rule has one exception, `web/basemap.js`.

- [ ] **Step 2: `web/credits.html`**

Replace the last sentence of the "Streets" section and add a section after it:

```html
    <section aria-labelledby="c-map">
      <h2 id="c-map" class="note-h">The map</h2>
      <p>The basemap behind the city map is drawn from <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> data, © OpenStreetMap contributors (<a href="https://opendatacommons.org/licenses/odbl/">ODbL</a>), in the <a href="https://openmaptiles.org/">OpenMapTiles</a> schema (© OpenMapTiles, CC BY 4.0), made with <a href="https://github.com/onthegomap/planetiler">Planetiler</a> (Apache License 2.0) from a Geofabrik extract, with Natural Earth (public domain) and OpenStreetMap water polygons at low zooms.
      It is drawn with <a href="https://maplibre.org/">MapLibre GL JS</a> and read with <a href="https://github.com/protomaps/PMTiles">PMTiles</a>, both under the BSD 3-Clause license.
      Its labels are set in Noto Sans, copyright The Noto Project Authors, under the SIL Open Font License 1.1: <a href="glyphs/LICENSE-noto-sans.txt">read the license</a>.</p>
    </section>
```

- [ ] **Step 3: Verify and commit**

```bash
just format-check && just test && just e2e
git add CLAUDE.md web/credits.html README.md
git commit -m "docs: the map page on a basemap, in the project notes and the credits

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

Expected: all three commands succeed. Report the three results (test counts, e2e counts) with the output, not a summary.

---

## Self-Review

**Spec coverage.** Replaced map drawing → Tasks 5, 8. Self-hosted PMTiles, Planetiler → Tasks 6, 7. MapLibre vendored, one JS exception → Tasks 6, 8, 10. Rust-owned overlay (`overlay.rs`, `style.rs`) → Task 4, 5. Tiles required, missing/outside/no-roads messages → Tasks 5, 8, 9. Sample city removed from the map page → Task 5 (`NO_ROADS`), Task 9 (sample-city test). Print dropped on the map page → Task 8 (map.html). Actual geometry → Task 1. Georeferencing → Tasks 1 (origin), 2 (projection and the `geom` cross-check), 9 (alignment on real tiles). Home hero stays → Task 5 (`svg.rs`/`camera.rs` kept, only map-only functions removed). Glyphs from Noto Sans → Task 6. Range-capable server → Task 6. Legend replaced → Tasks 5, 8. Credits/README/CLAUDE.md → Tasks 7, 10. WebGL proof first → Task 9 Step 1 (it comes before the rest of that task; Task 8 Step 8 says to stop if WebGL fails).

**Known weak points the executor must watch (not placeholders; decisions to confirm against the running code):**
- `style::layers` uses `circle-radius` and `line-width` expressions with a zoom `interpolate` inside `max`/`*`. MapLibre allows a zoom expression only as the input of a top-level `interpolate`/`step` *or* inside other expressions as long as it is not nested under a data expression in a way the spec forbids. If MapLibre rejects `["max", n, ["*", ["get", …], ["interpolate", …["zoom"]…]]]`, restructure as `["interpolate", ["exponential", 2], ["zoom"], 0, ["max", n, ["*", ["get", p], k]], 24, ["max", n, ["*", ["get", p], k·2^24]]]`; the unit test in Task 4 reads the two stops and must be adjusted with it.
- The Planetiler flag spellings (`--osm-path`, `--output`, `--bounds`, `--minzoom`, `--maxzoom`, `--download`, `--force`, `--tmpdir`) were checked against its README, `quickstart.sh` and `PlanetilerConfig`/`Arguments` sources, but the script was not run (no extract and no Planetiler on the planning machine); Task 7 Step 2 is its first run.
- The OpenMapTiles layer and class names in `make_basemap_style.mjs` (`landcover`, `park`, `water`, `waterway`, `building`, `transportation`, `transportation_name`, `water_name`, `place`; classes `motorway`…`service`, `path`, `pedestrian`, `track`) are from the schema as documented; check them against the first tiles built (`pmtiles show` or the browser's layer inspector) before trusting the style.

**Type and name consistency.** `MapEvent` variants and fields (`Ready { bounds }`, `Failed`, `Pick { hot }`, `Hover { hot }`, `Moved`) are the same in Tasks 3, 5, 8 (the adapter's JSON `kind`s `ready|failed|pick|hover|moved` and field names `bounds`, `hot`). `Mapper` method names match across `ports.rs`, `FakeMapper`, `NoMapper`, `BrowserMapper` and the adapter (`setPlaces`↔`set_places`, `zoomBy`↔`zoom_by`, `panBy`↔`pan_by`, `setImperial`↔`set_imperial`, `highlight`, `fit`, `listen`). `BasemapState` (the view-model's enum) is distinct from the wasm extern type `Basemap`. `hot` ids are `s-<uid>` / `j-<uid>` in `overlay.rs`, `href_of`, the layers' `promoteId: "hot"`, and the existing `PlaceRow.hot`. Layer ids `places-casing`, `places-street`, `places-junction` match `style.rs` and `basemap.js`'s `hotAt`; basemap layer ids `road`, `road-casing`, `background` match `make_basemap_style.mjs` and Task 9.
