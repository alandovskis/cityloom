mod support;

use std::fs;
use std::path::{Path, PathBuf};

use cityloom_model::NetworkTile;
use citymodel_builder::metro_index::{index_json, short_checksum};
use citymodel_builder::tiles::{self, TilesJob};
use sha2::{Digest, Sha256};
use support::{calls, fake_tool, quiet_progress};

const HIGHWAYS: &str = "motorway,motorway_link,trunk,trunk_link,primary,primary_link,secondary,secondary_link,tertiary,tertiary_link,unclassified,residential,living_street,service,pedestrian";

/// A small real OSM extract (part of the Plateau, in Montréal), which the fake osmium's `merge` writes as the roads:
/// the import step needs OSM data it can read, and the osmium steps before it only filter.
fn plateau() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../e2e/fixtures/plateau.osm.pbf")
}

/// A fake osmium body: `merge` copies `roads` to its output (`-o`, the fifth argument); every other call does nothing.
fn merge_writes(roads: &Path) -> String {
    format!(r#"if [ "$1" = merge ]; then cp '{}' "$5"; fi"#, roads.display())
}

#[test]
fn the_metro_bounds_are_the_doubles_python_computed() {
    // The values came from the former Python metro_tiles' arithmetic (python3, 2026-10-17).
    assert_eq!(tiles::metro_bounds(), "-74.4064,45.1955,-73.08290000000001,45.816500000000005");
}

#[test]
fn the_steps_are_the_scripts_then_the_import() {
    let steps: Vec<_> = tiles::steps().into_iter().map(|s| (s.key, s.group, s.subject)).collect();
    let expected = [
        ("area", "roads", "metro area"),
        ("highways", "roads", "highways"),
        ("tracks", "roads", "designated tracks"),
        ("merge", "roads", "merge"),
        ("import", "network", "whole area"),
        ("tiles", "tiles", "cutting 1,734 tiles"),
        ("index", "tiles", "index"),
    ];
    assert_eq!(steps, expected.map(|(k, g, s)| (k, g, s.to_string())));
    let grid = tiles::grid();
    assert_eq!((grid.nx, grid.ny), (51, 34));
}

struct Setup {
    dir: tempfile::TempDir,
}

impl Setup {
    fn new() -> Setup {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("quebec.osm.pbf"), b"x").unwrap();
        Setup { dir }
    }
    fn p(&self, name: &str) -> String {
        self.dir.path().join(name).display().to_string()
    }
    fn run(&self, osmium_body: &str) -> (Result<(), citymodel_builder::progress::StepError>, String) {
        let osmium = fake_tool(self.dir.path(), "osmium", osmium_body);
        let (progress, sink) = quiet_progress(tiles::steps(), self.dir.path());
        let source = self.dir.path().join("quebec.osm.pbf");
        let work = self.dir.path().join("work");
        let out = self.dir.path().join("out");
        let job = TilesJob { source: &source, work: &work, out: &out, osmium: osmium.to_str().unwrap() };
        let result = tiles::run(&progress, &job);
        progress.close();
        (result, sink.text())
    }
}

#[test]
fn osmium_is_called_with_the_scripts_arguments_and_cuts_no_tiles() {
    let s = Setup::new();
    let (result, _) = s.run(&merge_writes(&plateau()));
    result.unwrap();
    let (source, work) = (s.p("quebec.osm.pbf"), s.p("work"));
    let expected = vec![
        format!("extract -b -74.4064,45.1955,-73.08290000000001,45.816500000000005 {source} -o {work}/metro.osm.pbf --overwrite"),
        format!("tags-filter {work}/metro.osm.pbf w/highway={HIGHWAYS},cycleway -o {work}/streets.osm.pbf --overwrite"),
        format!("tags-filter {work}/metro.osm.pbf w/bicycle=designated -o {work}/designated.osm.pbf --overwrite"),
        format!("tags-filter {work}/designated.osm.pbf w/highway=path,footway -o {work}/designated-tracks.osm.pbf --overwrite"),
        format!("merge {work}/streets.osm.pbf {work}/designated-tracks.osm.pbf -o {work}/roads.osm.pbf --overwrite"),
    ];
    assert_eq!(calls(s.dir.path(), "osmium"), expected);
}

/// The tile names of the `.json` files in `dir` (not `index.json`), sorted as text.
fn json_tiles(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter_map(|n| n.strip_suffix(".json").map(str::to_string))
        .filter(|n| n != "index")
        .collect();
    names.sort();
    names
}

#[test]
fn the_roads_are_imported_and_cut_into_network_tiles_with_their_index() {
    let s = Setup::new();
    let out = s.dir.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("old.osm.pbf"), vec![0u8; 900]).unwrap();
    fs::write(out.join("40_30.json"), "{}").unwrap();
    fs::write(out.join("keep.txt"), "x").unwrap();
    let (result, _) = s.run(&merge_writes(&plateau()));
    result.unwrap();
    assert!(!out.join("old.osm.pbf").exists());
    assert!(!out.join("40_30.json").exists());
    assert!(out.join("keep.txt").exists());

    let names = json_tiles(&out);
    assert!(names.len() >= 2, "{names:?}");
    let mut named = Vec::new();
    let mut frames = Vec::new();
    for name in &names {
        let bytes = fs::read(out.join(format!("{name}.json"))).unwrap();
        let tile: NetworkTile = serde_json::from_slice(&bytes).unwrap();
        assert!(!tile.network.roads.is_empty(), "{name} holds no road");
        assert!(tile.network.corridors.is_empty(), "{name} has corridors");
        frames.push(tile.frame);
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        named.push((name.clone(), short_checksum(&digest)));
    }
    // every tile is in the whole area's frame
    assert!(frames.windows(2).all(|w| w[0] == w[1]));
    // the index lists the tiles x outer, y inner, as the cut makes them
    named.sort_by_key(|(name, _)| {
        let (x, y) = name.split_once('_').unwrap();
        (x.parse::<u16>().unwrap(), y.parse::<u16>().unwrap())
    });
    assert_eq!(fs::read_to_string(out.join("index.json")).unwrap(), index_json(&tiles::grid(), &named));
}

#[test]
fn the_rows_say_what_was_imported_and_how_many_tiles_how_big() {
    let s = Setup::new();
    let (result, text) = s.run(&merge_writes(&plateau()));
    result.unwrap();
    let tiles = json_tiles(&s.dir.path().join("out")).len();
    assert!(text.lines().any(|l| l.starts_with("PASS network") && l.contains(" roads")), "{text}");
    assert!(text.lines().any(|l| l.starts_with("PASS tiles") && l.contains(&format!("{tiles} tiles")) && l.contains(" MB")), "{text}");
}

#[test]
fn roads_the_importer_cannot_read_fail_the_import_step() {
    let s = Setup::new();
    let junk = s.dir.path().join("junk.osm.pbf");
    fs::write(&junk, b"not OSM at all").unwrap();
    let (result, text) = s.run(&merge_writes(&junk));
    let error = result.unwrap_err();
    assert!(error.lines[0].starts_with(&format!("cannot import {}", s.p("work/roads.osm.pbf"))), "{:?}", error.lines);
    assert!(text.contains("FAIL network whole area"), "{text}");
    assert!(!s.dir.path().join("out/index.json").exists());
}

#[test]
fn a_missing_source_is_a_clear_error() {
    let s = Setup::new();
    fs::remove_file(s.dir.path().join("quebec.osm.pbf")).unwrap();
    let (result, _) = s.run("true");
    let error = result.unwrap_err();
    assert_eq!(error.lines[0], format!("{} does not exist. Download the Quebec extract first (about 1.2 GB):", s.p("quebec.osm.pbf")));
    assert_eq!(error.lines[1], "curl -L -o quebec-latest.osm.pbf https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf");
    assert!(calls(s.dir.path(), "osmium").is_empty());
}

#[test]
fn a_failing_osmium_fails_the_step_with_its_log() {
    let s = Setup::new();
    let (result, text) = s.run("echo boom; exit 2");
    let error = result.unwrap_err();
    assert!(error.lines.iter().any(|l| l == "osmium exited with status 2"));
    assert!(error.lines.iter().any(|l| l == "boom"));
    assert!(error.lines.iter().any(|l| l.starts_with("full log:")));
    assert!(text.contains("FAIL roads metro area"), "{text}");
    assert_eq!(calls(s.dir.path(), "osmium").len(), 1);
}
