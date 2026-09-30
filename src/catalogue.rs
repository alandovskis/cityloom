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
            Mode::Foot => "On foot",
            Mode::Bike => "By bike",
            Mode::Transit => "By bus",
            Mode::Vehicle => "Vehicles",
            Mode::Green => "Planting",
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
}

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
    },
    Kind {
        id: "travel",
        name: "Travel lane",
        mark: "TL",
        mode: Mode::Vehicle,
        default_mm: 3300,
        min_mm: 2700,
        max_mm: 4000,
        people_per_hour_per_m: 900,
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
    },
    Kind {
        id: "parking",
        name: "Parking lane",
        mark: "PK",
        mode: Mode::Vehicle,
        default_mm: 2400,
        min_mm: 2100,
        max_mm: 3000,
        people_per_hour_per_m: 0,
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
