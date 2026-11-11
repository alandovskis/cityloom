//! Edits kept on real data: every place of an area edited and kept as the store's writes do, then what a city made
//! from another network of the same place (another area's box, a cut of the whole area) does with those records.
//! Shared by `real_city.rs` and `clip_agrees.rs`; each uses part of it.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use cityloom_editor::city::edits::{self, Document, EDITS_KEY, Edits};
use cityloom_editor::city::model::CityView;
use cityloom_editor::city::projection::Projection;
use cityloom_editor::city::store::CityStore;
use cityloom_editor::junction::model::RING_STEP_MM;
use cityloom_editor::place::area::Area;
use cityloom_editor::shared::ports::{MemoryStorage, Storage};
use osm_import::Network;

/// A store of `area` in `storage`, holding `network` as the area's roads.
pub fn store_with(storage: &Rc<MemoryStorage>, area: Area, network: &Network) -> CityStore {
    let store = CityStore::for_area(storage.clone() as Rc<dyn Storage>, area);
    assert!(store.keep_network(network));
    store
}

/// `area` moved `dx_m` metres east and `dy_m` north.
pub fn shifted_from(area: &Area, name: &str, dx_m: f64, dy_m: f64) -> Area {
    Area::new(name, area.lat + dy_m / 111_320.0, area.lon + dx_m / (111_320.0 * area.lat.to_radians().cos()))
}

pub fn document(storage: &MemoryStorage) -> Edits {
    match edits::parse(storage.recall(EDITS_KEY).as_deref()) {
        Document::Current(e) => e,
        _ => Edits::new(),
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Tally {
    pub back: usize,
    /// Written (the store said it was kept) but as first laid out when the city is opened again.
    pub lost: usize,
    pub refused: usize,
}

/// Where the places edited in an area lie on the earth, by the key of their record: a street's centreline and a
/// junction's point, as [lon, lat].
#[derive(Default)]
pub struct Placed {
    pub streets: BTreeMap<String, Vec<[f64; 2]>>,
    pub junctions: BTreeMap<String, [f64; 2]>,
}

pub fn projection(store: &CityStore, view: &CityView) -> Projection {
    let o = view.origin_m.expect("a city made from a network");
    Projection::new(store.area().bounds(), o)
}

/// Edits every street and junction of the area, each with a mark of its own (a street's `next_uid`, a
/// junction's ring, both raised by its place in the city), and keeps each as the store's writes do (the
/// place's record under its key, every other record left as it is), then opens the city again. One open for
/// all the writes: a write opens the whole city, which takes about a second in a debug build.
pub fn edit_everything(store: &CityStore, storage: &MemoryStorage) -> (Tally, Tally, Placed) {
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
pub fn metres(p: [f64; 2], origin: [f64; 2]) -> (f64, f64) {
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

/// How many places of an area show an edit made in the default area: of those made from the same
/// OpenStreetMap ids as an edited place there, and of those the importer made the same in both areas; and how
/// many took the edit of another place (not the same piece of road, or not the same junction).
#[derive(Debug, Default, Clone, Copy)]
pub struct Follow {
    /// Of the places that overlap an edited place in its ids, how many show an edit.
    pub overlapping: (usize, usize),
    /// Of the places that are the same in both areas (a street: the same ways, width and class; a junction:
    /// the same nodes and as many arms), how many show the edit.
    pub same: (usize, usize),
    /// Places that took an edit, and of those, how many took one made on another place.
    pub took: usize,
    pub wrong: usize,
    /// The check itself: other pieces of a street's way that meet it at an end, and of those, how many the
    /// check tells from the piece the street took.
    pub rivals: (usize, usize),
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

/// What the city of `store` does with the records of `edits`: (junctions, cut streets, other streets), and the keys
/// of the records some place took rightly.
pub fn followed(store: &CityStore, edits: &Edits, placed: &Placed) -> ([Follow; 3], BTreeSet<String>) {
    let city = store.open();
    let view = city.view(0);
    let at = projection(store, &view);
    let mut out = [Follow::default(); 3];
    let mut found = BTreeSet::new();
    for n in view.nodes.iter().filter(|n| n.junction) {
        let Some((_, r)) = city.junction_record(n.uid) else { continue };
        let from: Vec<_> = edits.junctions.values().filter(|j| !j.nodes.is_disjoint(&r.nodes)).collect();
        if !from.is_empty() {
            let same = from.iter().any(|j| j.nodes == r.nodes && j.arm_streets.len() == r.arm_streets.len());
            out[0].count(n.edited, same);
        }
        if let Some(took) = city.took_junction(n.uid) {
            // the edit applied is the record's (its mark), and the record was made on this junction: of the same
            // OpenStreetMap nodes, or of some of them and where it lies (osm2streets may put the same junction's point
            // a few metres apart in two areas, where its arms are trimmed otherwise)
            let record = &edits.junctions[took];
            let here = metres(at.lon_lat(n.x_mm, n.y_mm), placed.junctions[took]);
            let right = r.state.ring_extra_mm == record.state.ring_extra_mm
                && (record.nodes == r.nodes || (!record.nodes.is_disjoint(&r.nodes) && here.0.hypot(here.1) < 3.0));
            out[0].took += 1;
            out[0].wrong += usize::from(!right);
            if right {
                found.insert(took.to_string());
            }
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
            if right {
                found.insert(took.to_string());
            }
            for (rival, _) in edits.streets.iter().filter(|(k, s)| *k != took && !s.ways.is_disjoint(&r.ways) && !s.nodes.is_disjoint(&record.nodes)) {
                kind.rivals.1 += 1;
                kind.rivals.0 += usize::from(!same_piece(&line, &r.ways, rival, edits, placed));
            }
        }
    }
    (out, found)
}
