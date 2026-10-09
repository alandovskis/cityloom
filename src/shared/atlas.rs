//! The measures of the Transit Priority Atlas toolbox
//! (https://tpa.transitcosts.com/atlas/toolbox) and where CityLoom models each.
//! Codes and names follow the Atlas. What each measure does here is
//! CityLoom's own placeholder, not the Atlas's guidance.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Where {
    /// A lane arrangement in the street cross-section editor.
    Street,
    /// A feature of one approach in the intersection editor.
    Junction,
    /// Not modelled, with the reason in `note`.
    Not,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Measure {
    pub code: &'static str,
    pub name: &'static str,
    pub family: &'static str,
    pub group: &'static str,
    pub place: Where,
    pub note: &'static str,
}

pub const LINEAR: &str = "Linear continuous measures";
pub const LOCAL: &str = "Localized measures";
pub const AREA: &str = "Area-wide measures";

/// The message that names a measure (`street/i18n`), by its lowercase code.
pub fn name_key(code: &str) -> String {
    format!("atlas-{}-name", code.to_lowercase())
}

/// The message that says why a measure is not modelled; only those with a `note` have one.
pub fn note_key(code: &str) -> String {
    format!("atlas-{}-note", code.to_lowercase())
}

const fn m(code: &'static str, name: &'static str, family: &'static str, group: &'static str, place: Where, note: &'static str) -> Measure {
    Measure { code, name, family, group, place, note }
}

pub const MEASURES: [Measure; 30] = [
    m("A1", "Transit Streets", "A Transit-priority streets", LINEAR, Where::Street, ""),
    m("A2", "Transit Ways", "A Transit-priority streets", LINEAR, Where::Street, ""),
    m("A3", "Transit and Direct Access Streets", "A Transit-priority streets", LINEAR, Where::Street, ""),
    m("B1", "Center-Running Transit Lanes", "B Center-running transit lanes", LINEAR, Where::Street, ""),
    m("B2", "Center-Running Transit Lanes on Freeway Medians", "B Center-running transit lanes", LINEAR, Where::Street, ""),
    m("B3", "Static Alternate-Direction Center-Running Transit Lanes", "B Center-running transit lanes", LINEAR, Where::Street, ""),
    m("B4", "Dynamic Alternate-Direction Center-Running Transit Lanes", "B Center-running transit lanes", LINEAR, Where::Street, ""),
    m("C1", "Edge-Running Bidirectional Transit Lanes", "C Edge-running bidirectional transit lanes", LINEAR, Where::Street, ""),
    m("D1", "Offset Transit Lanes", "D Offset transit lanes", LINEAR, Where::Street, ""),
    m("E1", "Curb-Adjacent Transit Lanes", "E Curbside transit lanes", LINEAR, Where::Street, ""),
    m("E2", "Curb-Adjacent Reversible Parking and Transit Lanes", "E Curbside transit lanes", LINEAR, Where::Street, ""),
    m("E3", "Transit Lanes on Freeway Shoulders", "E Curbside transit lanes", LINEAR, Where::Street, ""),
    m("F1", "Contraflow Transit Lanes", "F Contraflow transit lanes", LINEAR, Where::Street, ""),
    m("F2", "Offset Contraflow Transit Lanes", "F Contraflow transit lanes", LINEAR, Where::Street, ""),
    m("G1", "Offset Queue-Jump Lanes", "G Queue jumps", LINEAR, Where::Junction, ""),
    m("G2", "Curbside Queue-Jump Lanes", "G Queue jumps", LINEAR, Where::Junction, ""),
    m("G3", "Virtual Queue-Jump Lane", "G Queue jumps", LINEAR, Where::Junction, ""),
    m("H1", "Signal-Controlled Bus Gates", "H Pre-signals and pre-yields", LOCAL, Where::Junction, ""),
    m("H2", "Yield-Controlled Bus Gates", "H Pre-signals and pre-yields", LOCAL, Where::Junction, ""),
    m("L1", "Indirect Left Turn via Alternative Itinerary", "L Turn conflicts management", LOCAL, Where::Junction, ""),
    m("L2", "Indirect Left Turn within the Intersection", "L Turn conflicts management", LOCAL, Where::Junction, ""),
    m("L3", "Right-In/Right-Out", "L Turn conflicts management", LOCAL, Where::Junction, ""),
    m("L4", "Dead-Ending of Lateral Streets", "L Turn conflicts management", LOCAL, Where::Junction, ""),
    m("M1", "Bus Bulbs", "M Bus stop related", LOCAL, Where::Junction, ""),
    m("M2", "Signal-Protected On-Street Platforms", "M Bus stop related", LOCAL, Where::Junction, ""),
    m("N1", "Transit Modal Filter", "N Transit modal filters", LOCAL, Where::Junction, ""),
    m(
        "TSP",
        "Transit Signal Priority",
        "TSP Transit signal priority",
        LOCAL,
        Where::Not,
        "The Atlas lists it as under development, and CityLoom has no signal timing to prioritise.",
    ),
    m(
        "Z1",
        "Limited Traffic Areas",
        "Z Areas with access restrictions",
        AREA,
        Where::Not,
        "An area of many streets, and CityLoom edits one street or one junction.",
    ),
    m(
        "W-D",
        "Dynamic Congestion Pricing",
        "W Congestion and road pricing",
        AREA,
        Where::Not,
        "The Atlas lists it as in development, and a price needs a city and demand to act on.",
    ),
    m(
        "W-F",
        "Fixed-Fee Road Pricing",
        "W Congestion and road pricing",
        AREA,
        Where::Not,
        "The Atlas lists it as in development, and a price needs a city and demand to act on.",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_listed_once() {
        let mut codes: Vec<_> = MEASURES.iter().map(|m| m.code).collect();
        codes.sort_unstable();
        let n = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), n);
    }

    #[test]
    fn what_cannot_be_modelled_says_why() {
        for m in MEASURES.iter().filter(|m| m.place == Where::Not) {
            assert!(!m.note.is_empty(), "{}", m.code);
        }
    }

    #[test]
    fn every_measure_is_named_and_every_note_is_said_in_both_languages() {
        use crate::shared::i18n::{Args, Locale};
        for locale in [Locale::En, Locale::FrCa] {
            let i = crate::i18n_for(locale);
            for m in MEASURES.iter() {
                let name = i.tr_now(&name_key(m.code), &Args::new());
                assert!(!name.is_empty(), "{}", m.code);
                if locale == Locale::En {
                    assert_eq!(name, m.name, "{}", m.code);
                }
                if !m.note.is_empty() {
                    let note = i.tr_now(&note_key(m.code), &Args::new());
                    if locale == Locale::En {
                        assert_eq!(note, m.note, "{}", m.code);
                    }
                }
            }
            assert!(i.missing().is_empty(), "{:?}: {:?}", locale, i.missing());
        }
    }
}
