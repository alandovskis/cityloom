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

/// A region carries the rules its streets share. For now that is which side
/// of the road traffic keeps to. Synthetic placeholder list.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Region {
    /// Its name is the message `region-<id>` (`shell/i18n`).
    pub id: &'static str,
    pub drive_side: Side,
}

pub const REGIONS: [Region; 6] = [
    Region { id: "canada", drive_side: Side::Right },
    Region { id: "united-states", drive_side: Side::Right },
    Region { id: "germany", drive_side: Side::Right },
    Region { id: "united-kingdom", drive_side: Side::Left },
    Region { id: "australia", drive_side: Side::Left },
    Region { id: "japan", drive_side: Side::Left },
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

/// What the space of a street's motor pieces can become when the street is made pedestrian, by kind id.
pub const PEDESTRIAN_KINDS: [&str; 3] = ["sidewalk", "planting", "terrace"];

/// Whether a kind of piece is part of the roadway, as opposed to the footway, the green or the buffer between pieces.
pub fn is_roadway(kind: usize) -> bool {
    !matches!(KINDS[kind].mode, Mode::Foot | Mode::Green | Mode::Neutral)
}

/// The groups the add menu sorts the kinds into, in its order. A group's name is the message `group-<id>`.
pub const GROUP_IDS: [&str; 7] = ["walking", "greenery", "cycling", "transit", "roadway", "furniture", "utilities"];

/// The message that names a class of street, as a word to put in a sentence (`city/i18n`).
pub fn class_key(class: StreetClass) -> &'static str {
    match class {
        StreetClass::Motorway => "class-motorway",
        StreetClass::Arterial => "class-arterial",
        StreetClass::Collector => "class-collector",
        StreetClass::Local => "class-local",
        StreetClass::Pedestrian => "class-pedestrian",
    }
}

/// The message that names a kind of piece (`street/i18n`).
pub fn kind_key(id: &str) -> String {
    format!("kind-{id}")
}

/// The message that gives a kind's two-letter mark, shown where its name does not fit.
pub fn mark_key(id: &str) -> String {
    format!("kind-mark-{id}")
}

/// The message that names a group of kinds.
pub fn group_key(id: &str) -> String {
    format!("group-{id}")
}

/// The message that names a surface material.
pub fn material_key(id: &str) -> String {
    format!("material-{id}")
}

/// The message that names a kind of curb.
pub fn curb_key(id: &str) -> String {
    format!("curb-{id}")
}

/// The message that names a direction of travel.
pub fn direction_key(id: &str) -> String {
    format!("direction-{id}")
}

/// The message that names a mode.
pub fn mode_key(mode: Mode) -> &'static str {
    match mode {
        Mode::Foot => "mode-foot",
        Mode::Bike => "mode-bike",
        Mode::Transit => "mode-transit",
        Mode::Vehicle => "mode-vehicle",
        Mode::Green => "mode-green",
        Mode::Neutral => "mode-neutral",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::{Args, Locale};

    /// The display word of each group, as the add menu shows it today.
    const GROUP_WORDS: [&str; 7] = ["Walking", "Greenery", "Cycling", "Transit", "Roadway", "Furniture", "Utilities"];

    #[test]
    fn every_catalogue_kind_mode_and_group_has_a_message_in_both_languages() {
        for locale in Locale::ALL {
            let i = crate::i18n_for(locale);
            for k in KINDS.iter() {
                i.tr_now(&kind_key(k.id), &Args::new());
            }
            for k in KINDS.iter() {
                i.tr_now(&mark_key(k.id), &Args::new());
            }
            for m in Mode::ALL.into_iter().chain([Mode::Neutral]) {
                i.tr_now(mode_key(m), &Args::new());
            }
            for g in GROUP_IDS {
                i.tr_now(&group_key(g), &Args::new());
            }
            assert!(i.missing().is_empty(), "{:?}: {:?}", locale, i.missing());
        }
    }

    #[test]
    fn the_english_messages_say_what_the_marks_and_groups_still_say() {
        let i = crate::i18n_for(Locale::En);
        assert_eq!(i.tr_now(&kind_key("sidewalk"), &Args::new()), "Sidewalk");
        assert_eq!(i.tr_now(mode_key(Mode::Vehicle), &Args::new()), "Cars and trucks");
        for k in KINDS.iter() {
            assert_eq!(i.tr_now(&mark_key(k.id), &Args::new()), k.mark, "{}", k.id);
        }
        for (g, word) in GROUP_IDS.iter().zip(GROUP_WORDS) {
            assert_eq!(i.tr_now(&group_key(g), &Args::new()), word, "{g}");
        }
    }

    #[test]
    fn every_street_class_has_a_message_in_both_languages() {
        for locale in Locale::ALL {
            let i18n = crate::i18n_for(locale);
            for class in StreetClass::ALL {
                assert!(i18n.has_message(class_key(class), locale), "{} in {}", class_key(class), locale.tag());
            }
        }
        assert_eq!(class_key(StreetClass::Local), "class-local");
    }

    #[test]
    fn the_keys_are_the_ids_with_a_prefix() {
        assert_eq!(kind_key("bikerack"), "kind-bikerack");
        assert_eq!(mode_key(Mode::Vehicle), "mode-vehicle");
        assert_eq!(group_key("walking"), "group-walking");
    }

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
    fn a_pedestrian_street_is_a_class_with_a_word_and_motor_space_can_become_three_kinds() {
        assert_eq!(StreetClass::Pedestrian.id(), "pedestrian");
        assert_eq!(StreetClass::ALL.map(StreetClass::id), ["motorway", "arterial", "collector", "local", "pedestrian"]);
        assert!(!StreetClass::Pedestrian.is_freeway());
        assert_eq!(class_key(StreetClass::Pedestrian), "class-pedestrian");
        assert_eq!(serde_json::to_string(&StreetClass::Pedestrian).unwrap(), "\"pedestrian\"");
        // Any width a motor piece can have (1800 mm for a shoulder to 4000 mm for a lane) is one these kinds allow, so a piece keeps its width when it becomes one.
        for id in PEDESTRIAN_KINDS {
            let k = &KINDS[kind_index(id).unwrap()];
            assert!(!is_motor(kind_index(id).unwrap()));
            assert!(k.min_mm <= 1_800 && k.max_mm >= 4_000, "{id}");
        }
    }

    #[test]
    fn every_material_curb_and_direction_has_a_message_in_both_languages_and_english_says_the_name() {
        for locale in Locale::ALL {
            let i = crate::i18n_for(locale);
            for (key, name) in MATERIALS
                .iter()
                .map(|m| (material_key(m.id), m.name))
                .chain(CURBS.iter().map(|c| (curb_key(c.id), c.name)))
                .chain(DIRECTIONS.iter().map(|d| (direction_key(d.id), d.name)))
            {
                let said = i.tr_now(&key, &Args::new());
                if locale == Locale::En {
                    assert_eq!(said, name, "{key}");
                }
            }
            assert!(i.missing().is_empty(), "{:?}: {:?}", locale, i.missing());
        }
    }
}
