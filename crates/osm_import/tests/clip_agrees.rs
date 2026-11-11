//! A place cut from the whole area's network is the place the importer makes when it reads only the place's box,
//! as far as edits are concerned: an edit made on the clipped import follows its place into the cut, and never onto
//! another place. As in `real_city.rs`, the measure is the places the importer made the same in both (a street of the
//! same ways, width and class; a junction of the same nodes and as many arms): near a box's edge osm2streets may
//! build a place otherwise when it reads the box alone, which no cut of the whole can follow.
//! A cut key may name a way the clipped import lacks: osm2streets' rule that a way with no node in the box is not
//! read is not reproduced (the network does not keep a way's nodes), so the cut keeps pieces across a box's corner.
//! Plateau Mont-Royal and Kreuzberg, © OpenStreetMap contributors, ODbL.

mod common;

use std::collections::BTreeSet;
use std::rc::Rc;

use cityloom_editor::city::model::City;
use cityloom_editor::place::Bounds;
use cityloom_editor::place::area::Area;
use cityloom_editor::shared::ports::MemoryStorage;
use common::{document, edit_everything, followed, shifted_from, store_with};
use osm_import::{Frame, Network, import_in, import_whole};

const KREUZBERG: &[u8] = include_bytes!("data/kreuzberg.osm");
const PLATEAU: &[u8] = include_bytes!("data/plateau.osm");

/// The least share of the clipped import's records found in the cut (measured: 292 of 312, 93.6%), and the least
/// share of them that are places the importer made the same in both (measured: 292 of 312, 93.6%). Both sit below
/// the measured values to leave room for importer noise; they keep the 95% from being met by a cut that drops places.
const RECORDS_FOUND_FLOOR: f64 = 0.90;
const SAME_MADE_FLOOR: f64 = 0.90;

/// The cut's roads as a share of the clipped import's, in every box: the cut also keeps the pieces of roads that
/// cross a corner of the box with no node of their way in it, which the clipped import does not read.
const ROADS: std::ops::RangeInclusive<f64> = 0.95..=1.10;

/// The keys of a network's streets, as the editor gives them (a street whose key another shares has none).
fn street_keys(net: &Network, name: &str) -> BTreeSet<String> {
    let city = City::from_network(net, name);
    city.view(0).edges.iter().filter_map(|e| city.street_record(e.uid)).map(|(key, _)| key).collect()
}

fn centre(f: &Frame) -> (f64, f64) {
    ((f.south + f.north) / 2.0, (f.west + f.east) / 2.0)
}

fn percent((part, of): (usize, usize)) -> f64 {
    100.0 * part as f64 / of as f64
}

/// What one box gives.
struct Measure {
    at: (f64, f64),
    /// Records of the clipped import found in the cut, of those made.
    records: (usize, usize),
    /// Places the importer made the same in both that follow, of those.
    same: (usize, usize),
    /// Street keys of the clipped import the cut has too, of those.
    keys: (usize, usize),
    /// Places that took the edit of another place.
    wrong: usize,
    /// Roads of the clipped import and of the cut.
    roads: (usize, usize),
    /// Streets of the cut made of no way the clipped import has.
    extra: usize,
    /// Of those, the ones that took a record; and any street that took one with no record of its own to tell its
    /// ways by.
    extra_taken: Vec<String>,
    lost: Vec<String>,
}

/// Every place of the clipped import edited and kept, then the city made from `cut` opened over the records.
fn measure(at: (f64, f64), area: Area, clipped: &Network, cut: &Network) -> Measure {
    let storage = Rc::new(MemoryStorage::default());
    let (streets, junctions, placed) = edit_everything(&store_with(&storage, area.clone(), clipped), &storage);
    assert_eq!((streets.lost, junctions.lost), (0, 0), "{at:?}: {streets:?} {junctions:?}");
    let edits = document(&storage);
    let store = store_with(&storage, area, cut);
    let (follows, took) = followed(&store, &edits, &placed);

    let read: BTreeSet<i64> = clipped.roads.iter().flat_map(|r| r.provenance.ways().map(|w| w.id)).collect();
    let city = store.open();
    let (mut extra, mut extra_taken) = (0, Vec::new());
    for e in &city.view(0).edges {
        let taken = city.took_street(e.uid);
        match city.street_record(e.uid) {
            Some((key, r)) if r.ways.is_disjoint(&read) => {
                extra += 1;
                extra_taken.extend(taken.map(|t| format!("{key} took {t}")));
            }
            Some(_) => {}
            None => extra_taken.extend(taken.map(|t| format!("street {} with no record of its own took {t}", e.uid))),
        }
    }
    let (from_cut, from_clip) = (street_keys(cut, "Plateau"), street_keys(clipped, "Plateau"));
    Measure {
        at,
        records: (took.len(), edits.streets.len() + edits.junctions.len()),
        same: follows.iter().fold((0, 0), |s, f| (s.0 + f.same.0, s.1 + f.same.1)),
        keys: (from_clip.intersection(&from_cut).count(), from_clip.len()),
        wrong: follows.iter().map(|f| f.wrong).sum(),
        roads: (clipped.roads.len(), cut.roads.len()),
        extra,
        extra_taken,
        lost: edits.streets.keys().chain(edits.junctions.keys()).filter(|k| !took.contains(*k)).cloned().collect(),
    }
}

fn sum(boxes: &[Measure], of: impl Fn(&Measure) -> (usize, usize)) -> (usize, usize) {
    boxes.iter().map(of).fold((0, 0), |s, x| (s.0 + x.0, s.1 + x.1))
}

/// What the boxes fail of the criterion: none takes another's edit and at least 95% of the places made the same
/// follow; and, so that it cannot be met by dropping places, the floors, the road counts and no extra street edited.
fn failures(boxes: &[Measure]) -> Vec<String> {
    let mut out = Vec::new();
    let (records, same) = (sum(boxes, |m| m.records), sum(boxes, |m| m.same));
    let wrong: usize = boxes.iter().map(|m| m.wrong).sum();
    if wrong > 0 {
        out.push(format!("{wrong} places took the edit of another place"));
    }
    if same.0 * 100 < same.1 * 95 {
        out.push(format!("{} of {} places the importer made the same in both follow", same.0, same.1));
    }
    if (records.0 as f64) < records.1 as f64 * RECORDS_FOUND_FLOOR {
        out.push(format!("{} of {} records found", records.0, records.1));
    }
    if (same.1 as f64) < records.1 as f64 * SAME_MADE_FLOOR {
        out.push(format!("{} places made the same, of {} records", same.1, records.1));
    }
    for m in boxes {
        if !ROADS.contains(&(m.roads.1 as f64 / m.roads.0 as f64)) {
            out.push(format!("{:?}: {} roads cut against {} clipped", m.at, m.roads.1, m.roads.0));
        }
        if !m.extra_taken.is_empty() {
            out.push(format!("{:?}: streets of no way the clipped import has took records: {:?}", m.at, m.extra_taken));
        }
    }
    out
}

/// The whole Plateau, its frame, and the boxes 250 m either way of its middle and of the points 150 m east, west,
/// north and south of it.
fn plateau_boxes() -> (Network, Frame, Vec<((f64, f64), Area, Bounds)>) {
    let (whole, frame) = import_whole(PLATEAU).expect("the extract reads");
    let (lat, lon) = centre(&frame);
    let middle = Area::new("Plateau", lat, lon);
    let boxes = [(0.0, 0.0), (150.0, 0.0), (-150.0, 0.0), (0.0, 150.0), (0.0, -150.0)]
        .into_iter()
        .map(|(dx, dy)| {
            let area = shifted_from(&middle, "Plateau", dx, dy);
            let b = Bounds::around(area.lat, area.lon, 250.0);
            ((dx, dy), area, b)
        })
        .collect();
    (whole, frame, boxes)
}

fn clipped(b: &Bounds) -> Network {
    import_in(PLATEAU, Some([b.south, b.west, b.north, b.east])).expect("the box reads")
}

#[test]
fn clipping_the_whole_plateau_gives_the_places_the_clipped_import_gives() {
    let (whole, frame, boxes) = plateau_boxes();
    let measured: Vec<Measure> = boxes.into_iter().map(|(at, area, b)| measure(at, area, &clipped(&b), &whole.clip_to(&frame, &b.frame()))).collect();
    for m in &measured {
        println!(
            "{:?} m: same-made places follow {} of {} ({:.1}%); records found {} of {} ({:.1}%); {} on another place; street keys the same {} of {}; \
             roads {} clipped, {} cut ({:.1}%); {} streets of no way the clipped import has, {} of them edited",
            m.at,
            m.same.0,
            m.same.1,
            percent(m.same),
            m.records.0,
            m.records.1,
            percent(m.records),
            m.wrong,
            m.keys.0,
            m.keys.1,
            m.roads.0,
            m.roads.1,
            percent((m.roads.1, m.roads.0)),
            m.extra,
            m.extra_taken.len()
        );
        println!("  records not found: {:?}", m.lost);
    }
    let (records, same, keys) = (sum(&measured, |m| m.records), sum(&measured, |m| m.same), sum(&measured, |m| m.keys));
    println!(
        "all five boxes: same-made places follow {} of {} ({:.1}%); records found {} of {} ({:.1}%); places made the same {} of {} records ({:.1}%); \
         street keys the same {} of {} ({:.1}%)",
        same.0,
        same.1,
        percent(same),
        records.0,
        records.1,
        percent(records),
        same.1,
        records.1,
        percent((same.1, records.1)),
        keys.0,
        keys.1,
        percent(keys)
    );
    assert_eq!(failures(&measured), Vec::<String>::new());
}

#[test]
fn a_cut_that_drops_places_does_not_meet_the_criterion() {
    let (whole, frame, boxes) = plateau_boxes();
    let (at, area, b) = boxes.into_iter().next().expect("the middle box");
    let clipped = clipped(&b);
    let cut = whole.clip_to(&frame, &b.frame());
    // (a city needs a road to be laid out at all: the emptiest cut is one road)
    for (what, step) in [("one road of the cut", cut.roads.len()), ("every other road of the cut", 2)] {
        let fails = failures(&[measure(at, area.clone(), &clipped, &keeping_every(&cut, step))]);
        println!("{what}: {fails:?}");
        assert!(fails.iter().any(|f| f.contains("records found")) && fails.iter().any(|f| f.contains("roads cut")), "{what}: {fails:?}");
    }
}

/// The network with every `step`th of its roads, from the first, and the junctions they use.
fn keeping_every(net: &Network, step: usize) -> Network {
    let mut kept = net.clone();
    kept.roads = net.roads.iter().step_by(step).cloned().collect();
    let used: BTreeSet<_> = kept.roads.iter().flat_map(|r| [r.edge.from, r.edge.to]).collect();
    kept.junctions.retain(|j| used.contains(&j.id));
    kept.corridors.clear();
    kept
}

#[test]
fn the_whole_kreuzbergs_frame_holds_the_areas_centre() {
    let (_, frame) = import_whole(KREUZBERG).expect("the extract reads");
    let (lat, lon) = (52.5010, 13.4235);
    assert!(frame.south < lat && lat < frame.north && frame.west < lon && lon < frame.east, "{frame:?}");
}
