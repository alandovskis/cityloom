mod support;

use std::fs;

use cityloom_prepare::metro_index::{index_json, short_checksum};
use cityloom_prepare::tiles::{self, TilesJob};
use sha2::{Digest, Sha256};
use support::{calls, fake_tool, quiet_progress};

const HIGHWAYS: &str = "motorway,motorway_link,trunk,trunk_link,primary,primary_link,secondary,secondary_link,tertiary,tertiary_link,unclassified,residential,living_street,service,pedestrian";

fn shown(bbox: [f64; 4]) -> String {
    format!("[{}, {}, {}, {}]", bbox[0], bbox[1], bbox[2], bbox[3])
}

#[test]
fn the_extract_bounds_are_the_doubles_python_computed() {
    // The values came from the former Python metro_tiles' arithmetic (python3, 2026-10-17).
    let all = tiles::extracts();
    assert_eq!(all.len(), 1734);
    assert_eq!(all[0].output, "0_0.osm.pbf");
    assert_eq!(all[1].output, "0_1.osm.pbf");
    let at = |x: usize, y: usize| &all[x * tiles::NY as usize + y];
    assert_eq!(shown(at(0, 0).bbox), "[-74.4064, 45.1955, -74.3679, 45.222500000000004]");
    assert_eq!(shown(at(31, 17).bbox), "[-73.6097, 45.5015, -73.5712, 45.5285]");
    assert_eq!(shown(at(50, 33).bbox), "[-73.12140000000001, 45.789500000000004, -73.08290000000001, 45.816500000000005]");
    assert_eq!(at(31, 17).output, "31_17.osm.pbf");
    assert_eq!(tiles::metro_bounds(), "-74.4064,45.1955,-73.08290000000001,45.816500000000005");
}

#[test]
fn the_steps_are_the_scripts() {
    let steps: Vec<_> = tiles::steps().into_iter().map(|s| (s.key, s.group, s.subject)).collect();
    let expected = [
        ("area", "roads", "metro area"),
        ("highways", "roads", "highways"),
        ("tracks", "roads", "designated tracks"),
        ("merge", "roads", "merge"),
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
    fn run(&self, osmium_body: &str, batch: usize) -> (Result<(), cityloom_prepare::progress::StepError>, String) {
        let osmium = fake_tool(self.dir.path(), "osmium", osmium_body);
        let (progress, sink) = quiet_progress(tiles::steps(), self.dir.path());
        let source = self.dir.path().join("quebec.osm.pbf");
        let work = self.dir.path().join("work");
        let out = self.dir.path().join("out");
        let job = TilesJob { source: &source, work: &work, out: &out, batch, osmium: osmium.to_str().unwrap() };
        let result = tiles::run(&progress, &job);
        progress.close();
        (result, sink.text())
    }
}

#[test]
fn osmium_is_called_with_the_scripts_arguments() {
    let s = Setup::new();
    let body = r#"if [ "$1" = extract ] && [ "$2" = --config ]; then n=$(ls "$(dirname "$LOG")" | grep -c '^config-'); cp "$3" "$(dirname "$LOG")/config-$n.json"; fi"#;
    let (result, _) = s.run(body, 500);
    result.unwrap();
    let (source, work) = (s.p("quebec.osm.pbf"), s.p("work"));
    let mut expected = vec![
        format!("extract -b -74.4064,45.1955,-73.08290000000001,45.816500000000005 {source} -o {work}/metro.osm.pbf --overwrite"),
        format!("tags-filter {work}/metro.osm.pbf w/highway={HIGHWAYS},cycleway -o {work}/streets.osm.pbf --overwrite"),
        format!("tags-filter {work}/metro.osm.pbf w/bicycle=designated -o {work}/designated.osm.pbf --overwrite"),
        format!("tags-filter {work}/designated.osm.pbf w/highway=path,footway -o {work}/designated-tracks.osm.pbf --overwrite"),
        format!("merge {work}/streets.osm.pbf {work}/designated-tracks.osm.pbf -o {work}/roads.osm.pbf --overwrite"),
    ];
    expected.extend(std::iter::repeat_n(format!("extract --config {work}/tiles.json {work}/roads.osm.pbf --overwrite"), 4));
    assert_eq!(calls(s.dir.path(), "osmium"), expected);

    let mut got = Vec::new();
    for n in 0..4 {
        let config: serde_json::Value = serde_json::from_str(&fs::read_to_string(s.dir.path().join(format!("config-{n}.json"))).unwrap()).unwrap();
        assert_eq!(config["directory"], s.p("out"));
        got.extend(config["extracts"].as_array().unwrap().clone());
    }
    let want = tiles::extracts();
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(&want) {
        assert_eq!(g["output"], w.output);
        assert_eq!(g["output_format"], "pbf");
        let bbox: Vec<f64> = g["bbox"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
        assert_eq!(bbox, w.bbox);
    }
}

#[test]
fn a_smaller_batch_runs_osmium_more_often() {
    let s = Setup::new();
    s.run("true", 2).0.unwrap();
    let extracts = calls(s.dir.path(), "osmium").iter().filter(|c| c.starts_with("extract --config")).count();
    assert_eq!(extracts, 867);
}

#[test]
fn a_missing_source_is_a_clear_error() {
    let s = Setup::new();
    fs::remove_file(s.dir.path().join("quebec.osm.pbf")).unwrap();
    let (result, _) = s.run("true", 500);
    let error = result.unwrap_err();
    assert_eq!(error.lines[0], format!("{} does not exist. Download the Quebec extract first (about 1.2 GB):", s.p("quebec.osm.pbf")));
    assert_eq!(error.lines[1], "curl -L -o quebec-latest.osm.pbf https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf");
    assert!(calls(s.dir.path(), "osmium").is_empty());
}

#[test]
fn the_old_tiles_go_first_and_the_empty_ones_are_dropped() {
    let s = Setup::new();
    let out = s.dir.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("old.osm.pbf"), vec![0u8; 900]).unwrap();
    fs::write(out.join("keep.txt"), "x").unwrap();
    let body = format!(
        r#"if [ "$1" = extract ] && [ "$2" = --config ]; then
  head -c 500 /dev/zero > '{o}/0_0.osm.pbf'; head -c 900 /dev/zero > '{o}/10_3.osm.pbf'
  head -c 700 /dev/zero > '{o}/2_0.osm.pbf'; head -c 400 /dev/zero > '{o}/5_5.osm.pbf'
fi"#,
        o = out.display()
    );
    let (result, _) = s.run(&body, 500);
    result.unwrap();
    assert!(!out.join("old.osm.pbf").exists());
    assert!(!out.join("5_5.osm.pbf").exists());
    assert!(out.join("keep.txt").exists());
    let sum = |name: &str| {
        let digest: [u8; 32] = Sha256::digest(fs::read(out.join(format!("{name}.osm.pbf"))).unwrap()).into();
        (name.to_string(), short_checksum(&digest))
    };
    let expected = index_json(&tiles::grid(), &[sum("0_0"), sum("10_3"), sum("2_0")]);
    assert_eq!(fs::read_to_string(out.join("index.json")).unwrap(), expected);
}

#[test]
fn the_tiles_row_says_how_many_and_how_big() {
    let s = Setup::new();
    let out = s.dir.path().join("out");
    let body = format!(r#"if [ "$2" = --config ]; then head -c 1500000 /dev/zero > '{}/0_0.osm.pbf'; fi"#, out.display());
    let (result, text) = s.run(&body, 500);
    result.unwrap();
    assert!(text.contains("1 tiles") && text.contains("1.5 MB"), "{text}");
}

#[test]
fn a_failing_osmium_fails_the_step_with_its_log() {
    let s = Setup::new();
    let (result, text) = s.run("echo boom; exit 2", 500);
    let error = result.unwrap_err();
    assert!(error.lines.iter().any(|l| l == "osmium exited with status 2"));
    assert!(error.lines.iter().any(|l| l == "boom"));
    assert!(error.lines.iter().any(|l| l.starts_with("full log:")));
    assert!(text.contains("FAIL roads metro area"), "{text}");
    assert_eq!(calls(s.dir.path(), "osmium").len(), 1);
}
