//! The Atlas table is a contract: every measure it says CityLoom models is built, and nothing is built that it
//! does not list. Run by `cargo test`; no screen takes part, so the tabs that list the measures can go.

use std::collections::BTreeSet;

use crate::junction::model::{
    APPROACHES, FILTER_CODE, GATE_SIGNAL, GATE_YIELD, Junction, Q_CURB, Q_OFFSET, Q_VIRTUAL, RULE_AROUND, RULE_DEAD_END, RULE_RIRO, RULE_WITHIN, RULES,
    STOP_BULB, STOP_PLATFORM, STOPS, State,
};
use crate::shared::atlas::{MEASURES, Where, name_key};
use crate::shared::catalogue::StreetClass;
use crate::shared::i18n::{Args, Locale};
use crate::shared::said::say_now;
use crate::shared::units::Units;
use crate::street::measures::DEFS;
use crate::street::model::{Editor, SAMPLES};

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

/// A measure applied on a street that can take it, and that does not show on it yet, is then recognised on it and
/// the street still fits, whichever sample it is.
fn arranged_and_recognised(code: &str) -> bool {
    (0..SAMPLES.len()).any(|sample| {
        let mut e = Editor::new(sample);
        let before = e.view();
        let Some(m) = before.measures.iter().find(|m| m.code == code) else { return false };
        if !m.can_apply || m.present {
            return false;
        }
        let applied = e.apply_measure(code);
        let after = e.view();
        applied && after.delta_mm <= 0 && after.measures.iter().any(|m| m.code == code && m.present)
    })
}

#[test]
fn every_street_measure_can_be_arranged_on_some_sample_street_and_is_then_recognised() {
    for code in listed(Where::Street) {
        assert!(arranged_and_recognised(code), "{code} cannot be arranged, fit and be recognised on any sample street");
    }
}

#[test]
fn the_freeway_measures_are_arranged_on_the_freeway_and_refused_elsewhere() {
    for code in ["B2", "E3"] {
        for (sample, s) in SAMPLES.iter().enumerate() {
            let m = Editor::new(sample).view().measures.into_iter().find(|m| m.code == code).unwrap_or_else(|| panic!("{code} is not offered on {}", s.name));
            let freeway = matches!(s.class, StreetClass::Motorway);
            assert_eq!(m.can_apply, freeway, "{code} on {} (freeway: {freeway})", s.name);
        }
    }
}

fn east_arm(j: &Junction) -> u32 {
    j.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid
}

/// Sets measure `code` on an arm, the way the inspector does (the bus gates need a bus lane first).
fn set(j: &mut Junction, uid: u32, code: &str) -> bool {
    match code {
        "G1" => j.set_approach(uid, Q_OFFSET),
        "G2" => j.set_approach(uid, Q_CURB),
        "G3" => j.set_approach(uid, Q_VIRTUAL),
        "H1" => j.set_bus_lane(uid, true) && j.set_approach(uid, GATE_SIGNAL),
        "H2" => j.set_bus_lane(uid, true) && j.set_approach(uid, GATE_YIELD),
        "M1" => j.set_stop(uid, STOP_BULB),
        "M2" => j.set_stop(uid, STOP_PLATFORM),
        "L1" => j.set_rule(uid, RULE_AROUND),
        "L2" => j.set_rule(uid, RULE_WITHIN),
        "L3" => j.set_rule(uid, RULE_RIRO),
        "L4" => j.set_rule(uid, RULE_DEAD_END),
        "N1" => j.set_filter(uid, true),
        other => panic!("{other} has no way to be set in this test"),
    }
}

/// The east arm's codes once a saved state is loaded as a city's junction.
/// The city supplies the streets' sections when it loads a junction, so they are put back from `streets`.
fn codes_after(saved: &State, streets: &State) -> serde_json::Value {
    let mut state = saved.clone();
    for (arm, street) in state.arms.iter_mut().zip(&streets.arms) {
        arm.section = street.section.clone();
        arm.beyond = street.beyond.clone();
        arm.edge = street.edge;
    }
    let j = Junction::from_city(crate::shared::said::Said::new("city-earlier"), &state, &state, 0).expect("draws");
    let east = j.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid;
    transit(&j, east)["codes"].clone()
}

fn transit(j: &Junction, uid: u32) -> serde_json::Value {
    let v = serde_json::to_value(j.view()).unwrap();
    v["arms"].as_array().unwrap().iter().find(|a| a["uid"] == uid).unwrap()["transit"].clone()
}

#[test]
fn every_junction_measure_can_be_set_is_reported_tagged_and_said_in_the_history() {
    let en = crate::i18n_for(Locale::En);
    let fr = crate::i18n_for(Locale::FrCa);
    for code in listed(Where::Junction) {
        let mut j = Junction::new(0);
        let uid = east_arm(&j);
        assert!(set(&mut j, uid, code), "{code} cannot be set");
        let t = transit(&j, uid);
        assert!(t["codes"].as_array().unwrap().iter().any(|c| c == code), "{code} is not reported on the arm");
        assert!(t["tags"].as_array().unwrap().iter().any(|g| g["code"] == code), "{code} has no tag on the plan");
        let last = j.revisions().last().cloned().expect("the edit is a revision");
        for i in [&en, &fr] {
            let line = say_now(i, Units::Metres, &last);
            assert!(line.contains(code), "{code} is not in the history line {line:?}");
        }
    }
}

#[test]
fn every_junction_measure_survives_a_save_and_a_reload() {
    for code in listed(Where::Junction) {
        let mut j = Junction::new(0);
        let uid = east_arm(&j);
        assert!(set(&mut j, uid, code), "{code} cannot be set");
        let saved = serde_json::to_value(j.current()).unwrap();
        let back: State = serde_json::from_value(saved).expect("the save loads");
        // The label and the streets' sections are not saved (the city keeps them), so the measures are what is compared.
        for (was, now) in j.current().arms.iter().zip(&back.arms) {
            let measures = |a: &crate::junction::model::Arm| (a.uid, a.approach, a.stop, a.rule, a.filter, a.bus_lane);
            assert_eq!(measures(now), measures(was), "{code} changes on the way through storage");
        }
        assert!(
            codes_after(&back, j.current()).as_array().unwrap().iter().any(|c| c == code),
            "{code} is not reported once the save is loaded as a city's junction"
        );
    }
}

#[test]
fn every_measure_is_named_in_both_languages() {
    for locale in [Locale::En, Locale::FrCa] {
        let i = crate::i18n_for(locale);
        for m in MEASURES.iter().filter(|m| m.place != Where::Not) {
            let key = name_key(m.code);
            let name = i.tr_now(&key, &Args::new());
            assert!(!name.is_empty() && name != key, "{locale:?} {}", m.code);
        }
        assert!(i.missing().is_empty(), "{locale:?}: {:?}", i.missing());
    }
}
