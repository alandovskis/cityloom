mod support;

use std::fs;
use std::path::{Path, PathBuf};

use cityloom_model::NetworkTile;
use citymodel_builder::archive::{self, read_index};
use citymodel_builder::commands::{self, Cli, Command, PackJob};
use citymodel_builder::metro_index::Grid;
use clap::Parser;
use support::{calls, fake_tool, quiet_progress};

#[test]
fn parse_timestamp_reads_osmiums_form() {
    assert_eq!(commands::parse_timestamp("2026-10-16T20:21:15Z"), Some(1792182075));
    assert_eq!(commands::parse_timestamp("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(commands::parse_timestamp("2000-02-29T23:59:59Z"), Some(951868799));
    for bad in ["2026-13-01T00:00:00Z", "2026-10-16 20:21:15", "", "x"] {
        assert_eq!(commands::parse_timestamp(bad), None, "{bad:?}");
    }
}

#[test]
fn source_time_asks_osmium_for_the_replication_timestamp() {
    let dir = tempfile::tempdir().unwrap();
    let osmium = fake_tool(dir.path(), "osmium", "echo 2026-10-16T20:21:15Z");
    let extract = Path::new("/data/quebec.osm.pbf");
    assert_eq!(commands::source_time(extract, osmium.to_str().unwrap()), 1792182075);
    assert_eq!(calls(dir.path(), "osmium"), ["fileinfo -g header.option.osmosis_replication_timestamp /data/quebec.osm.pbf"]);

    let silent = tempfile::tempdir().unwrap();
    let none = fake_tool(silent.path(), "osmium", "true");
    assert_eq!(commands::source_time(extract, none.to_str().unwrap()), 0);
    let failing = tempfile::tempdir().unwrap();
    let bad = fake_tool(failing.path(), "osmium", "echo 2026-10-16T20:21:15Z; exit 1");
    assert_eq!(commands::source_time(extract, bad.to_str().unwrap()), 0);
    assert_eq!(commands::source_time(extract, "/no/such/program"), 0);
}

const MONTREAL: (f64, f64, f64, f64) = (-74.4, 45.2, 0.0257, 0.018);
const FIXTURE: (f64, f64, f64, f64) = (-73.6078, 45.5161, 0.0257, 0.018);

/// A metro folder with the tiles `names` on disk and `listed` in index.json.
fn folders(root: &Path, grid: (f64, f64, f64, f64), on_disk: &[&str], listed: &[&str]) -> PathBuf {
    let metro = root.join("metro");
    fs::create_dir_all(&metro).unwrap();
    for name in on_disk {
        fs::write(metro.join(format!("{name}.json")), format!("tile {name}").repeat(30)).unwrap();
    }
    let tiles: Vec<String> = listed.iter().map(|n| format!("\"{n}\"")).collect();
    let (lon0, lat0, dlon, dlat) = grid;
    fs::write(
        metro.join("index.json"),
        format!("{{\"lon0\":{lon0},\"lat0\":{lat0},\"dlon\":{dlon},\"dlat\":{dlat},\"tiles\":[{}],\"checksums\":{{}}}}", tiles.join(",")),
    )
    .unwrap();
    metro
}

fn job(root: &Path, metro: PathBuf, grid: Option<Grid>) -> PackJob {
    PackJob { out: root.join("out.citymodel"), area: "montreal".into(), metro, source: "quebec-latest.osm.pbf".into(), source_time: 5, created: 7, grid }
}

fn header_grid(archive_path: &Path) -> (u16, u16) {
    let mut tar = tar::Archive::new(fs::File::open(archive_path).unwrap());
    let mut first = tar.entries().unwrap().next().unwrap().unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut first, &mut bytes).unwrap();
    let grid = read_index(&bytes).unwrap().header.grid;
    (grid.nx, grid.ny)
}

#[test]
fn pack_packs_the_folders_into_an_archive_the_server_reads() {
    let dir = tempfile::tempdir().unwrap();
    let metro = folders(dir.path(), MONTREAL, &["0_0", "12_9"], &["0_0", "12_9"]);
    let job = job(dir.path(), metro.clone(), None);
    let (progress, out) = quiet_progress(commands::pack_steps(&job.out), dir.path());
    commands::run_pack(&progress, &job).unwrap();
    progress.close();
    assert!(out.text().contains("PASS archive"), "{}", out.text());

    let data = dir.path().join("data");
    let unpacked = archive::unpack(&job.out, &data).unwrap();
    assert_eq!(unpacked.tiles, 2);
    for name in ["0_0", "12_9"] {
        let file = format!("{name}.json");
        assert_eq!(fs::read(data.join("montreal/metro").join(&file)).unwrap(), fs::read(metro.join(&file)).unwrap());
    }
    let index: serde_json::Value = serde_json::from_slice(&fs::read(data.join("montreal/metro/index.json")).unwrap()).unwrap();
    assert_eq!(index["tiles"], serde_json::json!(["0_0", "12_9"]));
    assert_eq!(header_grid(&job.out), (51, 34));
}

#[test]
fn a_grid_that_is_not_the_built_in_one_takes_its_size_from_the_tiles() {
    for (tiles, size) in [(&["0_0"][..], (1, 1)), (&["0_0", "10_3"][..], (11, 4))] {
        let dir = tempfile::tempdir().unwrap();
        let metro = folders(dir.path(), FIXTURE, tiles, tiles);
        let job = job(dir.path(), metro, None);
        let (progress, _) = quiet_progress(commands::pack_steps(&job.out), dir.path());
        commands::run_pack(&progress, &job).unwrap();
        assert_eq!(header_grid(&job.out), size);
    }
}

#[test]
fn a_tile_in_only_one_place_is_named() {
    for (on_disk, listed, named) in [(&["0_0", "3_4"][..], &["0_0"][..], "3_4"), (&["0_0"][..], &["0_0", "5_6"][..], "5_6")] {
        let dir = tempfile::tempdir().unwrap();
        let metro = folders(dir.path(), MONTREAL, on_disk, listed);
        let job = job(dir.path(), metro, None);
        let (progress, _) = quiet_progress(commands::pack_steps(&job.out), dir.path());
        let error = commands::run_pack(&progress, &job).unwrap_err();
        assert!(error.lines.iter().any(|l| l.contains(named)), "{:?}", error.lines);
        assert!(!job.out.exists());
        assert!(!dir.path().join("out.citymodel.part").exists());
    }
}

#[test]
fn old_pbf_tiles_are_not_network_tiles() {
    let dir = tempfile::tempdir().unwrap();
    let metro = folders(dir.path(), MONTREAL, &[], &["0_0"]);
    fs::write(metro.join("0_0.osm.pbf"), "an old tile").unwrap();
    let job = job(dir.path(), metro, None);
    let (progress, _) = quiet_progress(commands::pack_steps(&job.out), dir.path());
    let error = commands::run_pack(&progress, &job).unwrap_err();
    assert!(error.lines.iter().any(|l| l.contains("0_0.json")), "{:?}", error.lines);
    assert!(!job.out.exists());
}

#[test]
fn an_empty_folder_says_it_has_no_network_tiles() {
    let dir = tempfile::tempdir().unwrap();
    let metro = folders(dir.path(), MONTREAL, &[], &[]);
    let job = job(dir.path(), metro, None);
    let (progress, _) = quiet_progress(commands::pack_steps(&job.out), dir.path());
    let error = commands::run_pack(&progress, &job).unwrap_err();
    assert!(error.lines.iter().any(|l| l.starts_with("no network tiles in")), "{:?}", error.lines);
}

#[test]
fn fixture_writes_one_tile_of_the_whole_input_in_its_own_frame() {
    let dir = tempfile::tempdir().unwrap();
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../e2e/fixtures/plateau.osm.pbf");
    let out = dir.path().join("plateau.json");
    commands::fixture(&input, &out).unwrap();
    let bytes = fs::read(&out).unwrap();
    let tile: NetworkTile = serde_json::from_slice(&bytes).unwrap();
    assert!(tile.network.roads.len() > 150, "{} roads", tile.network.roads.len());
    let (network, frame) = osm_import::import_whole(&fs::read(&input).unwrap()).unwrap();
    assert_eq!(tile, NetworkTile { frame, network });
    assert!(!bytes.contains(&b'\n'), "compact JSON");
}

#[test]
fn fixture_says_what_it_cannot_read() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("junk.osm");
    fs::write(&input, "not OSM").unwrap();
    let out = dir.path().join("out.json");
    let error = commands::fixture(&input, &out).unwrap_err();
    assert!(error.starts_with(&format!("cannot import {}", input.display())), "{error}");
    assert!(!out.exists());
    let missing = commands::fixture(&dir.path().join("none.osm"), &out).unwrap_err();
    assert!(missing.starts_with(&format!("cannot read {}", dir.path().join("none.osm").display())), "{missing}");
}

#[test]
fn the_cli_parses_the_commands() {
    let parse = |args: &[&str]| Cli::try_parse_from(std::iter::once("citymodel_builder").chain(args.iter().copied()));
    assert!(parse(&["pack", "--area", "x", "--created", "0"]).is_ok());
    assert!(parse(&["pack"]).is_ok());
    assert!(parse(&["prepare"]).is_ok());
    assert!(parse(&["prepare", "q.osm.pbf", "--refresh"]).is_ok());
    assert!(parse(&["tiles", "q.osm.pbf", "work"]).is_ok());
    assert!(parse(&["tiles"]).is_err());
    // the tiles are no longer cut by osmium in batches
    assert!(parse(&["prepare", "--batch", "2"]).is_err());
    assert!(parse(&["tiles", "q.osm.pbf", "--batch", "2"]).is_err());
    assert!(parse(&["fixture", "in.osm.pbf", "out.json"]).is_ok());
    assert!(parse(&["fixture", "in.osm.pbf"]).is_err());
    assert!(parse(&["basemap", "q.osm.pbf"]).is_err());
}

#[test]
fn tiles_writes_where_out_says_and_else_where_it_did() {
    let parse = |args: &[&str]| Cli::try_parse_from(std::iter::once("citymodel_builder").chain(args.iter().copied())).unwrap();
    match parse(&["tiles", "q.osm.pbf", "work", "--out", "/tmp/metro"]).command {
        Command::Tiles { out, .. } => assert_eq!(out, Some(PathBuf::from("/tmp/metro"))),
        other => panic!("{other:?}"),
    }
    match parse(&["tiles", "q.osm.pbf"]).command {
        Command::Tiles { out, workdir, .. } => assert_eq!((out, workdir), (None, None)),
        other => panic!("{other:?}"),
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap().to_path_buf();
    assert_eq!(commands::tiles_out(None), root.join("web/data/metro"));
    assert_eq!(commands::tiles_out(Some(PathBuf::from("/x/metro"))), PathBuf::from("/x/metro"));
}
