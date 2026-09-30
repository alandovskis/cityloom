//! Placeholder segment catalogue, outcome rates and sample streets.
//!
//! Everything here is synthetic. The catalogue, the widths and the
//! `people_per_hour_per_m` rates are stand-ins until real data is chosen.

use serde::Serialize;

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
    pub const ALL: [Mode; 5] = [
        Mode::Foot,
        Mode::Bike,
        Mode::Transit,
        Mode::Vehicle,
        Mode::Green,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Foot => "Walking",
            Mode::Bike => "Biking",
            Mode::Transit => "Buses",
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
}

/// A surface or curb material. Synthetic placeholder list, like the rest of
/// the catalogue.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Material {
    pub id: &'static str,
    pub name: &'static str,
}

pub const MATERIALS: [Material; 7] = [
    Material { id: "asphalt", name: "Asphalt" },
    Material { id: "concrete", name: "Concrete" },
    Material { id: "permeable", name: "Permeable paving" },
    Material { id: "brick", name: "Brick pavers" },
    Material { id: "grass", name: "Grass" },
    Material { id: "planted", name: "Planted bed" },
    Material { id: "gravel", name: "Gravel" },
];

pub const CURBS: [Material; 6] = [
    Material { id: "granite", name: "Granite" },
    Material { id: "concrete", name: "Concrete" },
    Material { id: "asphalt", name: "Asphalt" },
    Material { id: "planted", name: "Planted" },
    Material { id: "kassel", name: "Bus-friendly curb" },
    Material { id: "bikefriendly", name: "Bike-friendly curb" },
];

/// A new sidewalk or bike lane gets this curb (concrete).
pub const DEFAULT_CURB: usize = 1;

pub const KINDS: [Kind; 8] = [
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
        has_curb: true,
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
        materials: &[4, 5, 6],
        has_curb: false,
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
        has_curb: true,
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
        has_curb: false,
    },
    Kind {
        id: "bus",
        name: "Bus lane",
        mark: "BU",
        mode: Mode::Transit,
        default_mm: 3300,
        min_mm: 3000,
        max_mm: 4000,
        people_per_hour_per_m: 4000,
        materials: &[0, 1, 2],
        has_curb: false,
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
        has_curb: false,
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
        has_curb: false,
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
        has_curb: false,
    },
];

pub fn kind_index(id: &str) -> Option<usize> {
    KINDS.iter().position(|k| k.id == id)
}

pub struct Sample {
    pub name: &'static str,
    pub row_mm: i32,
    pub segments: &'static [(&'static str, i32)],
}

pub const SAMPLES: [Sample; 3] = [
    Sample {
        name: "Sample Street 1",
        row_mm: 18000,
        segments: &[
            ("sidewalk", 3300),
            ("parking", 2400),
            ("travel", 3300),
            ("travel", 3300),
            ("parking", 2400),
            ("sidewalk", 3300),
        ],
    },
    Sample {
        name: "Sample Avenue 2",
        row_mm: 30000,
        segments: &[
            ("sidewalk", 3500),
            ("planting", 1500),
            ("parking", 2400),
            ("travel", 3300),
            ("travel", 3300),
            ("median", 2000),
            ("travel", 3300),
            ("travel", 3300),
            ("parking", 2400),
            ("planting", 1500),
            ("sidewalk", 3500),
        ],
    },
    Sample {
        name: "Sample Lane 3",
        row_mm: 12000,
        segments: &[
            ("sidewalk", 2100),
            ("travel", 3000),
            ("travel", 3000),
            ("parking", 2400),
            ("sidewalk", 1500),
        ],
    },
];
