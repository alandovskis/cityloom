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

/// The groups the add menu sorts the kinds into, in its order. A group's name is the message `group-<id>`.
pub const GROUP_IDS: [&str; 7] = ["walking", "greenery", "cycling", "transit", "roadway", "furniture", "utilities"];

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
            for m in Mode::ALL {
                i.tr_now(mode_key(m), &Args::new());
            }
            for g in GROUP_IDS {
                i.tr_now(&group_key(g), &Args::new());
            }
            assert!(i.missing().is_empty(), "{:?}: {:?}", locale, i.missing());
        }
    }

    #[test]
    fn the_english_messages_say_what_the_names_and_labels_still_say() {
        let i = crate::i18n_for(Locale::En);
        for k in KINDS.iter() {
            assert_eq!(i.tr_now(&kind_key(k.id), &Args::new()), k.name, "{}", k.id);
        }
        for k in KINDS.iter() {
            assert_eq!(i.tr_now(&mark_key(k.id), &Args::new()), k.mark, "{}", k.id);
        }
        for m in Mode::ALL {
            assert_eq!(i.tr_now(mode_key(m), &Args::new()), m.label(), "{m:?}");
        }
        for (g, word) in GROUP_IDS.iter().zip(GROUP_WORDS) {
            assert_eq!(i.tr_now(&group_key(g), &Args::new()), word, "{g}");
        }
    }

    #[test]
    fn the_group_ids_are_those_of_the_add_menu() {
        let menu: Vec<&str> = crate::street::page::ADD_GROUPS.iter().map(|(id, _)| *id).collect();
        assert_eq!(menu, GROUP_IDS);
    }

    #[test]
    fn the_keys_are_the_ids_with_a_prefix() {
        assert_eq!(kind_key("bikerack"), "kind-bikerack");
        assert_eq!(mode_key(Mode::Vehicle), "mode-vehicle");
        assert_eq!(group_key("walking"), "group-walking");
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
