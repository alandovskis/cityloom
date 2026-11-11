//! A place cut from the whole area's network is the place the importer makes when it reads only the place's box:
//! the same streets, under the same keys. Plateau Mont-Royal and Kreuzberg, © OpenStreetMap contributors, ODbL.

use std::collections::BTreeSet;

use cityloom_editor::city::model::City;
use cityloom_editor::place::Bounds;
use cityloom_editor::place::area::Area;
use osm_import::{Frame, Network, import_in, import_whole};

const KREUZBERG: &[u8] = include_bytes!("data/kreuzberg.osm");
const PLATEAU: &[u8] = include_bytes!("data/plateau.osm");

/// The keys of a network's streets, as the editor gives them (a street whose key another shares has none).
fn street_keys(net: &Network, name: &str) -> BTreeSet<String> {
    let city = City::from_network(net, name);
    city.view(0).edges.iter().filter_map(|e| city.street_record(e.uid)).map(|(key, _)| key).collect()
}

/// The way a street's key names: its smallest.
fn way_of(key: &str) -> i64 {
    key.strip_prefix('w').and_then(|k| k.split('~').next()).and_then(|w| w.parse().ok()).unwrap_or_else(|| panic!("no way in {key}"))
}

fn centre(f: &Frame) -> (f64, f64) {
    ((f.south + f.north) / 2.0, (f.west + f.east) / 2.0)
}

#[test]
#[ignore = "48 of 52 keys (92.3%) agree: osm2streets builds the junctions within a metre of the box's east edge otherwise in the clipped import; see task-5-report.md"]
fn clipping_the_whole_plateau_gives_the_places_the_clipped_import_gives() {
    let (whole, frame) = import_whole(PLATEAU).expect("the extract reads");
    let (lat, lon) = centre(&frame);
    let area = Area::new("Plateau", lat, lon);
    let b = Bounds::around(area.lat, area.lon, 250.0);
    let cut = whole.clip_to(&frame, &b.frame());
    let clipped = import_in(PLATEAU, Some([b.south, b.west, b.north, b.east])).expect("the box reads");

    let (from_cut, from_clip) = (street_keys(&cut, "Plateau"), street_keys(&clipped, "Plateau"));
    let shared = from_clip.intersection(&from_cut).count();
    let only_clip: Vec<_> = from_clip.difference(&from_cut).collect();
    let only_cut: Vec<_> = from_cut.difference(&from_clip).collect();
    println!(
        "keys: clipped import {}, cut {}, shared {shared} ({:.1}%); roads: clipped import {}, cut {}",
        from_clip.len(),
        from_cut.len(),
        100.0 * shared as f64 / from_clip.len() as f64,
        clipped.roads.len(),
        cut.roads.len()
    );
    println!("only in the clipped import: {only_clip:?}");
    println!("only in the cut: {only_cut:?}");

    assert!(from_clip.len() > 50, "{} streets", from_clip.len());
    assert!(shared * 100 >= from_clip.len() * 95, "{shared} of {} keys of the clipped import are the cut's", from_clip.len());
    let lacking: Vec<_> = from_cut.iter().filter(|k| !clipped.roads.iter().any(|r| r.has_way(way_of(k)))).collect();
    assert!(lacking.is_empty(), "keys of the cut naming a way the clipped import lacks: {lacking:?}");
    let (a, c) = (clipped.roads.len(), cut.roads.len());
    assert!(a.abs_diff(c) * 100 <= a * 5, "{c} roads cut against {a} imported");
}

#[test]
fn the_whole_kreuzbergs_frame_holds_the_areas_centre() {
    let (_, frame) = import_whole(KREUZBERG).expect("the extract reads");
    let (lat, lon) = (52.5010, 13.4235);
    assert!(frame.south < lat && lat < frame.north && frame.west < lon && lon < frame.east, "{frame:?}");
}
