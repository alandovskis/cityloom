mod support;

use std::fs;
use std::path::{Path, PathBuf};

use cityloom_prepare::archive::{self, read_index};
use cityloom_prepare::commands::{self, Cli, PackJob};
use cityloom_prepare::metro_index::Grid;
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

/// A metro folder with the tiles `names` on disk and `listed` in index.json, and a basemap folder with one file.
fn folders(root: &Path, grid: (f64, f64, f64, f64), on_disk: &[&str], listed: &[&str]) -> (PathBuf, PathBuf) {
    let metro = root.join("metro");
    let basemap = root.join("basemap");
    fs::create_dir_all(&metro).unwrap();
    fs::create_dir_all(&basemap).unwrap();
    for name in on_disk {
        fs::write(metro.join(format!("{name}.osm.pbf")), format!("tile {name}").repeat(30)).unwrap();
    }
    let tiles: Vec<String> = listed.iter().map(|n| format!("\"{n}\"")).collect();
    let (lon0, lat0, dlon, dlat) = grid;
    fs::write(
        metro.join("index.json"),
        format!("{{\"lon0\":{lon0},\"lat0\":{lat0},\"dlon\":{dlon},\"dlat\":{dlat},\"tiles\":[{}],\"checksums\":{{}}}}", tiles.join(",")),
    )
    .unwrap();
    fs::write(basemap.join("montreal.pmtiles"), b"PMTiles".repeat(500)).unwrap();
    (metro, basemap)
}

fn job(root: &Path, metro: PathBuf, basemap: PathBuf, grid: Option<Grid>) -> PackJob {
    PackJob {
        out: root.join("out.cityloom"),
        area: "montreal".into(),
        metro,
        basemap,
        source: "quebec-latest.osm.pbf".into(),
        source_time: 5,
        created: 7,
        grid,
    }
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
    let (metro, basemap) = folders(dir.path(), MONTREAL, &["0_0", "12_9"], &["0_0", "12_9"]);
    let job = job(dir.path(), metro.clone(), basemap.clone(), None);
    let (progress, out) = quiet_progress(commands::pack_steps(&job.out), dir.path());
    commands::run_pack(&progress, &job).unwrap();
    progress.close();
    assert!(out.text().contains("PASS archive"), "{}", out.text());

    let data = dir.path().join("data");
    let unpacked = archive::unpack(&job.out, &data).unwrap();
    assert_eq!(unpacked.tiles, 2);
    for name in ["0_0", "12_9"] {
        let file = format!("{name}.osm.pbf");
        assert_eq!(fs::read(data.join("montreal/metro").join(&file)).unwrap(), fs::read(metro.join(&file)).unwrap());
    }
    assert_eq!(fs::read(data.join("montreal/basemap/montreal.pmtiles")).unwrap(), fs::read(basemap.join("montreal.pmtiles")).unwrap());
    let index: serde_json::Value = serde_json::from_slice(&fs::read(data.join("montreal/metro/index.json")).unwrap()).unwrap();
    assert_eq!(index["tiles"], serde_json::json!(["0_0", "12_9"]));
    assert_eq!(header_grid(&job.out), (51, 34));
}

#[test]
fn a_grid_that_is_not_the_built_in_one_takes_its_size_from_the_tiles() {
    for (tiles, size) in [(&["0_0"][..], (1, 1)), (&["0_0", "10_3"][..], (11, 4))] {
        let dir = tempfile::tempdir().unwrap();
        let (metro, basemap) = folders(dir.path(), FIXTURE, tiles, tiles);
        let job = job(dir.path(), metro, basemap, None);
        let (progress, _) = quiet_progress(commands::pack_steps(&job.out), dir.path());
        commands::run_pack(&progress, &job).unwrap();
        assert_eq!(header_grid(&job.out), size);
    }
}

#[test]
fn a_tile_in_only_one_place_is_named() {
    for (on_disk, listed, named) in [(&["0_0", "3_4"][..], &["0_0"][..], "3_4"), (&["0_0"][..], &["0_0", "5_6"][..], "5_6")] {
        let dir = tempfile::tempdir().unwrap();
        let (metro, basemap) = folders(dir.path(), MONTREAL, on_disk, listed);
        let job = job(dir.path(), metro, basemap, None);
        let (progress, _) = quiet_progress(commands::pack_steps(&job.out), dir.path());
        let error = commands::run_pack(&progress, &job).unwrap_err();
        assert!(error.lines.iter().any(|l| l.contains(named)), "{:?}", error.lines);
        assert!(!job.out.exists());
        assert!(!dir.path().join("out.cityloom.part").exists());
    }
}

#[test]
fn a_missing_basemap_is_a_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    let (metro, basemap) = folders(dir.path(), MONTREAL, &["0_0"], &["0_0"]);
    fs::remove_file(basemap.join("montreal.pmtiles")).unwrap();
    let job = job(dir.path(), metro, basemap, None);
    let (progress, _) = quiet_progress(commands::pack_steps(&job.out), dir.path());
    let error = commands::run_pack(&progress, &job).unwrap_err();
    assert!(error.lines.iter().any(|l| l.contains("no basemap")), "{:?}", error.lines);
    assert_eq!(progress.steps()[0].state, cityloom_prepare::progress::State::Failed);
}

#[test]
fn the_cli_parses_the_four_commands() {
    let parse = |args: &[&str]| Cli::try_parse_from(std::iter::once("cityloom_prepare").chain(args.iter().copied()));
    assert!(parse(&["pack", "--area", "x", "--created", "0"]).is_ok());
    assert!(parse(&["pack"]).is_ok());
    assert!(parse(&["prepare"]).is_ok());
    assert!(parse(&["prepare", "q.osm.pbf", "--refresh", "--batch", "500"]).is_ok());
    assert!(parse(&["tiles", "q.osm.pbf", "work", "--batch", "1"]).is_ok());
    assert!(parse(&["tiles"]).is_err());
    for batch in ["0", "501"] {
        assert!(parse(&["prepare", "--batch", batch]).is_err(), "prepare {batch}");
        assert!(parse(&["tiles", "q.osm.pbf", "--batch", batch]).is_err(), "tiles {batch}");
    }
    assert!(parse(&["basemap", "q.osm.pbf", "out", "--bounds", "-74.4,45.2,-73.09,45.81", "--minzoom", "0", "--maxzoom", "14", "--memory", "4g"]).is_ok());
    assert!(parse(&["basemap", "q.osm.pbf", "--memory", "4G"]).is_ok());
    assert!(parse(&["basemap", "q.osm.pbf", "--memory", "4gb"]).is_err());
    assert!(parse(&["basemap", "q.osm.pbf", "--bounds", "1,2,3"]).is_err());
    assert!(parse(&["basemap", "q.osm.pbf", "--bounds", "1,2,3,x"]).is_err());
}
