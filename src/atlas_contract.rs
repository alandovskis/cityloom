//! The Atlas table is a contract: every measure it says CityLoom models is built, and nothing is built that it
//! does not list. Run by `cargo test`; no screen takes part, so the tabs that list the measures can go.

use std::collections::BTreeSet;

use crate::junction::model::{APPROACHES, FILTER_CODE, RULES, STOPS};
use crate::shared::atlas::{MEASURES, Where};
use crate::street::measures::DEFS;
use crate::street::model::Editor;

fn listed(place: Where) -> BTreeSet<&'static str> {
    MEASURES.iter().filter(|m| m.place == place).map(|m| m.code).collect()
}

#[test]
fn the_street_editor_models_exactly_the_street_measures_of_the_table() {
    let built: BTreeSet<&str> = DEFS.iter().map(|d| d.code).collect();
    assert_eq!(built, listed(Where::Street));
}

#[test]
fn the_junction_editor_models_exactly_the_junction_measures_of_the_table() {
    let built: BTreeSet<&str> = APPROACHES.iter().chain(&STOPS).chain(&RULES).map(|i| i.code).filter(|c| !c.is_empty()).chain([FILTER_CODE]).collect();
    assert_eq!(built, listed(Where::Junction));
}

/// A measure applied on a street that can take it is then recognised on it, whichever sample it is.
fn arranged_and_recognised(code: &str) -> bool {
    (0..4).any(|sample| {
        let mut e = Editor::new(sample);
        let offered = e.view().measures.iter().any(|m| m.code == code && m.can_apply);
        offered && e.apply_measure(code) && e.view().measures.iter().any(|m| m.code == code && m.present)
    })
}

#[test]
fn every_street_measure_can_be_arranged_on_some_sample_street_and_is_then_recognised() {
    for code in listed(Where::Street) {
        assert!(arranged_and_recognised(code), "{code} cannot be arranged and recognised on any sample street");
    }
}

#[test]
fn the_freeway_measures_are_arranged_on_the_freeway_and_refused_elsewhere() {
    for code in ["B2", "E3"] {
        assert!(!Editor::new(1).view().measures.iter().find(|m| m.code == code).unwrap().can_apply, "{code} off a freeway");
        assert!(Editor::new(3).view().measures.iter().find(|m| m.code == code).unwrap().can_apply, "{code} on a freeway");
    }
}
