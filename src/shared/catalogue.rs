//! Placeholder segment catalogue and outcome rates.
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
}

impl Mode {
    pub const ALL: [Mode; 5] = [Mode::Foot, Mode::Bike, Mode::Transit, Mode::Vehicle, Mode::Green];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Foot => "Walking",
            Mode::Bike => "Biking",
            Mode::Transit => "Transit",
            Mode::Vehicle => "Cars and trucks",
            Mode::Green => "Greenery",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Kind {
    pub id: &'static str,
    pub name: &'static str,
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

pub const MATERIALS: [Material; 8] = [
    Material { id: "asphalt", name: "Asphalt" },
    Material { id: "concrete", name: "Concrete" },
    Material { id: "permeable", name: "Permeable paving" },
    Material { id: "brick", name: "Brick pavers" },
    Material { id: "grass", name: "Grass" },
    Material { id: "planted", name: "Planted bed" },
    Material { id: "gravel", name: "Gravel" },
    Material { id: "trees", name: "Street trees" },
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

/// A region carries the rules its streets share. For now that is which side
/// of the road traffic keeps to. Synthetic placeholder list.
/// What sort of street a road is. It decides what a street may be: a motorway is limited-access.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreetClass {
    Motorway,
    Arterial,
    Collector,
    Local,
}

impl StreetClass {
    /// A limited-access road: no sidewalks, and a median and shoulders instead.
    pub fn is_freeway(self) -> bool {
        self == StreetClass::Motorway
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Region {
    pub id: &'static str,
    pub name: &'static str,
    pub drive_side: Side,
}

pub const REGIONS: [Region; 6] = [
    Region { id: "canada", name: "Canada", drive_side: Side::Right },
    Region { id: "united-states", name: "United States", drive_side: Side::Right },
    Region { id: "germany", name: "Germany", drive_side: Side::Right },
    Region { id: "united-kingdom", name: "United Kingdom", drive_side: Side::Left },
    Region { id: "australia", name: "Australia", drive_side: Side::Left },
    Region { id: "japan", name: "Japan", drive_side: Side::Left },
];

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

pub const KINDS: [Kind; 17] = [
    Kind {
        id: "sidewalk",
        name: "Sidewalk",
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
        name: "Planting strip",
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
        name: "Bike lane",
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
        name: "Driving lane",
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
        name: "Transit lane",
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
        name: "Parking",
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
        name: "Planted median",
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
        name: "Loading zone",
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
        name: "Shoulder",
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
        name: "Bike rack",
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
        name: "Bikeshare station",
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
        name: "Utility pole",
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
        name: "Bus shelter",
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
        name: "Bus station",
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
        name: "Bench",
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
        name: "Café terrace",
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
        name: "Street lamp",
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
];

pub fn kind_index(id: &str) -> Option<usize> {
    KINDS.iter().position(|k| k.id == id)
}

/// Whether a kind of piece is part of the roadway, as opposed to the footway or the green.
pub fn is_roadway(kind: usize) -> bool {
    !matches!(KINDS[kind].mode, Mode::Foot | Mode::Green)
}
