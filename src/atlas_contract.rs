//! The Atlas table is a contract: every measure it says CityLoom models is built, and nothing is built that it
//! does not list. Run by `cargo test`; no screen takes part, so the tabs that list the measures can go.

use std::collections::BTreeSet;

use crate::junction::model::{APPROACHES, FILTER_CODE, RULES, STOPS};
use crate::shared::atlas::{MEASURES, Where};
use crate::street::measures::DEFS;

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
