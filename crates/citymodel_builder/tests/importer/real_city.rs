//! Real OpenStreetMap data, read and turned into a city, as the app does it.
//! The extracts are Kreuzberg in Berlin (a quarter of a square kilometre) and the Plateau Mont-Royal in
//! Montréal (800 m square), © OpenStreetMap contributors, ODbL.

use cityloom_editor::city::model::City;
use citymodel_builder::importer::{self, import};

const KREUZBERG: &[u8] = include_bytes!("data/kreuzberg.osm");
const PLATEAU: &[u8] = include_bytes!("data/plateau.osm");

#[test]
fn a_real_neighbourhood_becomes_a_city_whose_places_can_all_be_opened() {
    let net = import(KREUZBERG).expect("the extract reads");
    assert!(net.roads.len() > 50, "{} roads", net.roads.len());
    assert!(!net.left_hand);
    let city = City::from_network(&net, "Kreuzberg");
    let v = city.view(0);
    assert!(v.nodes.iter().filter(|n| n.junction).count() > 5, "half of the extract's roads have no name and are left out");
    for n in v.nodes.iter().filter(|n| n.junction) {
        assert!(city.junction_editor(n.uid, 0).is_some(), "{:?} cannot be opened", n.name);
    }
    for e in &v.edges {
        assert!(city.street_editor(e.uid, 0).is_some(), "{:?} cannot be opened", e.name());
        assert!(e.row_mm > 0 && e.length_mm > 0, "{:?}: row {} length {}", e.name(), e.row_mm, e.length_mm);
    }
}

#[test]
fn a_montreal_neighbourhood_opens_and_most_of_its_meetings_of_streets_are_junctions() {
    let net = import(PLATEAU).expect("the extract reads");
    assert!(!net.left_hand);
    assert!(net.roads.len() > 150, "{} roads", net.roads.len());
    let mut degree = std::collections::HashMap::new();
    for r in net.roads.iter() {
        *degree.entry(r.edge.from).or_insert(0) += 1;
        *degree.entry(r.edge.to).or_insert(0) += 1;
    }
    let candidates = net.junctions.iter().filter(|n| n.junction && (3..=5).contains(&degree.get(&n.id).copied().unwrap_or(0))).count();
    let city = City::from_network(&net, "Plateau");
    let v = city.view(0);
    let junctions: Vec<_> = v.nodes.iter().filter(|n| n.junction).collect();
    // a meeting the junction editor cannot draw is a plain connection; that is the exception
    assert!(junctions.len() * 100 >= candidates * 85, "{} of {candidates} meetings are junctions", junctions.len());
    for n in junctions {
        assert!(city.junction_editor(n.uid, 0).is_some(), "{:?} cannot be opened", n.name);
    }
    for e in &v.edges {
        assert!(city.street_editor(e.uid, 0).is_some(), "{:?} cannot be opened", e.name());
    }
    // only a bike lane runs both ways. (The extract holds the roads only, no `highway=cycleway` ways and no
    // `oneway=no` on a side, so it has no two-way track to count; `tests/import.rs` pins them on fixtures.)
    assert!(net.roads.iter().flat_map(|r| r.base_lanes()).filter(|l| l.direction == importer::Direction::Both).all(|l| l.kind == importer::LaneKind::Bike));
    // the streets with a median in them
    assert!(net.roads.iter().any(|r| r.base_lanes().iter().any(|l| l.kind == importer::LaneKind::Other)), "a divided street is merged");
}

#[test]
fn no_street_of_the_montreal_extract_has_a_median_that_came_from_a_buffer() {
    // Every buffer of the extract is a curb that osm2streets inferred. One beside a bike lane is that lane's own
    // curb; any other is the neutral buffer. A median is only what lies between two merged carriageways.
    let net = import(PLATEAU).expect("the extract reads");
    let city = City::from_network(&net, "Plateau");
    let kind = |id| cityloom_editor::shared::catalogue::kind_index(id).unwrap();
    let (median, buffer) = (kind("median"), kind("buffer"));
    let (mut medians, mut buffers) = (0, 0);
    for e in &city.view(0).edges {
        let view = city.street_editor(e.uid, 0).unwrap().view();
        medians += view.segments.iter().filter(|s| s.kind == median).count();
        buffers += view.segments.iter().filter(|s| s.kind == buffer).count();
    }
    let others = net.roads.iter().flat_map(|r| r.base_lanes()).filter(|l| l.kind == importer::LaneKind::Other).count();
    let curbs = net.roads.iter().flat_map(|r| r.base_lanes()).filter(|l| l.kind == importer::LaneKind::Buffer).count();
    assert_eq!(medians, others, "a median is a lane of the road that no lane uses, and nothing else");
    assert!(buffers > 0 && buffers <= curbs, "{buffers} buffers of {curbs} curbs: those beside a bike lane or a sidewalk are its own");
}

#[test]
fn every_street_of_the_montreal_extract_that_changed_width_with_the_buffers_is_found_by_the_width_it_had() {
    // What importer version 4 made of the extract: every buffer that is not planted a lane of its own, a median.
    use importer::{BufferType, LaneKind};
    let net = import(PLATEAU).expect("the extract reads");
    let mut old = net.clone();
    for l in old.roads.iter_mut().flat_map(|r| r.base_lanes_mut().iter_mut()) {
        if l.kind == LaneKind::Buffer && !matches!(l.buffer_type, Some(BufferType::Planters | BufferType::Verge)) {
            (l.kind, l.buffer_type) = (LaneKind::Other, None);
        }
    }
    let (before, mut now) = (City::from_network(&old, "Plateau"), City::from_network(&net, "Plateau"));
    let edges = before.view(0).edges;
    assert_eq!(edges.len(), now.view(0).edges.len());
    let (mut changed, mut found) = (0, 0);
    for e in &edges {
        let saved = before.street_editor(e.uid, 0).unwrap().snapshot();
        if now.street_editor(e.uid, 0).unwrap().snapshot().row_mm == saved.row_mm {
            continue;
        }
        changed += 1;
        found += usize::from(now.keep_street(e.uid, saved));
    }
    println!("{found} of {changed} streets that changed width are found by the width they had");
    assert!(changed > 100, "{changed}");
    assert_eq!(found, changed);
}

#[test]
fn no_street_of_the_montreal_extract_has_planting_the_data_does_not_show() {
    // Every buffer of the extract is a curb that osm2streets inferred; none is a planter or a verge.
    let net = import(PLATEAU).expect("the extract reads");
    let buffers = net.roads.iter().flat_map(|r| r.base_lanes()).filter(|l| l.kind == importer::LaneKind::Buffer).count();
    assert!(buffers > 100, "{buffers} buffers");
    let city = City::from_network(&net, "Plateau");
    let planting = cityloom_editor::shared::catalogue::kind_index("planting").unwrap();
    for e in &city.view(0).edges {
        let view = city.street_editor(e.uid, 0).unwrap().view();
        assert!(view.segments.iter().all(|s| s.kind != planting), "{:?} has planting", e.name());
    }
}

// ---- edits kept on real data -------------------------------------------------------------------------------------
//
// The e2e fixture is the Plateau's metro tile: the default area and areas shifted a little from it, as the app cuts
// them, each read through the real importer and kept in the same storage.

use crate::common;

mod kept {
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use cityloom_editor::city::store::CityStore;
    use cityloom_editor::place::area::{Area, default_area};
    use cityloom_editor::shared::ports::MemoryStorage;
    use citymodel_builder::importer;

    use super::common::{Follow, document, edit_everything, followed, shifted_from, store_with};

    const TILE: &[u8] = include_bytes!("../../../../e2e/fixtures/plateau.osm.pbf");

    fn store(storage: &Rc<MemoryStorage>, area: Area) -> CityStore {
        let b = area.bounds();
        let network = importer::import_in(TILE, Some([b.south, b.west, b.north, b.east])).expect("the tile reads");
        store_with(storage, area, &network)
    }

    /// The default area moved `dx_m` metres east and `dy_m` north.
    fn shifted(dx_m: f64, dy_m: f64) -> Area {
        shifted_from(&default_area(), "shifted", dx_m, dy_m)
    }

    #[test]
    fn every_place_of_the_default_area_has_a_key_no_other_place_shares() {
        let storage = Rc::new(MemoryStorage::default());
        let city = store(&storage, default_area()).open();
        let view = city.view(0);
        let mut keys: BTreeMap<String, usize> = BTreeMap::new();
        let mut keyless = Vec::new();
        for e in &view.edges {
            match city.street_record(e.uid) {
                Some((key, _)) => *keys.entry(key).or_default() += 1,
                None => keyless.push(format!("s{}", e.uid)),
            }
        }
        for n in view.nodes.iter().filter(|n| n.junction) {
            match city.junction_record(n.uid) {
                Some((key, _)) => *keys.entry(key).or_default() += 1,
                None => keyless.push(format!("j{}", n.uid)),
            }
        }
        let shared: Vec<_> = keys.iter().filter(|(_, n)| **n > 1).collect();
        assert_eq!(shared, Vec::<(&String, &usize)>::new(), "keys held by more than one place");
        assert_eq!(keyless, Vec::<String>::new(), "places with no record to keep");
    }

    #[test]
    fn every_street_and_junction_of_the_default_area_edited_and_opened_again_keeps_its_edit() {
        let storage = Rc::new(MemoryStorage::default());
        let (streets, junctions, _) = edit_everything(&store(&storage, default_area()), &storage);
        println!("default area: streets {streets:?}, junctions {junctions:?}");
        assert_eq!((streets.lost, streets.refused), (0, 0), "streets: {streets:?}");
        assert_eq!((junctions.lost, junctions.refused), (0, 0), "junctions: {junctions:?}");
    }

    #[test]
    fn edits_follow_their_places_into_shifted_areas_and_never_onto_another_place() {
        let storage = Rc::new(MemoryStorage::default());
        let (.., placed) = edit_everything(&store(&storage, default_area()), &storage);
        let edits = document(&storage);
        let shifts: Vec<((f64, f64), [Follow; 3])> = [(150.0, 0.0), (-150.0, 0.0), (0.0, 150.0), (0.0, -150.0)]
            .into_iter()
            .map(|(dx, dy)| ((dx, dy), followed(&store(&storage, shifted(dx, dy)), &edits, &placed).0))
            .collect();
        for ((dx, dy), [junctions, cut, other]) in &shifts {
            println!(
                "shifted ({dx}, {dy}) m (followed, of): junctions {:?} overlapping, {:?} the same, took {} wrong {}; cut streets {:?}, {:?}, took {} wrong {} (rivals told apart {:?}); other streets {:?}, {:?}, took {} wrong {} (rivals told apart {:?})",
                junctions.overlapping,
                junctions.same,
                junctions.took,
                junctions.wrong,
                cut.overlapping,
                cut.same,
                cut.took,
                cut.wrong,
                cut.rivals,
                other.overlapping,
                other.same,
                other.took,
                other.wrong,
                other.rivals
            );
        }
        for (shift, kinds) in shifts {
            for (what, f) in ["junctions", "cut streets", "other streets"].into_iter().zip(kinds) {
                assert_eq!(f.wrong, 0, "{shift:?}: {what} that took the edit of another place: {f:?}");
                assert!(f.rivals.0 * 100 >= f.rivals.1 * 90, "{shift:?}: the check tells the other pieces of a way apart: {f:?}");
                assert!(f.same.0 * 100 >= f.same.1 * 95, "{shift:?}: {what} the importer made the same in both areas: {f:?}");
                assert!(f.overlapping.0 * 100 >= f.overlapping.1 * 90, "{shift:?}: {what} {f:?}");
            }
        }
    }
}

mod two_way_track {
    //! A street edited while its track was read as one-way (before the ways of a track were fetched) is still found
    //! when the track is read as two-way: De Maisonneuve Ouest near Rue Saint-Marc.

    use std::rc::Rc;

    use cityloom_editor::city::store::CityStore;
    use cityloom_editor::place::area::Area;
    use cityloom_editor::shared::ports::{MemoryStorage, Storage};
    use citymodel_builder::importer;

    const FIXTURE: &str = include_str!("de_maisonneuve.osm");

    /// The fixture without the ways of its cycleways: what production read before they were fetched.
    fn without_tracks() -> String {
        FIXTURE.lines().filter(|l| !l.contains("v=\"cycleway\"")).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn a_record_made_with_the_track_one_way_applies_to_the_street_when_it_is_two_way() {
        let before = importer::import(without_tracks().as_bytes()).unwrap();
        let now = importer::import(FIXTURE.as_bytes()).unwrap();
        let storage = Rc::new(MemoryStorage::default());
        let store = CityStore::for_area(storage.clone() as Rc<dyn Storage>, Area::new("Saint-Marc", 45.4935, -73.58));
        assert!(store.keep_network(&before));
        // way 330153470's street
        let index = before
            .roads
            .iter()
            .position(|r| r.has_way(330153470))
            .unwrap_or_else(|| panic!("no street: {:?}", before.roads.iter().map(|r| &r.provenance).collect::<Vec<_>>()));
        let view = store.open().view(0);
        let (uid, one_way_mm) = (view.edges[index].uid, view.edges[index].row_mm);
        assert_eq!(one_way_mm, 10_700, "as De Maisonneuve was imported until now");
        let mut edited = store.open().street_editor(uid, 0).unwrap().snapshot();
        edited.next_uid += 1;
        assert!(store.write_street(uid, edited.clone()));

        assert!(store.keep_network(&now));
        let city = store.open();
        let view = city.view(0);
        let index = now.roads.iter().position(|r| r.has_way(330153470)).unwrap();
        let bike = now.roads[index].base_lanes().iter().find(|l| l.kind == importer::LaneKind::Bike).unwrap();
        assert_eq!((bike.direction, bike.width_m), (importer::Direction::Both, 3.0), "the street is two-way now");
        assert_eq!(view.edges[index].row_mm, one_way_mm, "and is shown as it was saved");
        assert!(view.edges[index].edited, "and the record made at {one_way_mm} mm still applies");
        assert_eq!(city.street_editor(view.edges[index].uid, 0).unwrap().snapshot(), edited);
    }
}
