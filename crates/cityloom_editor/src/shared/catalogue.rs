//! The catalogue of `cityloom_model`, and what the editor adds to it: the regions, the groups of the add menu and the
//! keys of the messages that name its entries.

pub use cityloom_model::catalogue::*;
use serde::Serialize;

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

/// What the space of a street's motor pieces can become when the street is made pedestrian, by kind id.
pub const PEDESTRIAN_KINDS: [&str; 3] = ["sidewalk", "planting", "terrace"];

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
    fn the_english_messages_say_what_the_groups_still_say() {
        let i = crate::i18n_for(Locale::En);
        assert_eq!(i.tr_now(&kind_key("sidewalk"), &Args::new()), "Sidewalk");
        assert_eq!(i.tr_now(mode_key(Mode::Vehicle), &Args::new()), "Cars and trucks");
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
