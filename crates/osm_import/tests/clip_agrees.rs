//! A place cut from the whole area's network is the place the importer makes when it reads only the place's box,
//! as far as edits are concerned: an edit made on the clipped import follows its place into the cut, and never onto
//! another place. As in `real_city.rs`, the measure is the places the importer made the same in both (a street of the
//! same ways, width and class; a junction of the same nodes and as many arms): near a box's edge osm2streets may
//! build a place otherwise when it reads the box alone, which no cut of the whole can follow.
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

/// The keys of a network's streets, as the editor gives them (a street whose key another shares has none).
fn street_keys(net: &Network, name: &str) -> BTreeSet<String> {
    let city = City::from_network(net, name);
    city.view(0).edges.iter().filter_map(|e| city.street_record(e.uid)).map(|(key, _)| key).collect()
}

fn centre(f: &Frame) -> (f64, f64) {
    ((f.south + f.north) / 2.0, (f.west + f.east) / 2.0)
}

#[test]
fn clipping_the_whole_plateau_gives_the_places_the_clipped_import_gives() {
    let (whole, frame) = import_whole(PLATEAU).expect("the extract reads");
    let (lat, lon) = centre(&frame);
    let middle = Area::new("Plateau", lat, lon);
    // over the five boxes: (records found, made), (same-made places that follow, of), street keys (the same, of)
    let (mut records, mut same, mut keys, mut wrong) = ((0, 0), (0, 0), (0, 0), 0);
    for (dx, dy) in [(0.0, 0.0), (150.0, 0.0), (-150.0, 0.0), (0.0, 150.0), (0.0, -150.0)] {
        let area = shifted_from(&middle, "Plateau", dx, dy);
        let b = Bounds::around(area.lat, area.lon, 250.0);
        let clipped = import_in(PLATEAU, Some([b.south, b.west, b.north, b.east])).expect("the box reads");
        let cut = whole.clip_to(&frame, &b.frame());

        // every place of the clipped import edited and kept, then the city made from the cut opened over the records
        let storage = Rc::new(MemoryStorage::default());
        let (streets, junctions, placed) = edit_everything(&store_with(&storage, area.clone(), &clipped), &storage);
        assert_eq!((streets.lost, junctions.lost), (0, 0), "({dx}, {dy}): {streets:?} {junctions:?}");
        let edits = document(&storage);
        let made = edits.streets.len() + edits.junctions.len();
        let (follows, took) = followed(&store_with(&storage, area, &cut), &edits, &placed);
        let box_wrong: usize = follows.iter().map(|f| f.wrong).sum();
        let box_same = follows.iter().fold((0, 0), |s, f| (s.0 + f.same.0, s.1 + f.same.1));

        // for information: every record, the places osm2streets builds otherwise at the box's edge too, and the keys
        let (from_cut, from_clip) = (street_keys(&cut, "Plateau"), street_keys(&clipped, "Plateau"));
        let box_keys = (from_clip.intersection(&from_cut).count(), from_clip.len());
        println!(
            "({dx}, {dy}) m: same-made places follow {} of {} ({:.1}%); records found {} of {made} ({:.1}%); {box_wrong} on another place; \
             street keys the same {} of {}; roads {} clipped, {} cut; junctions {:?}, cut streets {:?}, other streets {:?}",
            box_same.0,
            box_same.1,
            percent(box_same),
            took.len(),
            percent((took.len(), made)),
            box_keys.0,
            box_keys.1,
            clipped.roads.len(),
            cut.roads.len(),
            follows[0],
            follows[1],
            follows[2]
        );
        let lost: Vec<_> = edits.streets.keys().chain(edits.junctions.keys()).filter(|k| !took.contains(*k)).collect();
        println!("  records not found: {lost:?}");
        records = (records.0 + took.len(), records.1 + made);
        same = (same.0 + box_same.0, same.1 + box_same.1);
        keys = (keys.0 + box_keys.0, keys.1 + box_keys.1);
        wrong += box_wrong;
    }
    println!(
        "all five boxes: same-made places follow {} of {} ({:.1}%); records found {} of {} ({:.1}%); street keys the same {} of {} ({:.1}%); {wrong} on another place",
        same.0,
        same.1,
        percent(same),
        records.0,
        records.1,
        percent(records),
        keys.0,
        keys.1,
        percent(keys)
    );
    assert_eq!(wrong, 0, "places that took the edit of another place");
    assert!(same.0 * 100 >= same.1 * 95, "{} of {} places the importer made the same in both follow", same.0, same.1);
}

fn percent((part, of): (usize, usize)) -> f64 {
    100.0 * part as f64 / of as f64
}

#[test]
fn the_whole_kreuzbergs_frame_holds_the_areas_centre() {
    let (_, frame) = import_whole(KREUZBERG).expect("the extract reads");
    let (lat, lon) = (52.5010, 13.4235);
    assert!(frame.south < lat && lat < frame.north && frame.west < lon && lon < frame.east, "{frame:?}");
}
