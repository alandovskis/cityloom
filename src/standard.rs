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
