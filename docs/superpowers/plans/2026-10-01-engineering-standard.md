# Engineering standard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the city an engineering standard that sets the minimum and maximum width of every kind of street piece from the street's road type, design speed and the traffic volume of the mode that uses the piece, with a default per city and a choice of several.

**Architecture:** A new pure-Rust `standard` module holds the road types, speed bands, three synthetic built-in standards and the lookup. `Street` gains a `design` (road type, speed, four volumes); `Editor` takes the standard as a setting and reports an advisory "Lane widths" check plus per-piece ranges; `City` holds the chosen standard and saves it by id. The two pages only draw what the model returns and relay input.

**Tech Stack:** Rust 2024 (serde, serde_json, wasm-bindgen), compiled to WebAssembly; plain JavaScript modules and CSS in `web/`.

**Spec:** `docs/superpowers/specs/2026-10-01-engineering-standard-design.md`

## Global Constraints

- All lengths are integer millimetres. Speed is whole km/h, 5 to 130, and stays in km/h whatever the m/ft toggle says. Volumes are whole numbers, 0 to 20000, peak hour, both directions.
- The standard is **advisory**: a width outside it is accepted and reported as a failed check. No clamping of dragging or typing.
- `KINDS[k].min_mm` / `max_mm` are unchanged. The standard sits inside them: a built-in standard's range must lie inside its kind's envelope for every road type, speed band and volume band (a test enforces it).
- Every built-in standard value is a **synthetic placeholder**. Say so in the code comment and keep the pages' existing "Sample numbers for now, not real measurements." note.
- The rules live in Rust. The pages hold none.
- A save made before this change must still load with its streets and junctions intact. `SAVE_VERSION` is **not** bumped.
- The standard is saved by **id**, not index.
- The design (road type, speed, volumes) and the chosen standard are settings, not undo history. A street whose design differs from today's counts as edited on the map.
- Name collision: `junction::Profile` already exists, so the spec's "profile" is called `Design` in code (`Street.design`).
- Commit after each task, one commit each, conventional-commit style, ending with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Before any UI edit (Tasks 6 and 7) read `/Users/alex/.claude/plugins/cache/impeccable/impeccable/4.3.1/skills/impeccable/reference/craft-floor.md` and follow it. The UI follows `DESIGN.md` (drafting sheet, redline for failure with words and an icon, never colour alone).

**Environment:** `cargo` is not on `PATH` in this shell. In every command below, run first:

```sh
export PATH="/Users/alex/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
```

Baseline: `cargo test` passes 113 tests in the library.

## Review Focus

- A city saved before this change (no `design` on streets, no `standard`) loads with its edited streets and junctions intact and gets sample-default designs. (Task 4 test.)
- A speed or volume the resident types that is empty, negative, fractional or out of range keeps the last valid value and says so, instead of landing in a band. (Task 2 setter tests, Task 6 manual check.)
- A speed exactly on a band edge (30, 31, 50, 51, 70, 71 km/h) falls in the band the spec names. (Task 1 test.)
- A street read on the left-hand side of the road (`for_side`) or from its other end (`reversed`) keeps its design. (Task 2 test.)
- Switching the city's standard re-checks every street, and the untouched sample city passes under the default standard in either driving side. (Task 4 tests.)

---

### Task 1: The `standard` module

**Files:**
- Create: `src/standard.rs`
- Modify: `src/lib.rs` (add `pub mod standard;`)

**Interfaces:**
- Consumes: `catalogue::{KINDS, Mode, SAMPLES}`.
- Produces (later tasks rely on these exact names):
  - `pub struct RoadType { pub id: &'static str, pub name: &'static str }`, `pub const ROAD_TYPES: [RoadType; 4]`, `pub const LOCAL`, `COLLECTOR`, `ARTERIAL`, `FREEWAY: usize`.
  - `pub const SPEED_BANDS: usize = 4`, `MIN_SPEED_KMH = 5`, `MAX_SPEED_KMH = 130`, `MAX_VOLUME = 20_000`.
  - `pub const VOLUME_MODES: [Mode; 4]` (Foot, Bike, Transit, Vehicle), the order of `Design::volumes`.
  - `pub fn speed_band(speed_kmh: i32) -> usize`.
  - `pub struct Design { pub road_type: usize, pub speed_kmh: i32, pub volumes: [i32; 4] }` (`Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize`) with `Design::for_sample(sample: usize) -> Design` and `Design::is_sound(&self) -> bool`.
  - `pub struct Standard` (`Clone, Copy, Debug`) with `raw_limits(&self, d: &Design, kind: usize) -> (i32, i32)` and `limits(&self, d: &Design, kind: usize) -> (i32, i32)`.
  - `pub static STANDARDS: [Standard; 3]` in the order compact, balanced, generous; `pub const BALANCED: usize = 1`.
  - `pub struct StandardInfo { id, name, note }` (Serialize), `pub fn info() -> Vec<StandardInfo>`, `pub fn catalogue() -> serde_json::Value`.

- [ ] **Step 1: Register the module and write the failing tests**

In `src/lib.rs`, add `pub mod standard;` after `pub mod plan;` (keep the list alphabetical: `plan`, `standard`, `street_measures`).

Create `src/standard.rs` containing only the test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::{kind_index, SAMPLES};

    fn kind(id: &str) -> usize {
        kind_index(id).unwrap()
    }

    fn design(road_type: usize, speed_kmh: i32, volumes: [i32; 4]) -> Design {
        Design { road_type, speed_kmh, volumes }
    }

    #[test]
    fn the_standards_assume_this_order_of_kinds() {
        let ids: Vec<&str> = KINDS.iter().map(|k| k.id).collect();
        assert_eq!(ids, ["sidewalk", "planting", "bike", "travel", "bus", "parking", "median", "loading", "shoulder"]);
    }

    #[test]
    fn a_speed_falls_in_exactly_one_band() {
        let bands: Vec<usize> = [5, 30, 31, 50, 51, 70, 71, 130].iter().map(|&s| speed_band(s)).collect();
        assert_eq!(bands, [0, 0, 1, 1, 2, 2, 3, 3]);
    }

    #[test]
    fn a_volume_falls_in_low_typical_or_high() {
        let rule = VolumeRule { low_below: 200, high_above: 800, steps: [(0, 0); 3] };
        let bands: Vec<usize> = [0, 199, 200, 800, 801, 20_000].iter().map(|&v| volume_band(v, &rule)).collect();
        assert_eq!(bands, [0, 0, 1, 1, 2, 2]);
    }

    #[test]
    fn a_busier_sidewalk_wants_more() {
        let s = &STANDARDS[BALANCED];
        let walk = |foot| s.limits(&design(COLLECTOR, 40, [foot, 60, 6, 600]), kind("sidewalk"));
        assert_eq!(walk(100), (1400, 3500));
        assert_eq!(walk(500), (2000, 4500));
        assert_eq!(walk(1000), (2500, 5500));
    }

    #[test]
    fn road_type_and_speed_move_the_driving_lane() {
        let s = &STANDARDS[BALANCED];
        let travel = |road, speed| s.limits(&design(road, speed, [300, 60, 6, 600]), kind("travel"));
        assert_eq!(travel(COLLECTOR, 40), (3100, 3600));
        assert_eq!(travel(LOCAL, 30), (3000, 3500));
        assert_eq!(travel(ARTERIAL, 60), (3300, 3800));
        assert_eq!(travel(FREEWAY, 100), (3400, 3900));
    }

    #[test]
    fn a_piece_with_no_traffic_ignores_volume_speed_and_road() {
        let s = &STANDARDS[BALANCED];
        let a = s.limits(&design(LOCAL, 30, [0, 0, 0, 0]), kind("planting"));
        let b = s.limits(&design(FREEWAY, 130, [20_000; 4]), kind("planting"));
        assert_eq!(a, b);
        assert_eq!(a, (900, 3000));
    }

    #[test]
    fn the_standards_differ() {
        let d = design(COLLECTOR, 40, [300, 60, 6, 600]);
        for k in 0..KINDS.len() {
            let ranges: Vec<_> = STANDARDS.iter().map(|s| s.limits(&d, k)).collect();
            assert!(ranges[0] != ranges[1] || ranges[1] != ranges[2], "{}: {ranges:?}", KINDS[k].id);
        }
    }

    /// Every built-in standard, for every road type, speed band, volume band
    /// and kind, gives a range that is not crossed and lies inside the
    /// catalogue's: nothing relies on `limits` clamping it.
    #[test]
    fn every_standard_stays_inside_the_catalogue() {
        for s in &STANDARDS {
            for road_type in 0..ROAD_TYPES.len() {
                for speed in [5, 30, 31, 50, 51, 70, 71, 130] {
                    for pick in 0..81usize {
                        let volumes: [i32; 4] = std::array::from_fn(|m| {
                            let r = &s.volume[m];
                            [r.low_below - 1, r.low_below, r.high_above + 1][(pick / 3usize.pow(m as u32)) % 3]
                        });
                        let d = design(road_type, speed, volumes);
                        for (k, kind) in KINDS.iter().enumerate() {
                            let (lo, hi) = s.raw_limits(&d, k);
                            assert!(
                                kind.min_mm <= lo && lo <= hi && hi <= kind.max_mm,
                                "{} {} road {road_type} {speed} km/h {volumes:?}: {lo} to {hi}",
                                s.id,
                                kind.id
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_crossed_range_is_held_inside_the_catalogue() {
        let mut s = STANDARDS[BALANCED];
        s.anchor[kind("travel")] = (3900, 3950);
        // At 130 km/h on a freeway the steps push it past the catalogue's 4000.
        let d = design(FREEWAY, 130, [300, 60, 6, 600]);
        assert!(s.raw_limits(&d, kind("travel")).0 > 4000);
        assert_eq!(s.limits(&d, kind("travel")), (4000, 4000));
    }

    #[test]
    fn the_balanced_standard_suits_every_sample_street() {
        for (i, sample) in SAMPLES.iter().enumerate() {
            let d = Design::for_sample(i);
            assert!(d.is_sound(), "{}", sample.name);
            for (id, width) in sample.segments {
                let (lo, hi) = STANDARDS[BALANCED].limits(&d, kind(id));
                assert!((lo..=hi).contains(width), "{} {id} {width} not in {lo} to {hi}", sample.name);
            }
        }
    }

    #[test]
    fn a_design_with_a_bad_value_is_not_sound() {
        let ok = Design::for_sample(0);
        assert!(ok.is_sound());
        assert!(!Design { road_type: 4, ..ok }.is_sound());
        assert!(!Design { speed_kmh: 4, ..ok }.is_sound());
        assert!(!Design { speed_kmh: 131, ..ok }.is_sound());
        assert!(!Design { volumes: [0, 0, 0, -1], ..ok }.is_sound());
        assert!(!Design { volumes: [20_001, 0, 0, 0], ..ok }.is_sound());
    }

    #[test]
    fn the_sample_streets_start_as_the_kind_of_road_they_are() {
        assert_eq!(Design::for_sample(0).road_type, COLLECTOR);
        assert_eq!(Design::for_sample(1).road_type, ARTERIAL);
        assert_eq!(Design::for_sample(2).road_type, LOCAL);
        assert_eq!(Design::for_sample(3).road_type, FREEWAY);
        assert_eq!(Design::for_sample(99), Design::for_sample(3));
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test standard 2>&1 | tail -20`
Expected: compile errors such as "cannot find type `Design` in this scope".

- [ ] **Step 3: Write the module above the tests**

Put this at the top of `src/standard.rs`, above the `#[cfg(test)]` block:

```rust
//! The engineering standard: how wide each kind of street piece may be.
//!
//! A standard gives every kind a range of widths. The range depends on the
//! street's road type, its design speed and the volume of the traffic of the
//! mode that uses the piece (a driving lane looks at vehicles, a sidewalk at
//! people walking). It is advice: a width outside it is reported, not refused.
//! The catalogue's own limits (`KINDS[k].min_mm` and `max_mm`) still hold and
//! the standard lies inside them.
//!
//! Every number here is a synthetic placeholder, like the rest of the
//! catalogue. None of it is any jurisdiction's standard.

use serde::{Deserialize, Serialize};

use crate::catalogue::{KINDS, Mode, SAMPLES};

#[derive(Clone, Copy, Debug, Serialize)]
pub struct RoadType {
    pub id: &'static str,
    pub name: &'static str,
}

pub const ROAD_TYPES: [RoadType; 4] = [
    RoadType { id: "local", name: "Local street" },
    RoadType { id: "collector", name: "Collector" },
    RoadType { id: "arterial", name: "Arterial" },
    RoadType { id: "freeway", name: "Freeway" },
];

pub const LOCAL: usize = 0;
pub const COLLECTOR: usize = 1;
pub const ARTERIAL: usize = 2;
pub const FREEWAY: usize = 3;

/// The speed bands are up to 30 km/h, 31 to 50, 51 to 70 and over 70.
const BAND_TOPS_KMH: [i32; 3] = [30, 50, 70];
pub const SPEED_BANDS: usize = 4;
pub const MIN_SPEED_KMH: i32 = 5;
pub const MAX_SPEED_KMH: i32 = 130;
pub const MAX_VOLUME: i32 = 20_000;

/// The modes a volume is kept for, in the order of `Design::volumes`.
pub const VOLUME_MODES: [Mode; 4] = [Mode::Foot, Mode::Bike, Mode::Transit, Mode::Vehicle];

pub fn speed_band(speed_kmh: i32) -> usize {
    BAND_TOPS_KMH.iter().position(|&top| speed_kmh <= top).unwrap_or(SPEED_BANDS - 1)
}

/// What a street is, as far as the standard is concerned: its road type, its
/// design speed and the traffic of each mode in the busiest hour, both
/// directions (people walking, people biking, buses, vehicles).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Design {
    /// Index into `ROAD_TYPES`.
    pub road_type: usize,
    pub speed_kmh: i32,
    /// Per hour, indexed like `VOLUME_MODES`.
    pub volumes: [i32; 4],
}

/// What each sample street starts as, in the order of `SAMPLES`.
const SAMPLE_DESIGNS: [Design; 4] = [
    Design { road_type: COLLECTOR, speed_kmh: 40, volumes: [300, 60, 6, 600] },
    Design { road_type: ARTERIAL, speed_kmh: 50, volumes: [600, 100, 20, 1500] },
    Design { road_type: LOCAL, speed_kmh: 30, volumes: [100, 20, 0, 150] },
    Design { road_type: FREEWAY, speed_kmh: 100, volumes: [0, 0, 0, 4000] },
];
const _: () = assert!(SAMPLE_DESIGNS.len() == SAMPLES.len());

impl Design {
    pub fn for_sample(sample: usize) -> Design {
        SAMPLE_DESIGNS[sample.min(SAMPLE_DESIGNS.len() - 1)]
    }

    /// Whether every value is one the standard can read.
    pub fn is_sound(&self) -> bool {
        self.road_type < ROAD_TYPES.len()
            && (MIN_SPEED_KMH..=MAX_SPEED_KMH).contains(&self.speed_kmh)
            && self.volumes.iter().all(|v| (0..=MAX_VOLUME).contains(v))
    }
}

/// How one mode's volume moves the range of the kinds that mode uses.
#[derive(Clone, Copy, Debug)]
pub struct VolumeRule {
    /// A volume below this is low; above `high_above` it is high; else typical.
    pub low_below: i32,
    pub high_above: i32,
    /// Added to the `(min, max)` of each of the mode's kinds, for low, typical and high.
    pub steps: [(i32, i32); 3],
}

fn volume_band(volume: i32, rule: &VolumeRule) -> usize {
    if volume < rule.low_below {
        0
    } else if volume > rule.high_above {
        2
    } else {
        1
    }
}

/// The kinds that follow the speed of the street.
fn follows_speed(id: &str) -> bool {
    matches!(id, "travel" | "bus" | "shoulder" | "bike")
}

/// The kinds that follow the road type.
fn follows_road(id: &str) -> bool {
    matches!(id, "travel" | "bus" | "shoulder")
}

/// A standard. Read it as a table with a row for each road type and speed
/// band, each row giving a `(min, max)` width for every kind, then adjusted by
/// the volume of the mode that uses the kind. The rows are built from the
/// pieces below so a standard stays short to write and read.
#[derive(Clone, Copy, Debug)]
pub struct Standard {
    pub id: &'static str,
    pub name: &'static str,
    pub note: &'static str,
    /// `(min, max)` of each kind, in the order of `KINDS`, on a collector at
    /// 31 to 50 km/h.
    pub anchor: [(i32, i32); KINDS.len()],
    /// Added, for each speed band, to the kinds that follow speed.
    pub speed: [(i32, i32); SPEED_BANDS],
    /// Added, for each road type, to the kinds that follow the road.
    pub road: [(i32, i32); ROAD_TYPES.len()],
    /// One rule for each mode in `VOLUME_MODES`.
    pub volume: [VolumeRule; VOLUME_MODES.len()],
}

impl Standard {
    /// The range for `kind` on a street of this design, before it is held to
    /// the catalogue's own range.
    pub fn raw_limits(&self, d: &Design, kind: usize) -> (i32, i32) {
        let id = KINDS[kind].id;
        let (mut lo, mut hi) = self.anchor[kind];
        if follows_speed(id) {
            let (a, b) = self.speed[speed_band(d.speed_kmh)];
            (lo, hi) = (lo + a, hi + b);
        }
        if follows_road(id) {
            let (a, b) = self.road[d.road_type.min(ROAD_TYPES.len() - 1)];
            (lo, hi) = (lo + a, hi + b);
        }
        if let Some(m) = VOLUME_MODES.iter().position(|&m| m == KINDS[kind].mode) {
            let rule = &self.volume[m];
            let (a, b) = rule.steps[volume_band(d.volumes[m], rule)];
            (lo, hi) = (lo + a, hi + b);
        }
        (lo, hi)
    }

    /// The range for `kind` on a street of this design: the standard's, held
    /// inside the catalogue's, and never crossed.
    pub fn limits(&self, d: &Design, kind: usize) -> (i32, i32) {
        let (lo, hi) = self.raw_limits(d, kind);
        let k = &KINDS[kind];
        let lo = lo.clamp(k.min_mm, k.max_mm);
        (lo, hi.clamp(k.min_mm, k.max_mm).max(lo))
    }
}

// Kinds, in order: sidewalk, planting, bike, travel, bus, parking, median,
// loading, shoulder. Modes, in order: walking, biking, buses, vehicles.
pub static STANDARDS: [Standard; 3] = [
    Standard {
        id: "compact",
        name: "Compact",
        note: "Narrower lanes and sidewalks, to fit more in a tight street.",
        anchor: [(1800, 4000), (800, 2500), (1500, 2300), (3000, 3400), (3300, 3600), (2100, 2500), (900, 3000), (2100, 2700), (2000, 3000)],
        speed: [(-100, -100), (0, 0), (100, 100), (200, 200)],
        road: [(0, 0), (0, 0), (100, 100), (100, 100)],
        volume: [
            VolumeRule { low_below: 200, high_above: 800, steps: [(-400, -800), (0, 0), (400, 800)] },
            VolumeRule { low_below: 50, high_above: 300, steps: [(-200, 0), (0, 0), (200, 200)] },
            VolumeRule { low_below: 10, high_above: 40, steps: [(0, 0), (0, 0), (100, 0)] },
            VolumeRule { low_below: 400, high_above: 2000, steps: [(0, 0), (0, 0), (100, 100)] },
        ],
    },
    Standard {
        id: "balanced",
        name: "Balanced",
        note: "Between the other two. The city starts on this one.",
        anchor: [(2000, 4500), (900, 3000), (1700, 2400), (3100, 3600), (3300, 3700), (2200, 2700), (1000, 4000), (2400, 3000), (2400, 3200)],
        speed: [(-100, -100), (0, 0), (100, 100), (200, 200)],
        road: [(0, 0), (0, 0), (100, 100), (100, 100)],
        volume: [
            VolumeRule { low_below: 200, high_above: 800, steps: [(-600, -1000), (0, 0), (500, 1000)] },
            VolumeRule { low_below: 50, high_above: 300, steps: [(-200, 0), (0, 0), (200, 200)] },
            VolumeRule { low_below: 10, high_above: 40, steps: [(0, 0), (0, 0), (100, 0)] },
            VolumeRule { low_below: 400, high_above: 2000, steps: [(0, 0), (0, 0), (100, 100)] },
        ],
    },
    Standard {
        id: "generous",
        name: "Generous",
        note: "Wider lanes and sidewalks, for comfort over capacity.",
        anchor: [(2400, 5000), (1200, 3500), (1800, 2600), (3200, 3700), (3400, 3800), (2400, 2800), (1500, 4500), (2600, 3200), (2600, 3300)],
        speed: [(-100, -100), (0, 0), (100, 100), (100, 100)],
        road: [(0, 0), (0, 0), (0, 0), (100, 100)],
        volume: [
            VolumeRule { low_below: 200, high_above: 800, steps: [(-600, -1000), (0, 0), (500, 1000)] },
            VolumeRule { low_below: 50, high_above: 300, steps: [(-200, 0), (0, 0), (200, 200)] },
            VolumeRule { low_below: 10, high_above: 40, steps: [(0, 0), (0, 0), (100, 0)] },
            VolumeRule { low_below: 400, high_above: 2000, steps: [(0, 0), (0, 0), (0, 0)] },
        ],
    },
];

/// The standard a city starts on unless it names another.
pub const BALANCED: usize = 1;

#[derive(Serialize)]
pub struct StandardInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub note: &'static str,
}

pub fn info() -> Vec<StandardInfo> {
    STANDARDS.iter().map(|s| StandardInfo { id: s.id, name: s.name, note: s.note }).collect()
}

/// What the pages need to draw the controls, as JSON: the road types, the
/// bounds of the numbers, the volumes asked for and the standards on offer.
pub fn catalogue() -> serde_json::Value {
    let labels = ["People walking", "People biking", "Buses", "Cars and trucks"];
    serde_json::json!({
        "road_types": ROAD_TYPES,
        "speed_kmh": { "min": MIN_SPEED_KMH, "max": MAX_SPEED_KMH },
        "volume_max": MAX_VOLUME,
        "volumes": labels,
        "standards": info(),
    })
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test standard 2>&1 | tail -20`
Expected: all `standard::tests::*` pass (12 tests). If `every_standard_stays_inside_the_catalogue` fails, a number above is wrong: fix the number, not the test.

- [ ] **Step 5: Run the whole suite**

Run: `cargo test 2>&1 | grep "test result"`
Expected: `test result: ok. 125 passed` (113 plus 12) and no warnings about unused items other than ones Task 2 will use. Unused-code warnings for `Design`, `info` etc. are fine at this stage.

- [ ] **Step 6: Commit**

```bash
git add src/standard.rs src/lib.rs
git commit -m "$(cat <<'EOF'
feat: an engineering standard that gives each kind of piece a width range by road type, speed and volume

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: A street carries its design

**Files:**
- Modify: `src/model.rs` (imports; `Street`; `Street::is_sound`; `Editor` struct, `new`, `load_sample`, `from_street`, `snapshot`; new setters; tests)

**Interfaces:**
- Consumes: Task 1's `Design`, `STANDARDS`, `BALANCED`, `VOLUME_MODES`.
- Produces:
  - `Street.design: Design`.
  - `Editor::set_road_type(&mut self, road_type: usize) -> bool`
  - `Editor::set_speed(&mut self, speed_kmh: i32) -> bool`
  - `Editor::set_volume(&mut self, mode: usize, volume: i32) -> bool` (`mode` indexes `VOLUME_MODES`)
  - `Editor::set_standard(&mut self, standard: usize) -> bool`
  - private `Editor::standard(&self) -> &'static Standard` and fields `design: Design`, `standard: usize`.

- [ ] **Step 1: Write the failing tests**

In `src/model.rs`, at the end of `mod tests` (before its closing `}`), add:

```rust
    #[test]
    fn a_street_starts_with_the_design_of_its_sample() {
        for i in 0..SAMPLES.len() {
            assert_eq!(Street::sample(i, Side::Right).design, Design::for_sample(i));
        }
    }

    #[test]
    fn a_street_saved_without_a_design_loads_with_its_samples() {
        let street = Street::sample(1, Side::Right);
        let mut saved = serde_json::to_value(&street).unwrap();
        assert!(saved.as_object_mut().unwrap().remove("design").is_some());
        let loaded: Street = serde_json::from_value(saved).unwrap();
        assert_eq!(loaded, street);
        // One that has a design keeps it.
        let mut own = street.clone();
        own.design.speed_kmh = 30;
        assert_eq!(serde_json::from_str::<Street>(&serde_json::to_string(&own).unwrap()).unwrap(), own);
    }

    #[test]
    fn the_design_is_a_setting_and_not_history() {
        let mut e = Editor::new(0);
        assert!(e.set_speed(50));
        assert!(e.set_road_type(standard::ARTERIAL));
        assert!(e.set_volume(3, 700));
        assert!(!e.view().can_undo);
        assert!(e.view().revisions.is_empty());
        let d = e.snapshot().design;
        assert_eq!((d.speed_kmh, d.road_type, d.volumes[3]), (50, standard::ARTERIAL, 700));
        // Undo and start over leave it alone.
        e.remove(e.current()[1].uid);
        assert!(e.undo());
        assert_eq!(e.snapshot().design, d);
    }

    #[test]
    fn a_design_value_that_is_unchanged_or_out_of_range_is_refused() {
        let mut e = Editor::new(0);
        let before = e.snapshot().design;
        assert!(!e.set_speed(40), "unchanged");
        assert!(!e.set_speed(0));
        assert!(!e.set_speed(4));
        assert!(!e.set_speed(131));
        assert!(!e.set_road_type(1), "unchanged");
        assert!(!e.set_road_type(4));
        assert!(!e.set_volume(3, 600), "unchanged");
        assert!(!e.set_volume(3, -1));
        assert!(!e.set_volume(3, 20_001));
        assert!(!e.set_volume(4, 10), "no such mode");
        assert_eq!(e.snapshot().design, before);
    }

    #[test]
    fn the_design_goes_with_the_street_to_the_other_side_and_the_other_end() {
        let mut street = Street::sample(1, Side::Right);
        street.design.speed_kmh = 60;
        assert_eq!(street.for_side(Side::Left).design, street.design);
        assert_eq!(street.reversed().design, street.design);
        let e = Editor::from_street(&street, &street, 3);
        assert_eq!(e.snapshot().design, street.design);
    }

    #[test]
    fn a_street_with_a_bad_design_is_not_sound() {
        let mut s = Street::sample(0, Side::Right);
        assert!(s.is_sound());
        s.design.speed_kmh = 500;
        assert!(!s.is_sound());
    }

    #[test]
    fn loading_a_sample_loads_its_design() {
        let mut e = Editor::new(0);
        assert!(e.set_speed(70));
        e.load_sample(1);
        assert_eq!(e.snapshot().design, Design::for_sample(1));
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `cargo test model::tests 2>&1 | tail -20`
Expected: compile errors (`no field design`, `no method set_speed`).

- [ ] **Step 3: Implement**

1. Imports. In `src/model.rs`, below `use crate::street_measures;` add:

```rust
use crate::standard::{self, Design, STANDARDS, Standard, VOLUME_MODES};
```

2. `Street`. Replace the `Street` struct (the derive line through the closing brace) with:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "SavedStreet")]
pub struct Street {
    /// Index into `SAMPLES`: the kind of street it began as, which names it.
    pub sample: usize,
    pub row_mm: i32,
    pub side: Side,
    pub segments: Vec<Segment>,
    pub next_uid: u32,
    /// The road type, speed and traffic the engineering standard reads.
    pub design: Design,
}

/// A street as a save wrote it. Saves from before streets had a design have
/// none, and read the design their sample starts with.
#[derive(Deserialize)]
struct SavedStreet {
    sample: usize,
    row_mm: i32,
    side: Side,
    segments: Vec<Segment>,
    next_uid: u32,
    #[serde(default)]
    design: Option<Design>,
}

impl From<SavedStreet> for Street {
    fn from(s: SavedStreet) -> Street {
        let design = s.design.unwrap_or_else(|| Design::for_sample(s.sample));
        Street { sample: s.sample, row_mm: s.row_mm, side: s.side, segments: s.segments, next_uid: s.next_uid, design }
    }
}
```

3. `Street::is_sound`: change `self.sample < SAMPLES.len()` to `self.sample < SAMPLES.len()\n            && self.design.is_sound()`.

4. `Editor` struct: after the `region` field and before `time_min`, add:

```rust
    /// The road type, speed and traffic of the street. Like the region, a
    /// setting of the sheet and not part of the history.
    design: Design,
    /// Index into `STANDARDS`: the standard the street is checked against. Also a setting.
    standard: usize,
```

5. `Editor::new`: in the struct literal, after `region: 0,` add `design: Design::for_sample(0),` and `standard: standard::BALANCED,`.

6. `load_sample`: after `self.sample = sample;` add `self.design = Design::for_sample(sample);`.

7. `from_street`: in the literal, after `region,` add `design: now.design,` and `standard: standard::BALANCED,`.

8. `snapshot`: after `next_uid: self.next_uid,` add `design: self.design,`.

9. Add the setters inside `impl Editor`, directly after `snapshot`:

```rust
    // ---- the street's design: settings, not history -----------------------

    pub fn set_road_type(&mut self, road_type: usize) -> bool {
        self.set_design(Design { road_type, ..self.design })
    }

    pub fn set_speed(&mut self, speed_kmh: i32) -> bool {
        self.set_design(Design { speed_kmh, ..self.design })
    }

    /// `mode` indexes `VOLUME_MODES`: walking, biking, buses, vehicles.
    pub fn set_volume(&mut self, mode: usize, volume: i32) -> bool {
        if mode >= VOLUME_MODES.len() {
            return false;
        }
        let mut volumes = self.design.volumes;
        volumes[mode] = volume;
        self.set_design(Design { volumes, ..self.design })
    }

    /// Takes a design that is sound and different; says whether it did.
    fn set_design(&mut self, design: Design) -> bool {
        if design == self.design || !design.is_sound() {
            return false;
        }
        self.design = design;
        true
    }

    /// Chooses the standard the street is checked against.
    pub fn set_standard(&mut self, standard: usize) -> bool {
        if standard >= STANDARDS.len() || standard == self.standard {
            return false;
        }
        self.standard = standard;
        true
    }

    fn standard(&self) -> &'static Standard {
        &STANDARDS[self.standard]
    }
```

`standard()` and `Standard` are used in Task 3; if the compiler warns "never used" now, that is expected.

- [ ] **Step 4: Run to see the tests pass**

Run: `cargo test 2>&1 | grep -E "test result|FAILED|panicked|error"`
Expected: `ok. 131 passed` (125 plus 6) and no failures. Existing city tests that compare `back.streets` with `city.streets` still pass because the design round-trips.

- [ ] **Step 5: Commit**

```bash
git add src/model.rs
git commit -m "$(cat <<'EOF'
feat: a street holds its road type, design speed and traffic volumes

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: The widths check, the pieces' ranges and the transit minimum

**Files:**
- Modify: `src/model.rs` (`checks`, new `lane_widths`, `Check`, `SegView`, `View`, `view`, `seg_views`, `measure_views`; tests)
- Modify: `src/street_measures.rs` (`detect` takes the bus minimum; remove `MIN_TRANSIT_LANE_MM`)

**Interfaces:**
- Consumes: Task 2's `Editor::standard()`, `design`; Task 1's `Standard::limits`.
- Produces:
  - A fourth-position-later check `id: "widths"`, label `"Lane widths within the standard"`, appended **last** so existing check indices do not move.
  - `pub struct OutOfRange { pub kind: usize, pub width_mm: i32, pub min_mm: i32, pub max_mm: i32 }` (Serialize) and `Check.out_of_range: Vec<OutOfRange>` (skipped in JSON when empty), worst first.
  - `SegView.std_min_mm`, `std_max_mm: i32`, `std_ok: bool` (for the type the piece is at the shown time).
  - `View.design: Design`, `View.standard: usize`, `View.std_limits: Vec<[i32; 2]>` (indexed by kind).
  - `street_measures::detect(raw, now, side, freeway, bus_min_mm: i32)`.

- [ ] **Step 1: Write the failing tests**

At the end of `mod tests` in `src/model.rs` add:

```rust
    #[test]
    fn the_default_standard_accepts_every_sample_street() {
        for i in 0..SAMPLES.len() {
            let v = Editor::new(i).view();
            let c = v.checks.iter().find(|c| c.id == "widths").unwrap();
            assert!(c.ok, "{}: {}", SAMPLES[i].name, c.detail);
            assert!(v.segments.iter().all(|s| s.std_ok), "{}", SAMPLES[i].name);
        }
    }

    #[test]
    fn a_lane_outside_the_standard_fails_the_check_and_says_which() {
        let mut e = Editor::new(0);
        let lane = e.view().segments.iter().find(|s| s.kind == kind("travel")).unwrap().uid;
        assert!(e.set_width(lane, 3800));
        let v = e.view();
        let c = v.checks.iter().find(|c| c.id == "widths").unwrap();
        assert!(!c.ok);
        assert_eq!(c.amount_mm, 200);
        assert_eq!(c.detail, "Driving lane 3800 mm, standard allows 3100 to 3600 mm");
        let o = &c.out_of_range[0];
        assert_eq!((o.kind, o.width_mm, o.min_mm, o.max_mm), (kind("travel"), 3800, 3100, 3600));
        let s = v.segments.iter().find(|s| s.uid == lane).unwrap();
        assert!(!s.std_ok);
        assert_eq!((s.std_min_mm, s.std_max_mm), (3100, 3600));
        // The widths check comes last, so the others keep their places.
        assert_eq!(v.checks.last().unwrap().id, "widths");
        assert_eq!(v.checks[1].id, "edges");
    }

    #[test]
    fn the_check_names_the_worst_lane_first_and_counts_the_rest() {
        let mut e = Editor::new(0);
        let lanes: Vec<u32> = e.view().segments.iter().filter(|s| s.kind == kind("travel")).map(|s| s.uid).collect();
        assert!(e.set_width(lanes[0], 3700));
        assert!(e.set_width(lanes[1], 3800));
        let c = e.view().checks.into_iter().find(|c| c.id == "widths").unwrap();
        assert_eq!(c.out_of_range.iter().map(|o| o.width_mm).collect::<Vec<_>>(), [3800, 3700]);
        assert!(c.detail.starts_with("Driving lane 3800 mm") && c.detail.ends_with("and 1 more"), "{}", c.detail);
    }

    #[test]
    fn a_busier_street_changes_the_range_a_piece_is_held_to() {
        let mut e = Editor::new(0);
        let walk = |e: &Editor| e.view().segments[0].std_min_mm;
        assert_eq!(walk(&e), 2000);
        assert!(e.set_volume(0, 1000));
        assert_eq!(walk(&e), 2500);
        assert!(e.set_volume(0, 100));
        assert_eq!(walk(&e), 1400);
        // A table of the range for every kind is there for the page.
        let v = e.view();
        assert_eq!(v.std_limits.len(), KINDS.len());
        assert_eq!(v.std_limits[kind("sidewalk")], [1400, 3500]);
    }

    #[test]
    fn the_standard_is_a_setting_the_view_reports() {
        let mut e = Editor::new(0);
        assert_eq!(e.view().standard, standard::BALANCED);
        let before = e.view().std_limits[kind("sidewalk")];
        assert!(e.set_standard(2));
        let v = e.view();
        assert_eq!(v.standard, 2);
        assert_ne!(v.std_limits[kind("sidewalk")], before);
        assert!(!v.can_undo);
        assert!(!e.set_standard(2), "unchanged");
        assert!(!e.set_standard(3), "no such standard");
    }

    #[test]
    fn the_transit_minimum_comes_from_the_standard() {
        let mut e = Editor::new(1);
        assert!(e.apply_measure("B1"));
        let bus = e.view().segments.iter().find(|s| s.kind == kind("bus")).unwrap().uid;
        assert!(e.set_road_type(standard::COLLECTOR));
        assert!(e.set_width(bus, 3300));
        let problems = |e: &Editor| e.view().measures.into_iter().find(|m| m.code == "B1").unwrap().problems;
        assert!(problems(&e).is_empty(), "{:?}", problems(&e));
        // An arterial's bus lane is held to 100 mm more.
        assert!(e.set_road_type(standard::ARTERIAL));
        assert_eq!(problems(&e), ["A bus lane is 3300 mm wide, and a transit lane wants 3400 mm"]);
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `cargo test model::tests 2>&1 | tail -20`
Expected: compile errors (`no field std_ok`, `no field standard` on `View`).

- [ ] **Step 3: Implement**

1. **`street_measures.rs`.** Delete the `MIN_TRANSIT_LANE_MM` constant and its doc comment (`/// A transit lane should be at least this wide (synthetic).`). Change the signature of `detect`:

```rust
pub fn detect(raw: &[Segment], now: &[Segment], side: Side, freeway: bool, bus_min_mm: i32) -> Vec<Found> {
```

and the check in it:

```rust
    for &i in &bus {
        if road[i].width_mm < bus_min_mm {
            base.push(format!("A bus lane is {} mm wide, and a transit lane wants {} mm", road[i].width_mm, bus_min_mm));
        }
    }
```

Also update the doc comment above `detect` by adding one sentence: `` `bus_min_mm` is the narrowest bus lane the street's standard allows. ``

2. **`model.rs`, `Check`.** Replace the struct with:

```rust
#[derive(Serialize)]
pub struct Check {
    pub id: &'static str,
    pub ok: bool,
    /// The length the detail speaks of, so the page can print it in its units.
    pub amount_mm: i32,
    pub label: &'static str,
    pub detail: String,
    /// For the widths check: each piece outside the standard's range, the
    /// furthest out first, so the page can print it in its units.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub out_of_range: Vec<OutOfRange>,
}

#[derive(Serialize)]
pub struct OutOfRange {
    pub kind: usize,
    pub width_mm: i32,
    pub min_mm: i32,
    pub max_mm: i32,
}
```

In `fn checks`, add `out_of_range: Vec::new(),` after the `detail:` field of each of the four existing `Check { ... }` literals (`fits`, `edges`, `access`, `side`). Find them with `grep -n "Check {" src/model.rs`.

3. **`checks` signature and tail.** Change the signature to

```rust
fn checks(segs: &[Segment], row_mm: i32, side: Side, freeway: bool, standard: &Standard, design: &Design) -> Vec<Check> {
```

Change the line `    vec![` that starts the returned list (just after `let side_name = ...;`) to `    let mut list = vec![`, and replace the list's closing `    ]\n}` with:

```rust
    ];
    list.push(lane_widths(segs, standard, design));
    list
}
```

Add this function directly below `checks`:

```rust
/// Whether every piece is inside the standard's range for its kind on this
/// street. Advisory: it reports, it does not refuse.
fn lane_widths(now: &[Segment], standard: &Standard, design: &Design) -> Check {
    let mut out: Vec<(i32, OutOfRange)> = now
        .iter()
        .filter_map(|s| {
            let (min_mm, max_mm) = standard.limits(design, s.kind);
            let away = (min_mm - s.width_mm).max(s.width_mm - max_mm);
            (away > 0).then_some((away, OutOfRange { kind: s.kind, width_mm: s.width_mm, min_mm, max_mm }))
        })
        .collect();
    out.sort_by_key(|(away, _)| std::cmp::Reverse(*away));
    let amount_mm = out.first().map_or(0, |(away, _)| *away);
    let out: Vec<OutOfRange> = out.into_iter().map(|(_, o)| o).collect();
    let detail = match out.split_first() {
        None => "Every piece is inside the standard's range".to_string(),
        Some((w, rest)) => {
            let first = format!("{} {} mm, standard allows {} to {} mm", KINDS[w.kind].name, w.width_mm, w.min_mm, w.max_mm);
            if rest.is_empty() { first } else { format!("{first} and {} more", rest.len()) }
        }
    };
    Check { id: "widths", ok: out.is_empty(), amount_mm, label: "Lane widths within the standard", detail, out_of_range: out }
}
```

4. **`SegView`.** After `pub max_mm: i32,` add:

```rust
    /// The standard's range for this piece's type at the shown time, and
    /// whether its width is inside it.
    pub std_min_mm: i32,
    pub std_max_mm: i32,
    pub std_ok: bool,
```

In `seg_views`, after `let n = s.at(self.time_min);` add `let (std_min_mm, std_max_mm) = self.standard().limits(&self.design, n.kind);` and in the `SegView { ... }` literal, after `max_mm,` add:

```rust
                    std_min_mm,
                    std_max_mm,
                    std_ok: (std_min_mm..=std_max_mm).contains(&s.width_mm),
```

5. **`View`.** In the `View` struct, after `pub changed: bool,` add:

```rust
    /// The street's road type, speed and traffic.
    pub design: Design,
    /// Index into the standards: the one the street is checked against.
    pub standard: usize,
    /// The standard's range for each kind on this street, indexed by kind.
    pub std_limits: Vec<[i32; 2]>,
```

In `fn view`, change the `checks:` line to

```rust
            checks: checks(&now, self.row_mm, REGIONS[self.region].drive_side, SAMPLES[self.sample].freeway, self.standard(), &self.design),
```

and after `changed: segs != existing,` add:

```rust
            design: self.design,
            standard: self.standard,
            std_limits: (0..KINDS.len())
                .map(|k| {
                    let (lo, hi) = self.standard().limits(&self.design, k);
                    [lo, hi]
                })
                .collect(),
```

6. **`measure_views`.** Add one line before the `street_measures::detect(...)` statement (after `let freeway = ...;`) and pass its result as the new last argument, leaving the rest of the `.into_iter().zip(...)` chain unchanged:

```rust
        let bus_min_mm = self.standard().limits(&self.design, kind_index("bus").expect("the catalogue has a bus lane")).0;
        street_measures::detect(raw, now, side, freeway, bus_min_mm)
            .into_iter()
```

- [ ] **Step 4: Run to see the tests pass**

Run: `cargo test 2>&1 | grep -E "test result|FAILED|panicked|error"`
Expected: `ok. 137 passed` (131 plus 6). The existing `every_lane_measure_can_be_arranged_and_is_then_recognised` still passes: arranged bus lanes on the avenue may now be told off for width, and it accepts problems that start with "A bus lane is". If any *other* existing test fails because a sample or arranged street now fails the widths check, report it rather than changing numbers.

- [ ] **Step 5: Commit**

```bash
git add src/model.rs src/street_measures.rs
git commit -m "$(cat <<'EOF'
feat: a Lane widths check against the street's standard, and each piece's allowed range

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: The city holds the standard

**Files:**
- Modify: `src/city.rs` (imports, `DEFAULT_STANDARD`, `Saved`, `City`, `new`, `load`, `save`, `street_editor`, `view`, `CityView`, new methods; tests)

**Interfaces:**
- Consumes: `Editor::set_standard`, `standard::{STANDARDS, BALANCED, info, StandardInfo}`.
- Produces:
  - `pub const DEFAULT_STANDARD: usize` (next to `NAME`).
  - `City::set_standard(&mut self, standard: usize) -> bool`, `City::standard(&self) -> usize`.
  - `CityView.standard: usize`, `CityView.standards: Vec<StandardInfo>`.
  - A save written with `"standard": "<id>"`; an older save with no such key loads on the default.

- [ ] **Step 1: Write the failing tests**

At the end of `mod tests` in `src/city.rs` add:

```rust
    #[test]
    fn the_city_starts_on_its_default_standard() {
        let city = City::new();
        assert_eq!(city.standard(), DEFAULT_STANDARD);
        let v = city.view(0);
        assert_eq!(v.standard, DEFAULT_STANDARD);
        assert_eq!(v.standards.iter().map(|s| s.id).collect::<Vec<_>>(), ["compact", "balanced", "generous"]);
    }

    #[test]
    fn the_standard_is_saved_by_its_id() {
        let mut city = City::new();
        assert!(city.set_standard(0));
        let saved = city.save();
        assert!(saved.contains(r#""standard":"compact""#), "{saved}");
        assert_eq!(City::load(&saved).standard(), 0);
        // An id nobody knows falls back to the city's default.
        assert_eq!(City::load(&saved.replace("compact", "nonesuch")).standard(), DEFAULT_STANDARD);
    }

    #[test]
    fn a_save_from_before_designs_loads_with_its_streets() {
        let mut city = City::new();
        let mut e = city.street_editor(1, 0).unwrap();
        let uid = e.view().segments[2].uid;
        assert!(e.set_width(uid, 2_700));
        assert!(city.keep_street(1, e.snapshot()));
        let mut saved: serde_json::Value = serde_json::from_str(&city.save()).unwrap();
        saved.as_object_mut().unwrap().remove("standard");
        for street in saved["streets"].as_object_mut().unwrap().values_mut() {
            assert!(street.as_object_mut().unwrap().remove("design").is_some());
        }
        let back = City::load(&saved.to_string());
        assert_eq!(back.standard(), DEFAULT_STANDARD);
        assert_eq!(back.streets, city.streets);
        assert_eq!(back.streets[&1].design, Design::for_sample(back.streets[&1].sample));
        assert_eq!(back.view(0).edited, 1);
    }

    #[test]
    fn choosing_a_standard_re_checks_every_street() {
        let mut city = City::new();
        assert_eq!(city.view(0).failing, 0);
        assert!(city.set_standard(2));
        let v = city.view(0);
        assert_eq!(v.standard, 2);
        assert!(v.failing > 0);
        assert!(v.edges.iter().any(|e| e.failing.iter().any(|f| f == "Lane widths within the standard")));
        assert!(!city.set_standard(2), "unchanged");
        assert!(!city.set_standard(3), "no such standard");
        assert!(city.set_standard(DEFAULT_STANDARD));
        assert_eq!(city.view(0).failing, 0);
    }

    #[test]
    fn a_street_whose_design_changed_counts_as_edited_and_comes_back_in_the_editor() {
        let mut city = City::new();
        let mut e = city.street_editor(1, 0).unwrap();
        assert!(e.set_speed(30));
        assert!(city.keep_street(1, e.snapshot()));
        assert!(city.view(0).edges[0].edited);
        assert_eq!(city.street_editor(1, 0).unwrap().view().design.speed_kmh, 30);
        city.reset();
        assert_eq!(city.view(0).edited, 0);
        assert_eq!(city.street_editor(1, 0).unwrap().view().design.speed_kmh, 50);
    }
```

In the test module's `use` line (top of `mod tests`; it is `use super::*;`), `Design` must be in scope: add `use crate::standard::Design;` right under it.

- [ ] **Step 2: Run to see them fail**

Run: `cargo test city::tests 2>&1 | tail -20`
Expected: compile errors (`no method named standard`, `cannot find DEFAULT_STANDARD`).

- [ ] **Step 3: Implement**

1. Imports: change `use crate::model::{Editor, Street};` to

```rust
use crate::model::{Editor, Street};
use crate::standard::{self, STANDARDS, StandardInfo};
```

2. Under `pub const NAME: &str = "Sample city";` add:

```rust
/// The standard this city starts on: its default. A resident can pick another.
pub const DEFAULT_STANDARD: usize = standard::BALANCED;
```

3. `Saved`: add

```rust
    /// The id of the standard the city is on. Saves from before standards have none.
    #[serde(default)]
    standard: Option<String>,
```

after `junctions: BTreeMap<u32, State>,`.

4. `City` struct: add `standard: usize,` after `today_junctions`. In `City::new`, set `standard: DEFAULT_STANDARD` in the literal (`City { streets: ..., today_streets, today_junctions, standard: DEFAULT_STANDARD }` — keep the existing field order and add it last).

5. `City::load`: after the version check block and before `for (uid, street) in saved.streets {`, add:

```rust
        if let Some(i) = saved.standard.as_deref().and_then(|id| STANDARDS.iter().position(|s| s.id == id)) {
            city.standard = i;
        }
```

6. `City::save`: change the `Saved { ... }` literal to include `standard: Some(STANDARDS[self.standard].id.to_string()),`.

7. Add a private helper and the new methods inside `impl City`, after `reset`:

```rust
    pub fn standard(&self) -> usize {
        self.standard
    }

    /// Chooses the standard every street is checked against. False when there
    /// is no such standard or it is the one already chosen.
    pub fn set_standard(&mut self, standard: usize) -> bool {
        if standard >= STANDARDS.len() || standard == self.standard {
            return false;
        }
        self.standard = standard;
        true
    }

    /// A street editor on the street as `today` and `now`, checked against the city's standard.
    fn editor_for(&self, today: &Street, now: &Street, region: usize) -> Editor {
        let mut e = Editor::from_street(today, now, region);
        e.set_standard(self.standard);
        e
    }
```

(`Start over` deliberately leaves the standard alone: it is the city's rule, not an edit to a street or junction.)

8. `street_editor`: replace its last line with

```rust
        Some(self.editor_for(self.today_streets.get(&edge)?, self.streets.get(&edge)?, region))
```

9. `City::view`: replace `let editor = Editor::from_street(today, now, region);` with `let editor = self.editor_for(today, now, region);`. Change the last line of the function to

```rust
        CityView { name: NAME, nodes, edges, bounds_mm: [x0, y0, x1, y1], places, failing, edited, standard: self.standard, standards: standard::info() }
```

10. `CityView`: add

```rust
    /// Index into `standards`: the one every street is checked against.
    pub standard: usize,
    pub standards: Vec<StandardInfo>,
```

- [ ] **Step 4: Run to see the tests pass**

Run: `cargo test 2>&1 | grep -E "test result|FAILED|panicked|error"`
Expected: `ok. 142 passed` (137 plus 5). In particular `every_place_in_the_first_city_works` and `the_first_city_works_on_the_other_side_of_the_road_too` still pass: the untouched city meets the default standard.

- [ ] **Step 5: Commit**

```bash
git add src/city.rs
git commit -m "$(cat <<'EOF'
feat: the city holds an engineering standard, a default and a choice of several, and saves it by id

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: WebAssembly exports

**Files:**
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: Tasks 1 to 4.
- Produces (JS names):
  - `Sheet.set_road_type(index)`, `Sheet.set_speed(kmh)`, `Sheet.set_volume(mode, value)`, `Sheet.set_standard(index)` → `boolean`.
  - `City.set_standard(index)` → `boolean`, `City.standard()` → `number`.
  - `standards()` → JSON string `{ road_types: [{id,name}], speed_kmh: {min,max}, volume_max, volumes: [label x4], standards: [{id,name,note}] }`.

- [ ] **Step 1: Add the exports**

In `src/lib.rs`, inside `impl Sheet`, after `apply_measure`, add:

```rust
    pub fn set_road_type(&mut self, index: usize) -> bool {
        self.0.set_road_type(index)
    }

    pub fn set_speed(&mut self, speed_kmh: i32) -> bool {
        self.0.set_speed(speed_kmh)
    }

    /// `mode` is 0 walking, 1 biking, 2 buses, 3 vehicles.
    pub fn set_volume(&mut self, mode: usize, volume: i32) -> bool {
        self.0.set_volume(mode, volume)
    }

    pub fn set_standard(&mut self, index: usize) -> bool {
        self.0.set_standard(index)
    }
```

Below `samples()` add:

```rust
/// What the pages need to draw the engineering standard's controls, as JSON.
#[wasm_bindgen]
pub fn standards() -> String {
    json(&standard::catalogue())
}
```

Inside `impl City`, after `reset`, add:

```rust
    /// The index of the standard the city is checked against.
    pub fn standard(&self) -> usize {
        self.0.standard()
    }

    /// Chooses the standard; false when there is no such one or it is the one already chosen.
    pub fn set_standard(&mut self, index: usize) -> bool {
        self.0.set_standard(index)
    }
```

- [ ] **Step 2: Test build and WebAssembly build**

Run: `cargo test 2>&1 | grep "test result"`
Expected: `ok. 142 passed`.

Run: `./scripts/build.sh 2>&1 | tail -5`
Expected: finishes without error and writes `web/pkg/` (git-ignored).

Run: `grep -c "set_road_type\|set_volume\|standards" web/pkg/cityloom_editor.js`
Expected: a non-zero count.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "$(cat <<'EOF'
feat: export the engineering standard to the pages

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: The street page

**Files:**
- Modify: `web/index.html` (a "Standard" tab and panel)
- Modify: `web/app.js` (load `standards()`; the form; the table; the inspector line; the widths check text)
- Modify: `web/style.css` (form and failure styles)

**Interfaces:**
- Consumes: Task 5 exports; `view.design`, `view.standard`, `view.std_limits`, `view.segments[i].std_*`, `view.checks[i].out_of_range`.
- Produces: the Standard tab and the inspector line described in the spec.

Read `craft-floor.md` (see Global Constraints) before editing.

- [ ] **Step 1: The tab and panel**

In `web/index.html`, after the Measures tab button (`id="t-measures"`) add:

```html
            <button type="button" role="tab" class="tab" id="t-standard" aria-controls="p-standard" aria-selected="false" tabindex="-1">Standard</button>
```

and after the closing `</div>` of the `p-measures` panel add:

```html
          <div class="panel" id="p-standard" role="tabpanel" aria-labelledby="t-standard" hidden>
            <section aria-labelledby="h-standard">
              <h2 id="h-standard" class="note-h">Engineering standard</h2>
              <p class="hint" id="std-lead"></p>
              <div class="std-form" id="std-form"></div>
            </section>
            <section aria-labelledby="h-std-table">
              <h2 id="h-std-table" class="note-h">Allowed widths on this street</h2>
              <table class="nt" id="std-table"></table>
            </section>
          </div>
```

The shared tab code (`initTabs` in `shell.js`) picks the new tab up with no change.

- [ ] **Step 2: Load the catalogue and build the form once**

In `web/app.js`, change the import line to include `standards`:

```js
import init, { Sheet, atlas, catalogue, materials, samples, standards } from "./pkg/cityloom_editor.js";
```

After `const ATLAS = JSON.parse(atlas());` add `const STD = JSON.parse(standards());`.

Add this block after `renderNotes` (before `renderFit`). The form is built once so typing is never interrupted by a redraw; `renderStandard` only updates values, and never the field being edited.

```js
// ---- engineering standard ---------------------------------------------------

const stdField = (id, label, control) => `<div class="field"><label for="${id}">${label}</label>${control}</div>`;
const stdNumber = (id, attrs, tag) =>
  `<span class="wfield"><input type="number" id="${id}" inputmode="numeric" ${attrs}><span class="unit-tag" aria-hidden="true">${tag}</span></span>`;
$("std-form").innerHTML =
  stdField("std-road", "Road type", `<select id="std-road">${STD.road_types.map((r, i) => `<option value="${i}">${esc(r.name)}</option>`).join("")}</select>`) +
  stdField("std-speed", "Design speed", stdNumber("std-speed", `min="${STD.speed_kmh.min}" max="${STD.speed_kmh.max}" step="5"`, "km/h")) +
  `<p class="hint" id="std-vol-h">Traffic in the busiest hour, both directions</p>` +
  STD.volumes.map((label, i) => stdField(`std-vol-${i}`, label, stdNumber(`std-vol-${i}`, `data-volume="${i}" min="0" max="${STD.volume_max}" step="10"`, "/h"))).join("");

const designValue = (input) =>
  input.id === "std-road" ? view.design.road_type : input.id === "std-speed" ? view.design.speed_kmh : view.design.volumes[Number(input.dataset.volume)];

function renderStandard() {
  const std = STD.standards[view.standard];
  $("std-lead").textContent = `${std.name}. ${std.note} Chosen for the whole city on the map.`;
  for (const input of $("std-form").querySelectorAll("input, select")) {
    if (document.activeElement !== input) input.value = designValue(input);
  }
  const rows = KINDS.map((k, i) => {
    const [lo, hi] = view.std_limits[i];
    const here = view.segments.filter((s) => s.kind === i);
    const now = here.length
      ? here.map((s) => (s.std_ok ? fmtN(s.width_mm) : `<span class="bad">${fmtN(s.width_mm)}<span class="sr-only"> (outside the range)</span></span>`)).join(", ")
      : "—";
    return `<tr><td>${esc(k.name)}</td><td>${fmtN(lo)} to ${fmtN(hi)}</td><td>${now}</td></tr>`;
  }).join("");
  $("std-table").innerHTML = `<caption class="sr-only">Allowed width of each kind of piece on this street, in ${unitWord()}</caption>${head(["Piece", "Allowed", "Now"])}<tbody>${rows}</tbody>`;
}

// A value the model refuses is put back as it was, and the person is told.
$("std-form").addEventListener("change", (e) => {
  const input = e.target.closest("input, select");
  if (!input) return;
  const typed = input.value;
  const n = typed.trim() === "" ? NaN : Number(typed);
  let ok = false;
  if (input.id === "std-road") ok = sheet.set_road_type(n);
  else if (Number.isInteger(n)) ok = input.id === "std-speed" ? sheet.set_speed(n) : sheet.set_volume(Number(input.dataset.volume), n);
  refresh();
  const now = designValue(input);
  input.value = now;
  const name = input.labels[0].textContent;
  if (ok) say(`${name}: ${input.tagName === "SELECT" ? STD.road_types[now].name : now}.`);
  else if (String(now) !== typed) say(`${name} needs a whole number from ${input.min} to ${input.max}. Kept ${now}.`);
});
```

(`say`, `refresh`, `head`, `fmtN`, `unitWord`, `esc`, `KINDS` already exist in this file.)

In `render()`, add `renderStandard();` after `renderNotes();`.

- [ ] **Step 3: The widths check and the inspector line**

In `checkDetail`, before `return c.detail;` add:

```js
  if (c.id === "widths") {
    if (c.ok) return "Every piece is inside the standard's range";
    const [w, ...rest] = c.out_of_range;
    const first = `${KINDS[w.kind].name} ${fmt(w.width_mm)}, standard allows ${fmtN(w.min_mm)} to ${fmt(w.max_mm)}`;
    return rest.length ? `${first} and ${rest.length} more` : first;
  }
```

In `renderInspector`, directly after the `const hi = ...` line add:

```js
  const stdLine = s.std_ok
    ? `Standard: ${num(s.std_min_mm).toFixed(2)} to ${num(s.std_max_mm).toFixed(2)} ${units}`
    : `${ICON.bad}Standard: ${num(s.std_min_mm).toFixed(2)} to ${num(s.std_max_mm).toFixed(2)} ${units}, ${
        s.width_mm < s.std_min_mm ? `${fmt(s.std_min_mm - s.width_mm)} under the minimum` : `${fmt(s.width_mm - s.std_max_mm)} over the maximum`
      }`;
```

and in the template, after the existing `<p class="insp-range">Allowed ${lo} to ${hi} ${units}</p>` line add:

```js
      <p class="insp-range${s.std_ok ? "" : " std-bad"}">${stdLine}</p>
```

- [ ] **Step 4: Styles**

Append to `web/style.css`:

```css
/* ---- engineering standard ---------------------------------------------- */
.std-form {
  display: grid;
  gap: 8px;
  margin: 8px 0 14px;
}
.std-form .field {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 4px 12px;
  font-size: 15px;
}
.std-form label {
  font-weight: 600;
}
.std-form select {
  min-height: 30px;
  max-width: 190px;
  padding: 2px 6px;
  border: var(--line) solid var(--ink);
  border-radius: 0;
  background: var(--sheet);
  color: var(--ink);
  font: inherit;
  font-size: 14px;
}
.std-form .hint {
  margin: 6px 0 0;
}
.nt .bad {
  color: var(--red);
  font-weight: 600;
}
.insp-range.std-bad {
  color: var(--red);
  font-weight: 600;
}
.insp-range svg {
  width: 14px;
  height: 14px;
  margin-right: 4px;
  vertical-align: -2px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: square;
}
```

- [ ] **Step 5: Look at it once, in one batched pass**

Run `./scripts/build.sh`, then `python3 -m http.server 8137 --directory web` in the background, and use the claude-in-chrome skill. In one pass, desktop width and a phone width (about 390 px) together, in light and dark:

1. `http://127.0.0.1:8137/map.html`, open the first street. Open the Standard tab: the form shows Collector / 40 km/h and the four volumes; the table lists nine kinds with ranges in metres.
2. Select the driving lane and type a width over its range. The inspector's second line goes red with words and an icon. The Checks tab shows "Lane widths within the standard" failing with the lane named. The Standard table marks the width.
3. Switch units to ft: the form is unchanged (km/h), the table and the inspector line follow.
4. Type `abc`, an empty value, `-5`, `1000` into speed: each keeps the last value and the live region says so. Type `50`: it takes.
5. Keyboard only: Tab through the form, change a value with Enter.
6. Reload the page: the design persisted (the street is saved with the city).
7. Check the page at 390 px: the form fields wrap and nothing scrolls sideways.

Fix everything this shows in one batch, check once more, then stop.

- [ ] **Step 6: Commit**

```bash
git add web/index.html web/app.js web/style.css
git commit -m "$(cat <<'EOF'
feat(web): the street's road type, speed and traffic, its allowed widths, and the lane width check

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: The map page

**Files:**
- Modify: `web/map.html` (a "Standard" tab and panel)
- Modify: `web/map.js` (load `standards()`, the select)

**Interfaces:**
- Consumes: `City.set_standard`, `view.standard`, `view.standards` (via `STD.standards` too), `writeCity` from `city.js`.
- Produces: a select on the map's notes that chooses the city's standard and re-checks every street.

Read `craft-floor.md` again if it has dropped out of context.

- [ ] **Step 1: The tab and panel**

In `web/map.html`, after the Changes tab button (`id="t-changes"`) add:

```html
            <button type="button" role="tab" class="tab" id="t-standard" aria-controls="p-standard" aria-selected="false" tabindex="-1">Standard</button>
```

and after the closing `</div>` of the `p-changes` panel add:

```html
          <div class="panel" id="p-standard" role="tabpanel" aria-labelledby="t-standard" hidden>
            <section aria-labelledby="h-standard">
              <h2 id="h-standard" class="note-h">Engineering standard</h2>
              <p class="hint">The lane widths every street in the city is checked against. Each street's road type, speed and traffic are set on its own page.</p>
              <div class="std-form"><div class="field"><label for="std-pick">Standard</label><select id="std-pick"></select></div></div>
              <p class="hint" id="std-note"></p>
            </section>
          </div>
```

- [ ] **Step 2: Wire the select**

In `web/map.js` change the imports:

```js
import init, { catalogue, materials, standards } from "./pkg/cityloom_editor.js";
import { openCity, regionIndex, resetCity, writeCity } from "./city.js";
```

After `const KIND_ORDER = ...;` add `const STD = JSON.parse(standards());`.

Add this block before the `// ---- start over` section:

```js
// ---- engineering standard ---------------------------------------------------

const pick = $("std-pick");
pick.innerHTML = STD.standards.map((s, i) => `<option value="${i}">${esc(s.name)}</option>`).join("");

function renderStandard() {
  pick.value = String(view.standard);
  $("std-note").textContent = STD.standards[view.standard].note;
}

pick.addEventListener("change", () => {
  writeCity((c) => c.set_standard(Number(pick.value)));
  reload();
  const name = STD.standards[view.standard].name;
  say(view.failing ? `${name} standard. ${plural(view.failing, "place")} now fail${view.failing === 1 ? "s" : ""} a check.` : `${name} standard. Every place works.`);
});
```

and add `renderStandard();` to `render()` after `renderNotes();`.

- [ ] **Step 3: Look at it once, in one batched pass**

Run `./scripts/build.sh`; with the server from Task 6 still running, use the claude-in-chrome skill, desktop and about 390 px, light and dark:

1. `map.html`, Standard tab: the select shows Balanced; the note reads under it.
2. Choose Generous: the live region announces how many places fail; the Checks tab lists streets with "Lane widths within the standard"; open one such street and see the same check there and the new range in its Standard tab.
3. Choose Balanced again: every place works.
4. Reload: the choice persisted. Open the street page in a second tab, change the standard on the map, return: the street page reads the new standard after reload.
5. Keyboard only: Tab to the select and change it with the arrow keys.

Fix everything this shows in one batch, check once more, then stop.

- [ ] **Step 4: Commit**

```bash
git add web/map.html web/map.js
git commit -m "$(cat <<'EOF'
feat(web): choose the city's engineering standard on the map

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: Docs and spec

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-10-01-engineering-standard-design.md`
- Modify: `DESIGN.md` (one short paragraph)

- [ ] **Step 1: README**

In `README.md`, in the `src/` bullet, after the mention of `model.rs` add `, `standard.rs` for the engineering standard that gives each kind of piece a width range by road type, speed and traffic volume`. Keep the line otherwise as it is.

- [ ] **Step 2: Bring the spec in line with what was built**

In `docs/superpowers/specs/2026-10-01-engineering-standard-design.md`:
- Replace the word `profile` with `design` wherever it names the per-street inputs (the heading "Per-street inputs", the `Street` field, the table intro, `for_side`/`reversed`, the tests list), and add one sentence under "Per-street inputs": "It is called `Design` in code, because `junction::Profile` already exists."
- In "City", delete the bullet about `City::street_profile` and `City::set_street_profile`, and replace it with: "A street's design travels with the street: the street editor holds it and `keep_street` stores it, so the city needs no separate accessors."
- Under "Model, the check", add: "`Check` carries `out_of_range` (each piece outside the range, furthest out first) so the page can print the lanes in its own units."
- Under "City", add: "Start over leaves the chosen standard alone: it is the city's rule, not an edit to a street."
- Under "A standard", add: "The 16 rows are built from an anchor row, a step per speed band, a step per road type and the volume steps, which keeps a standard short. The lookup is unchanged."
- Remove the "Open items for the plan" section.

- [ ] **Step 3: DESIGN.md**

Append a short paragraph to the part of `DESIGN.md` that describes the editor's notes, saying: the Standard tab and the standard range line in the lane inspector use the existing field, hint and redline styles; a piece outside the standard is shown in redline with words ("0.20 m over the maximum") and an icon, never colour alone. Match the surrounding heading level and tone.

- [ ] **Step 4: Final check and commit**

Run: `cargo test 2>&1 | grep "test result"`
Expected: `ok. 142 passed`.

```bash
git add README.md DESIGN.md docs/superpowers/specs/2026-10-01-engineering-standard-design.md
git commit -m "$(cat <<'EOF'
docs: the engineering standard in the README, the design notes and the spec

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
EOF
)"
```

---

## Self-review

**Spec coverage.**
- Road type, speed and per-mode volume stored on each street: Tasks 1, 2.
- Advisory "Lane widths" check naming lane, width and range: Task 3; page text in the user's units: Task 6.
- Every kind covered; volume of the relevant mode only: Task 1 (`raw_limits`, `VOLUME_MODES`, tests `a_piece_with_no_traffic...`, `a_busier_sidewalk...`).
- `KINDS` limits kept; standard inside them; test that no built-in relies on clamping: Task 1.
- Three built-in standards, a city default, choice on the map, saved by id: Tasks 1, 4, 7.
- Old saves load without a version bump: Tasks 2 (`SavedStreet`) and 4 (test).
- `MIN_TRANSIT_LANE_MM` replaced by the standard's bus minimum: Task 3.
- Setting-not-history, edited-on-map: Tasks 2, 4.
- Standard tab, inspector line, Checks tab, map select: Tasks 6, 7.
- Input handling for bad values: Tasks 2 (model) and 6 (page).
- Tests listed in the spec: all present except "the three standards give different answers" (present as `the_standards_differ`) and the browser check (manual, Tasks 6, 7).

**Deviations from the spec**, all recorded in Task 8: `profile` is `Design`; no `City::street_profile` accessors; `Check.out_of_range` added; Start over leaves the standard; the 16 rows are computed from an anchor and steps.

**Placeholder scan.** No TBD/TODO. The one judgement call left to the engineer is in Task 3 step 3.6 (binding `found` so the `detect` call compiles), which names the exact change. Task 8 step 3 asks for one paragraph of prose in DESIGN.md, with its content given.

**Type consistency.** `Design { road_type, speed_kmh, volumes }`, `Standard::{raw_limits, limits}`, `Editor::{set_road_type, set_speed, set_volume, set_standard}`, `City::{set_standard, standard}`, `OutOfRange { kind, width_mm, min_mm, max_mm }`, `View.{design, standard, std_limits}`, `SegView.{std_min_mm, std_max_mm, std_ok}`, `CityView.{standard, standards}` and the JS reads of them match across tasks. `STANDARDS` is a `static` (not `const`) so `Editor::standard()` can return `&'static Standard`. Test counts: 113, then 125, 131, 137, 142.
