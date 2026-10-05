//! Real OpenStreetMap data, read and turned into a city, as the app does it.
//! The extracts are Kreuzberg in Berlin (a quarter of a square kilometre) and the Plateau Mont-Royal in
//! Montréal (800 m square), © OpenStreetMap contributors, ODbL.

use cityloom_editor::city::model::City;
use osm_import::import;

const KREUZBERG: &[u8] = include_bytes!("data/kreuzberg.osm");
const PLATEAU: &[u8] = include_bytes!("data/plateau.osm");

#[test]
fn a_real_neighbourhood_becomes_a_city_whose_places_can_all_be_opened() {
    let net = import(KREUZBERG).expect("the extract reads");
    assert!(net.roads.len() > 50, "{} roads", net.roads.len());
    assert!(!net.left_hand);
    let city = City::from_network(&net, "Kreuzberg");
    let v = city.view(0);
    assert!(v.nodes.iter().filter(|n| n.junction).count() > 10);
    for n in v.nodes.iter().filter(|n| n.junction) {
        assert!(city.junction_editor(n.uid, 0).is_some(), "{} cannot be opened", n.name);
    }
    for e in &v.edges {
        assert!(city.street_editor(e.uid, 0).is_some(), "{} cannot be opened", e.name);
        assert!(e.row_mm > 0 && e.length_mm > 0, "{}: row {} length {}", e.name, e.row_mm, e.length_mm);
    }
}

#[test]
fn a_montreal_neighbourhood_opens_and_most_of_its_meetings_of_streets_are_junctions() {
    let net = import(PLATEAU).expect("the extract reads");
    assert!(!net.left_hand);
    assert!(net.roads.len() > 150, "{} roads", net.roads.len());
    let mut degree = std::collections::HashMap::new();
    for r in &net.roads {
        *degree.entry(r.from).or_insert(0) += 1;
        *degree.entry(r.to).or_insert(0) += 1;
    }
    let candidates = net.nodes.iter().filter(|n| n.junction && (3..=5).contains(&degree.get(&n.id).copied().unwrap_or(0))).count();
    let city = City::from_network(&net, "Plateau");
    let v = city.view(0);
    let junctions: Vec<_> = v.nodes.iter().filter(|n| n.junction).collect();
    // a meeting the junction editor cannot draw is a plain connection; that is the exception
    assert!(junctions.len() * 100 >= candidates * 85, "{} of {candidates} meetings are junctions", junctions.len());
    for n in junctions {
        assert!(city.junction_editor(n.uid, 0).is_some(), "{} cannot be opened", n.name);
    }
    for e in &v.edges {
        assert!(city.street_editor(e.uid, 0).is_some(), "{} cannot be opened", e.name);
    }
    // the streets with a median in them
    assert!(net.roads.iter().any(|r| r.lanes.iter().any(|l| l.kind == osm_import::LaneKind::Other)), "a divided street is merged");
}
