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
        assert!(city.junction_editor(n.uid, 0).is_some(), "{:?} cannot be opened", n.name);
    }
    for e in &v.edges {
        assert!(city.street_editor(e.uid, 0).is_some(), "{:?} cannot be opened", e.name());
    }
    // the streets with a median in them
    assert!(net.roads.iter().any(|r| r.lanes.iter().any(|l| l.kind == osm_import::LaneKind::Other)), "a divided street is merged");
}

// ---- edits kept on real data -------------------------------------------------------------------------------------
//
// The e2e fixture is the Plateau's metro tile: the default area and areas shifted a little from it, as the app cuts
// them, each read through the real importer and kept in the same storage.

mod kept {
    use std::collections::{BTreeMap, BTreeSet};
    use std::rc::Rc;

    use cityloom_editor::city::edits::{self, Document, EDITS_KEY, Edits};
    use cityloom_editor::city::store::CityStore;
    use cityloom_editor::place::area::{Area, default_area};
    use cityloom_editor::shared::catalogue::KINDS;
    use cityloom_editor::shared::ports::{MemoryStorage, Storage};

    const TILE: &[u8] = include_bytes!("../../e2e/fixtures/plateau.osm.pbf");

    fn store(storage: &Rc<MemoryStorage>, area: Area) -> CityStore {
        let b = area.bounds();
        let network = osm_import::import_in(TILE, Some([b.south, b.west, b.north, b.east])).expect("the tile reads");
        let store = CityStore::for_area(storage.clone() as Rc<dyn Storage>, area);
        assert!(store.keep_network(&network));
        store
    }

    /// The default area moved `dx_m` metres east.
    fn shifted(dx_m: f64) -> Area {
        let a = default_area();
        Area::new("shifted", a.lat, a.lon + dx_m / (111_320.0 * a.lat.to_radians().cos()))
    }

    fn document(storage: &MemoryStorage) -> Edits {
        match edits::parse(storage.recall(EDITS_KEY).as_deref()) {
            Document::Current(e) => e,
            _ => Edits::new(),
        }
    }

    #[derive(Debug, Default, PartialEq)]
    struct Tally {
        back: usize,
        /// Written (the store said it was kept) but as first laid out when the city is opened again.
        lost: usize,
        refused: usize,
    }

    /// Edits every street and junction of the area and keeps each as the store's writes do (the place's record
    /// under its key, every other record left as it is), then opens the city again. One open for all the writes:
    /// a write opens the whole city, which takes about a second in a debug build.
    fn edit_everything(store: &CityStore, storage: &MemoryStorage) -> (Tally, Tally) {
        let mut city = store.open();
        let view = city.view(0);
        let mut edits = document(storage);
        let (mut streets, mut junctions) = (Tally::default(), Tally::default());
        let mut written = (Vec::new(), Vec::new());
        for e in &view.edges {
            let mut s = city.street_editor(e.uid, 0).expect("every street opens").snapshot();
            if s.segments.len() >= 2 {
                s.segments[0].width_mm -= 100;
                s.segments[1].width_mm += 100;
            } else {
                // a street of one piece gets another surface
                let piece = &mut s.segments[0];
                let other = KINDS[piece.kind].materials.iter().copied().find(|&m| m != piece.material).expect("a kind with a second surface");
                piece.material = other;
            }
            assert!(city.keep_street(e.uid, s));
            match city.street_record(e.uid) {
                Some((key, record)) => {
                    edits.streets.insert(key, record);
                    written.0.push(e.uid);
                }
                None => streets.refused += 1,
            }
        }
        for n in view.nodes.iter().filter(|n| n.junction) {
            let mut j = city.junction_editor(n.uid, 0).expect("every junction opens");
            // a corner rounded otherwise: the first arm and radius the junction allows
            let arms: Vec<(u32, i32)> = j.current().arms.iter().map(|a| (a.uid, a.corner_mm)).collect();
            let changed = arms.iter().any(|&(uid, now)| [7_000, 6_000, 5_000, 8_000, 3_000].into_iter().any(|mm| mm != now && j.set_corner(uid, mm)));
            assert!(changed, "no corner of {} can be changed", n.uid);
            assert!(city.keep_junction(n.uid, j.snapshot()));
            match city.junction_record(n.uid) {
                Some((key, record)) => {
                    edits.junctions.insert(key, record);
                    written.1.push(n.uid);
                }
                None => junctions.refused += 1,
            }
        }
        assert!(storage.remember(EDITS_KEY, &edits.to_json()));
        let back = store.open().view(0);
        for uid in written.0 {
            let tally = &mut streets;
            if back.edges.iter().any(|x| x.uid == uid && x.edited) { tally.back += 1 } else { tally.lost += 1 }
        }
        for uid in written.1 {
            let tally = &mut junctions;
            if back.nodes.iter().any(|x| x.uid == uid && x.edited) { tally.back += 1 } else { tally.lost += 1 }
        }
        (streets, junctions)
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
        let (streets, junctions) = edit_everything(&store(&storage, default_area()), &storage);
        println!("default area: streets {streets:?}, junctions {junctions:?}");
        assert_eq!((streets.lost, streets.refused), (0, 0), "streets: {streets:?}");
        assert_eq!((junctions.lost, junctions.refused), (0, 0), "junctions: {junctions:?}");
    }

    /// How many places of an area show an edit made in the default area: of those made from the same
    /// OpenStreetMap ids as an edited place there, and of those the importer made the same in both areas.
    #[derive(Debug, Default, Clone, Copy)]
    struct Follow {
        /// Of the places that overlap an edited place in its ids, how many show an edit.
        overlapping: (usize, usize),
        /// Of the places that are the same in both areas (a street: the same ways and the same width; a junction:
        /// the same nodes and as many arms), how many show the edit.
        same: (usize, usize),
    }

    impl Follow {
        fn count(&mut self, edited: bool, same: bool) {
            self.overlapping.1 += 1;
            self.overlapping.0 += usize::from(edited);
            if same {
                self.same.1 += 1;
                self.same.0 += usize::from(edited);
            }
        }
    }

    /// (junctions, cut streets, other streets).
    fn followed(storage: &Rc<MemoryStorage>, area: Area, edits: &Edits) -> [Follow; 3] {
        let city = store(storage, area).open();
        let view = city.view(0);
        let mut out = [Follow::default(); 3];
        for n in view.nodes.iter().filter(|n| n.junction) {
            let Some((_, r)) = city.junction_record(n.uid) else { continue };
            let from: Vec<_> = edits.junctions.values().filter(|j| !j.nodes.is_disjoint(&r.nodes)).collect();
            if !from.is_empty() {
                let same = from.iter().any(|j| j.nodes == r.nodes && j.arm_streets.len() == r.arm_streets.len());
                out[0].count(n.edited, same);
            }
        }
        for e in &view.edges {
            let Some((key, r)) = city.street_record(e.uid) else { continue };
            let from: Vec<(&String, _)> = edits.streets.iter().filter(|(_, s)| !s.ways.is_disjoint(&r.ways) && !s.nodes.is_disjoint(&r.nodes)).collect();
            if from.is_empty() {
                continue;
            }
            let cut = key.contains("~x") || from.iter().any(|(k, _)| k.contains("~x"));
            let same = from.iter().any(|(_, s)| s.ways == r.ways && s.street.row_mm == r.street.row_mm && s.street.class == r.street.class);
            out[if cut { 1 } else { 2 }].count(e.edited, same);
        }
        out
    }

    #[test]
    fn edits_follow_their_places_into_areas_shifted_east_and_west() {
        let storage = Rc::new(MemoryStorage::default());
        edit_everything(&store(&storage, default_area()), &storage);
        let edits = document(&storage);
        let ids: BTreeSet<i64> = edits.streets.values().flat_map(|s| s.ways.iter().copied()).collect();
        assert!(!ids.is_empty());
        let shifts: Vec<(f64, [Follow; 3])> = [150.0, -150.0].into_iter().map(|dx| (dx, followed(&storage, shifted(dx), &edits))).collect();
        for (dx, [junctions, cut, other]) in &shifts {
            println!(
                "shifted {dx} m (followed, of): junctions {:?} overlapping, {:?} the same; cut streets {:?}, {:?}; other streets {:?}, {:?}",
                junctions.overlapping, junctions.same, cut.overlapping, cut.same, other.overlapping, other.same
            );
        }
        for (dx, kinds) in shifts {
            for (what, f) in ["junctions", "cut streets", "other streets"].into_iter().zip(kinds) {
                assert_eq!(f.same.0, f.same.1, "{dx} m: every one of the {what} the importer made the same in both areas follows: {f:?}");
                assert!(f.overlapping.0 * 100 >= f.overlapping.1 * 95, "{dx} m: {what} {f:?}");
            }
        }
    }
}
