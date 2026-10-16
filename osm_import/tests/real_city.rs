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
    for r in net.roads.iter().filter(|r| r.name.as_deref().is_some_and(|n| !n.trim().is_empty())) {
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
    // only a bike lane runs both ways. (The extract holds the roads only, no `highway=cycleway` ways and no
    // `oneway=no` on a side, so it has no two-way track to count; `tests/import.rs` pins them on fixtures.)
    assert!(net.roads.iter().flat_map(|r| &r.lanes).filter(|l| l.way == osm_import::Way::Both).all(|l| l.kind == osm_import::LaneKind::Bike));
    // the streets with a median in them
    assert!(net.roads.iter().any(|r| r.lanes.iter().any(|l| l.kind == osm_import::LaneKind::Other)), "a divided street is merged");
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
    let others = net.roads.iter().flat_map(|r| &r.lanes).filter(|l| l.kind == osm_import::LaneKind::Other).count();
    let curbs = net.roads.iter().flat_map(|r| &r.lanes).filter(|l| l.kind == osm_import::LaneKind::Buffer).count();
    assert_eq!(medians, others, "a median is a lane of the road that no lane uses, and nothing else");
    assert!(buffers > 0 && buffers <= curbs, "{buffers} buffers of {curbs} curbs: those beside a bike lane or a sidewalk are its own");
}

#[test]
fn no_street_of_the_montreal_extract_has_planting_the_data_does_not_show() {
    // Every buffer of the extract is a curb that osm2streets inferred; none is a planter or a verge.
    let net = import(PLATEAU).expect("the extract reads");
    let buffers = net.roads.iter().flat_map(|r| &r.lanes).filter(|l| l.kind == osm_import::LaneKind::Buffer).count();
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

mod kept {
    use std::collections::{BTreeMap, BTreeSet};
    use std::rc::Rc;

    use cityloom_editor::city::edits::{self, Document, EDITS_KEY, Edits};
    use cityloom_editor::city::model::CityView;
    use cityloom_editor::city::projection::Projection;
    use cityloom_editor::city::store::CityStore;
    use cityloom_editor::junction::model::RING_STEP_MM;
    use cityloom_editor::place::area::{Area, default_area};
    use cityloom_editor::shared::ports::{MemoryStorage, Storage};

    const TILE: &[u8] = include_bytes!("../../e2e/fixtures/plateau.osm.pbf");

    fn store(storage: &Rc<MemoryStorage>, area: Area) -> CityStore {
        let b = area.bounds();
        let network = osm_import::import_in(TILE, Some([b.south, b.west, b.north, b.east])).expect("the tile reads");
        let store = CityStore::for_area(storage.clone() as Rc<dyn Storage>, area);
        assert!(store.keep_network(&network));
        store
    }

    /// The default area moved `dx_m` metres east and `dy_m` north.
    fn shifted(dx_m: f64, dy_m: f64) -> Area {
        let a = default_area();
        Area::new("shifted", a.lat + dy_m / 111_320.0, a.lon + dx_m / (111_320.0 * a.lat.to_radians().cos()))
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

    /// Where the places edited in an area lie on the earth, by the key of their record: a street's centreline and a
    /// junction's point, as [lon, lat].
    #[derive(Default)]
    struct Placed {
        streets: BTreeMap<String, Vec<[f64; 2]>>,
        junctions: BTreeMap<String, [f64; 2]>,
    }

    fn projection(store: &CityStore, view: &CityView) -> Projection {
        let o = view.origin_m.expect("a city made from a network");
        Projection::new(store.area().bounds(), o)
    }

    /// Edits every street and junction of the area, each with a mark of its own (a street's `next_uid`, a
    /// junction's ring, both raised by its place in the city), and keeps each as the store's writes do (the
    /// place's record under its key, every other record left as it is), then opens the city again. One open for
    /// all the writes: a write opens the whole city, which takes about a second in a debug build.
    fn edit_everything(store: &CityStore, storage: &MemoryStorage) -> (Tally, Tally, Placed) {
        let mut city = store.open();
        let view = city.view(0);
        let at = projection(store, &view);
        let mut edits = document(storage);
        let (mut streets, mut junctions, mut placed) = (Tally::default(), Tally::default(), Placed::default());
        let mut written = (Vec::new(), Vec::new());
        for (i, e) in view.edges.iter().enumerate() {
            let mut s = city.street_editor(e.uid, 0).expect("every street opens").snapshot();
            s.next_uid += 1 + i as u32;
            assert!(city.keep_street(e.uid, s));
            match city.street_record(e.uid) {
                Some((key, record)) => {
                    placed.streets.insert(key.clone(), e.shape_mm.iter().map(|p| at.lon_lat(p[0], p[1])).collect());
                    edits.streets.insert(key, record);
                    written.0.push(e.uid);
                }
                None => streets.refused += 1,
            }
        }
        for (i, n) in view.nodes.iter().filter(|n| n.junction).enumerate() {
            let mut state = city.junction_editor(n.uid, 0).expect("every junction opens").snapshot();
            state.ring_extra_mm = (1 + i as i32) * RING_STEP_MM;
            assert!(city.keep_junction(n.uid, state), "junction number {i} takes a ring");
            match city.junction_record(n.uid) {
                Some((key, record)) => {
                    placed.junctions.insert(key.clone(), at.lon_lat(n.x_mm, n.y_mm));
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
        (streets, junctions, placed)
    }

    /// Positions as metres east and north of `origin`.
    fn metres(p: [f64; 2], origin: [f64; 2]) -> (f64, f64) {
        ((p[0] - origin[0]) * 111_320.0 * origin[1].to_radians().cos(), (p[1] - origin[1]) * 111_320.0)
    }

    fn distance_to_line(p: (f64, f64), line: &[(f64, f64)]) -> f64 {
        line.windows(2)
            .map(|w| {
                let (a, b) = (w[0], w[1]);
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let t = if dx == 0.0 && dy == 0.0 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0) };
                (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
            })
            .fold(f64::MAX, f64::min)
    }

    /// The points a quarter, half and three quarters along a line.
    fn inner_points(line: &[(f64, f64)]) -> Vec<(f64, f64)> {
        let length: f64 = line.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum();
        [0.25, 0.5, 0.75]
            .into_iter()
            .filter_map(|f| {
                let mut left = f * length;
                for w in line.windows(2) {
                    let d = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
                    if left <= d && d > 0.0 {
                        return Some((w[0].0 + (w[1].0 - w[0].0) * left / d, w[0].1 + (w[1].1 - w[0].1) * left / d));
                    }
                    left -= d;
                }
                None
            })
            .collect()
    }

    /// How near two centrelines are, in metres: the nearest that a point well inside one (a quarter, half or
    /// three quarters along) comes to the other. Two pieces of a way that only meet at a junction are far apart.
    fn apart_m(a: &[[f64; 2]], b: &[[f64; 2]]) -> f64 {
        let origin = a[0];
        let (a, b): (Vec<_>, Vec<_>) = (a.iter().map(|&p| metres(p, origin)).collect(), b.iter().map(|&p| metres(p, origin)).collect());
        let near = |from: &[(f64, f64)], to: &[(f64, f64)]| inner_points(from).into_iter().map(|p| distance_to_line(p, to)).fold(f64::MAX, f64::min);
        near(&a, &b).min(near(&b, &a))
    }

    /// Whether `line` is of the same piece of road as the edited street `took`: of the edited streets made from one
    /// of the same ways, it is the nearest (osm2streets may draw the same piece tens of metres apart in two areas,
    /// where it moves a junction's point, so nearness is measured against the other pieces, not in metres).
    fn same_piece(line: &[[f64; 2]], ways: &BTreeSet<i64>, took: &str, edits: &Edits, placed: &Placed) -> bool {
        let d = |key: &str| apart_m(line, &placed.streets[key]);
        let best = edits.streets.iter().filter(|(_, s)| !s.ways.is_disjoint(ways)).map(|(k, _)| d(k)).fold(f64::MAX, f64::min);
        d(took) <= best + 1.0
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

    /// How many places of an area show an edit made in the default area: of those made from the same
    /// OpenStreetMap ids as an edited place there, and of those the importer made the same in both areas; and how
    /// many took the edit of another place (not the same piece of road, or not the same junction).
    #[derive(Debug, Default, Clone, Copy)]
    struct Follow {
        /// Of the places that overlap an edited place in its ids, how many show an edit.
        overlapping: (usize, usize),
        /// Of the places that are the same in both areas (a street: the same ways, width and class; a junction:
        /// the same nodes and as many arms), how many show the edit.
        same: (usize, usize),
        /// Places that took an edit, and of those, how many took one made on another place.
        took: usize,
        wrong: usize,
        /// The check itself: other pieces of a street's way that meet it at an end, and of those, how many the
        /// check tells from the piece the street took.
        rivals: (usize, usize),
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
    fn followed(storage: &Rc<MemoryStorage>, area: Area, edits: &Edits, placed: &Placed) -> [Follow; 3] {
        let store = store(storage, area);
        let city = store.open();
        let view = city.view(0);
        let at = projection(&store, &view);
        let mut out = [Follow::default(); 3];
        for n in view.nodes.iter().filter(|n| n.junction) {
            let Some((_, r)) = city.junction_record(n.uid) else { continue };
            let from: Vec<_> = edits.junctions.values().filter(|j| !j.nodes.is_disjoint(&r.nodes)).collect();
            if !from.is_empty() {
                let same = from.iter().any(|j| j.nodes == r.nodes && j.arm_streets.len() == r.arm_streets.len());
                out[0].count(n.edited, same);
            }
            if let Some(took) = city.took_junction(n.uid) {
                // the edit applied is the record's (its mark), and the record was made on this junction
                let record = &edits.junctions[took];
                let here = metres(at.lon_lat(n.x_mm, n.y_mm), placed.junctions[took]);
                let right = r.state.ring_extra_mm == record.state.ring_extra_mm && !record.nodes.is_disjoint(&r.nodes) && here.0.hypot(here.1) < 3.0;
                out[0].took += 1;
                out[0].wrong += usize::from(!right);
            }
        }
        for e in &view.edges {
            let Some((key, r)) = city.street_record(e.uid) else { continue };
            let from: Vec<(&String, _)> = edits.streets.iter().filter(|(_, s)| !s.ways.is_disjoint(&r.ways) && !s.nodes.is_disjoint(&r.nodes)).collect();
            let cut = key.contains("~x") || from.iter().any(|(k, _)| k.contains("~x"));
            let kind = &mut out[if cut { 1 } else { 2 }];
            if !from.is_empty() {
                let same = from.iter().any(|(_, s)| s.ways == r.ways && s.street.row_mm == r.street.row_mm && s.street.class == r.street.class);
                kind.count(e.edited, same);
            }
            if let Some(took) = city.took_street(e.uid) {
                let record = &edits.streets[took];
                let line: Vec<[f64; 2]> = e.shape_mm.iter().map(|p| at.lon_lat(p[0], p[1])).collect();
                // the edit applied is the record's (its mark), made from a way of this street, on the same piece: by
                // its OpenStreetMap ids where both ends are nodes of the same key, else by where it lies
                let same_ids = took == key && !key.contains("~x");
                let right = r.street.next_uid == record.street.next_uid
                    && !record.ways.is_disjoint(&r.ways)
                    && (record.nodes.is_empty() || !record.nodes.is_disjoint(&r.nodes))
                    && (same_ids || same_piece(&line, &r.ways, took, edits, placed));
                kind.took += 1;
                kind.wrong += usize::from(!right);
                for (rival, _) in edits.streets.iter().filter(|(k, s)| *k != took && !s.ways.is_disjoint(&r.ways) && !s.nodes.is_disjoint(&record.nodes)) {
                    kind.rivals.1 += 1;
                    kind.rivals.0 += usize::from(!same_piece(&line, &r.ways, rival, edits, placed));
                }
            }
        }
        out
    }

    #[test]
    fn edits_follow_their_places_into_shifted_areas_and_never_onto_another_place() {
        let storage = Rc::new(MemoryStorage::default());
        let (.., placed) = edit_everything(&store(&storage, default_area()), &storage);
        let edits = document(&storage);
        let shifts: Vec<((f64, f64), [Follow; 3])> = [(150.0, 0.0), (-150.0, 0.0), (0.0, 150.0), (0.0, -150.0)]
            .into_iter()
            .map(|(dx, dy)| ((dx, dy), followed(&storage, shifted(dx, dy), &edits, &placed)))
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
