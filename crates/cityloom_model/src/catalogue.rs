//! The catalogue of the pieces a street is made of: their kinds, surface materials, curbs and directions, which a
//! street's `Segment`s name by index.
//!
//! Everything here is synthetic. The catalogue, the widths and the
//! `people_per_hour_per_m` rates are stand-ins until real data is chosen.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Foot,
    Bike,
    Transit,
    Vehicle,
    Green,
    /// Space no travel mode owns, between pieces: the buffer. It is not in `ALL`, so no share counts it.
    Neutral,
}

impl Mode {
    /// The modes the width is shared out among, in the order the table lists them.
    pub const ALL: [Mode; 5] = [Mode::Foot, Mode::Bike, Mode::Transit, Mode::Vehicle, Mode::Green];
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Kind {
    pub id: &'static str,
    /// Two-letter mark used on the sheet and in the schedule.
    pub mark: &'static str,
    pub mode: Mode,
    pub default_mm: i32,
    pub min_mm: i32,
    pub max_mm: i32,
    /// Synthetic capacity rate. Zero where the segment moves nobody.
    pub people_per_hour_per_m: i32,
    /// Allowed surface materials, as indices into `MATERIALS`. The first is
    /// the default.
    pub materials: &'static [usize],
    /// Whether this kind of piece has a curb material to choose.
    pub has_curb: bool,
    /// The curbs it may have, as indices into `CURBS`; empty without a curb.
    pub curbs: &'static [usize],
    /// Whether this kind of piece runs one way, and if so whether it must.
    pub direction: DirectionRule,
    /// Whether this kind is roadway space that can change type through the
    /// day: a lane that is a bus lane at rush hour and parking otherwise.
    pub shares_road: bool,
}

/// A driving lane must run one way; a bike lane may, or may be two-way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DirectionRule {
    None,
    Required,
    Optional,
}

/// A surface or curb material. Synthetic placeholder list, like the rest of
/// the catalogue.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Material {
    pub id: &'static str,
    pub name: &'static str,
}

/// The materials a piece can be of are the first eight; `rain_garden` is last so that stored indices hold, and no kind
/// lists it: it is only what a rain garden is drawn in (a curbside use, a green element).
pub const MATERIALS: [Material; 9] = [
    Material { id: "asphalt", name: "Asphalt" },
    Material { id: "concrete", name: "Concrete" },
    Material { id: "permeable", name: "Permeable paving" },
    Material { id: "brick", name: "Brick pavers" },
    Material { id: "grass", name: "Grass" },
    Material { id: "planted", name: "Planted bed" },
    Material { id: "gravel", name: "Gravel" },
    Material { id: "trees", name: "Street trees" },
    Material { id: "rain_garden", name: "Rain garden" },
];

/// Which way a one-way lane runs, seen from the cross-section: away from the
/// viewer (into the page) or toward them.
pub const DIRECTIONS: [Material; 2] = [Material { id: "away", name: "Away from you" }, Material { id: "toward", name: "Toward you" }];

/// The side of the road a region drives on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Right,
}

/// What sort of street a road is. It decides what a street may be: a motorway is limited-access.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreetClass {
    Motorway,
    Arterial,
    Collector,
    Local,
    /// A street with no motor piece (`is_motor`; bike lanes are allowed). The street editor derives it from the pieces.
    Pedestrian,
}

impl StreetClass {
    /// Every class, from the widest-reaching road to the quietest.
    pub const ALL: [StreetClass; 5] = [StreetClass::Motorway, StreetClass::Arterial, StreetClass::Collector, StreetClass::Local, StreetClass::Pedestrian];

    /// The word the class is kept and drawn by (the map's `class` property): its serde name.
    pub fn id(self) -> &'static str {
        match self {
            StreetClass::Motorway => "motorway",
            StreetClass::Arterial => "arterial",
            StreetClass::Collector => "collector",
            StreetClass::Local => "local",
            StreetClass::Pedestrian => "pedestrian",
        }
    }

    /// A limited-access road: no sidewalks, and a median and shoulders instead.
    pub fn is_freeway(self) -> bool {
        self == StreetClass::Motorway
    }
}

pub const CURBS: [Material; 7] = [
    Material { id: "granite", name: "Granite" },
    Material { id: "concrete", name: "Concrete" },
    Material { id: "asphalt", name: "Asphalt" },
    Material { id: "planted", name: "Planted" },
    Material { id: "kassel", name: "Bus-friendly curb" },
    Material { id: "bikefriendly", name: "Bike-friendly curb" },
    Material { id: "island", name: "Bus boarding island" },
];

/// A new sidewalk or bike lane gets this curb (concrete).
pub const DEFAULT_CURB: usize = 1;

pub const KINDS: [Kind; 18] = [
    Kind {
        id: "sidewalk",
        mark: "SW",
        mode: Mode::Foot,
        default_mm: 3000,
        min_mm: 1200,
        max_mm: 8000,
        people_per_hour_per_m: 2500,
        materials: &[1, 3, 0, 2],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: true,
        curbs: &[0, 1, 2, 3, 4, 5],
    },
    Kind {
        id: "planting",
        mark: "PL",
        mode: Mode::Green,
        default_mm: 1800,
        min_mm: 600,
        max_mm: 6000,
        people_per_hour_per_m: 0,
        materials: &[7, 4, 5, 6],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "bike",
        mark: "BL",
        mode: Mode::Bike,
        default_mm: 1800,
        min_mm: 1200,
        max_mm: 3000,
        people_per_hour_per_m: 3500,
        materials: &[0, 1, 2],
        direction: DirectionRule::Optional,
        shares_road: true,
        has_curb: true,
        curbs: &[0, 1, 2, 3, 4, 5, 6],
    },
    Kind {
        id: "travel",
        mark: "TL",
        mode: Mode::Vehicle,
        default_mm: 3300,
        min_mm: 2700,
        max_mm: 4000,
        people_per_hour_per_m: 900,
        materials: &[0, 1, 2],
        direction: DirectionRule::Required,
        shares_road: true,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "bus",
        mark: "TR",
        mode: Mode::Transit,
        default_mm: 3300,
        min_mm: 3000,
        max_mm: 4000,
        people_per_hour_per_m: 4000,
        materials: &[0, 1, 2],
        direction: DirectionRule::Optional,
        shares_road: true,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "parking",
        mark: "PK",
        mode: Mode::Vehicle,
        default_mm: 2400,
        min_mm: 2100,
        max_mm: 3000,
        people_per_hour_per_m: 0,
        materials: &[0, 2, 1],
        direction: DirectionRule::None,
        shares_road: true,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "median",
        mark: "MD",
        mode: Mode::Green,
        default_mm: 1800,
        min_mm: 600,
        max_mm: 6000,
        people_per_hour_per_m: 0,
        materials: &[4, 5, 6],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "loading",
        mark: "LZ",
        mode: Mode::Vehicle,
        default_mm: 2400,
        min_mm: 2100,
        max_mm: 3600,
        people_per_hour_per_m: 0,
        materials: &[0, 1, 2],
        direction: DirectionRule::None,
        shares_road: true,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "shoulder",
        mark: "SD",
        mode: Mode::Vehicle,
        default_mm: 3000,
        min_mm: 1800,
        max_mm: 3600,
        people_per_hour_per_m: 0,
        materials: &[0, 1, 2],
        direction: DirectionRule::None,
        shares_road: true,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "bikerack",
        mark: "BR",
        mode: Mode::Foot,
        default_mm: 1200,
        min_mm: 600,
        max_mm: 2400,
        people_per_hour_per_m: 0,
        materials: &[1, 3, 0, 2],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "bikeshare",
        mark: "BS",
        mode: Mode::Foot,
        default_mm: 2000,
        min_mm: 1200,
        max_mm: 3000,
        people_per_hour_per_m: 0,
        materials: &[1, 3, 0, 2],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "pole",
        mark: "UP",
        mode: Mode::Foot,
        default_mm: 600,
        min_mm: 300,
        max_mm: 1500,
        people_per_hour_per_m: 0,
        materials: &[1, 3, 0, 2],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "busshelter",
        mark: "SH",
        mode: Mode::Foot,
        default_mm: 1500,
        min_mm: 1000,
        max_mm: 3000,
        people_per_hour_per_m: 0,
        materials: &[1, 3, 0, 2],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "busstation",
        mark: "ST",
        mode: Mode::Foot,
        default_mm: 3000,
        min_mm: 2000,
        max_mm: 6000,
        people_per_hour_per_m: 0,
        materials: &[1, 3, 0, 2],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "bench",
        mark: "BN",
        mode: Mode::Foot,
        default_mm: 700,
        min_mm: 400,
        max_mm: 1500,
        people_per_hour_per_m: 0,
        materials: &[1, 3, 0, 2],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "terrace",
        mark: "CT",
        mode: Mode::Foot,
        default_mm: 2000,
        min_mm: 1200,
        max_mm: 5000,
        people_per_hour_per_m: 0,
        materials: &[1, 3, 0, 2],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "streetlamp",
        mark: "SL",
        mode: Mode::Foot,
        default_mm: 500,
        min_mm: 300,
        max_mm: 1000,
        people_per_hour_per_m: 0,
        materials: &[1, 3, 0, 2],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: false,
        curbs: &[],
    },
    Kind {
        id: "buffer",
        mark: "BF",
        mode: Mode::Neutral,
        default_mm: 600,
        min_mm: 150,
        max_mm: 3000,
        people_per_hour_per_m: 0,
        materials: &[1, 0],
        direction: DirectionRule::None,
        shares_road: false,
        has_curb: false,
        curbs: &[],
    },
];

pub fn kind_index(id: &str) -> Option<usize> {
    KINDS.iter().position(|k| k.id == id)
}

/// The kinds of piece that are cars, at any time of day: what the Car-free rule of an area does not allow. In the
/// order the rule words them. A bus is not one.
pub const CAR_KINDS: [&str; 3] = ["travel", "parking", "loading"];

/// The kinds of piece that are motor, at any time of day: a street with none is pedestrian (a bike lane is fine).
/// The cars and, besides them, the bus lane and the shoulder; `CAR_KINDS` is a subset.
pub const MOTOR_KINDS: [&str; 5] = ["travel", "parking", "loading", "bus", "shoulder"];

/// Whether a kind of piece (an index into `KINDS`) is a car piece.
pub fn is_car(kind: usize) -> bool {
    CAR_KINDS.contains(&KINDS[kind].id)
}

/// Whether a kind of piece (an index into `KINDS`) is a motor piece.
pub fn is_motor(kind: usize) -> bool {
    MOTOR_KINDS.contains(&KINDS[kind].id)
}

/// Whether a kind of piece is part of the roadway, as opposed to the footway, the green or the buffer between pieces.
pub fn is_roadway(kind: usize) -> bool {
    !matches!(KINDS[kind].mode, Mode::Foot | Mode::Green | Mode::Neutral)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cars_are_travel_parking_and_loading_and_motor_pieces_add_the_bus_and_the_shoulder() {
        let ids = |f: fn(usize) -> bool| -> Vec<&str> { KINDS.iter().enumerate().filter(|(i, _)| f(*i)).map(|(_, k)| k.id).collect() };
        assert_eq!(ids(is_car), ["travel", "parking", "loading"]);
        assert_eq!(CAR_KINDS, ["travel", "parking", "loading"]);
        assert_eq!(ids(is_motor), ["travel", "bus", "parking", "loading", "shoulder"], "in the order of KINDS");
        assert_eq!(MOTOR_KINDS, ["travel", "parking", "loading", "bus", "shoulder"]);
        assert!(CAR_KINDS.iter().all(|id| MOTOR_KINDS.contains(id)), "what a car-free area forbids is motor too");
        for id in ["bike", "sidewalk", "median", "buffer", "bikerack", "busshelter", "busstation"] {
            assert!(!is_car(kind_index(id).unwrap()) && !is_motor(kind_index(id).unwrap()), "{id}");
        }
        for id in ["bus", "shoulder"] {
            assert!(is_motor(kind_index(id).unwrap()) && !is_car(kind_index(id).unwrap()), "{id}");
        }
    }

    #[test]
    fn a_class_is_kept_by_its_word_and_only_a_motorway_is_a_freeway() {
        assert_eq!(StreetClass::ALL.map(StreetClass::id), ["motorway", "arterial", "collector", "local", "pedestrian"]);
        assert_eq!(serde_json::to_string(&StreetClass::Pedestrian).unwrap(), "\"pedestrian\"");
        assert_eq!(StreetClass::ALL.map(StreetClass::is_freeway), [true, false, false, false, false]);
    }
}
