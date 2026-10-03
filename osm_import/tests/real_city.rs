//! Real OpenStreetMap data, read and turned into a city, as the app does it.
//! The extract is a quarter of a square kilometre of Kreuzberg, Berlin
//! (© OpenStreetMap contributors, ODbL).

use cityloom_editor::city::model::City;
use osm_import::import;

const KREUZBERG: &[u8] = include_bytes!("data/kreuzberg.osm");

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
