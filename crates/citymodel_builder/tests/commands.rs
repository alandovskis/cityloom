mod support;

use std::fs;
use std::path::{Path, PathBuf};

use cityloom_model::{Network, NetworkTile};
use citymodel_builder::archive::{self, Header, read_index};
use citymodel_builder::commands::{self, Cli, Command, PackArgs, PackJob};
use citymodel_builder::metro_index::Grid;
use clap::Parser;
use support::{MONTREAL_GRID, TESTVILLE, calls, city_named, fake_tool, quiet_progress, write_city};

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

fn header(archive_path: &Path) -> Header {
    let mut tar = tar::Archive::new(fs::File::open(archive_path).unwrap());
    let mut first = tar.entries().unwrap().next().unwrap().unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut first, &mut bytes).unwrap();
    read_index(&bytes).unwrap().header
}

fn header_grid(archive_path: &Path) -> (u16, u16) {
    let grid = header(archive_path).grid;
    (grid.nx, grid.ny)
}

#[test]
fn pack_packs_the_folders_into_an_archive_the_server_reads() {
    let dir = tempfile::tempdir().unwrap();
    let metro = folders(dir.path(), MONTREAL, &["0_0", "12_9"], &["0_0", "12_9"]);
    let job = job(dir.path(), metro.clone(), Some(MONTREAL_GRID));
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
fn a_grid_not_given_always_takes_its_size_from_the_tiles() {
    let dir = tempfile::tempdir().unwrap();
    let metro = folders(dir.path(), MONTREAL, &["0_0", "12_9"], &["0_0", "12_9"]);
    let job = job(dir.path(), metro, None);
    let (progress, _) = quiet_progress(commands::pack_steps(&job.out), dir.path());
    commands::run_pack(&progress, &job).unwrap();
    assert_eq!(header_grid(&job.out), (13, 10));
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
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/e2e/fixtures/plateau.osm.pbf");
    let out = dir.path().join("plateau.json");
    commands::fixture(&input, &out).unwrap();
    let bytes = fs::read(&out).unwrap();
    let tile: NetworkTile = serde_json::from_slice(&bytes).unwrap();
    // 150 since the pieces of a block are one road (153 before)
    assert!(tile.network.roads.len() > 140, "{} roads", tile.network.roads.len());
    let (network, frame) = citymodel_builder::importer::import_whole(&fs::read(&input).unwrap()).unwrap();
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
fn import_writes_the_network_of_the_input_as_compact_json() {
    let dir = tempfile::tempdir().unwrap();
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/importer/data/plateau.osm");
    let out = dir.path().join("plateau.json");
    commands::import(&input, &out).unwrap();
    let bytes = fs::read(&out).unwrap();
    let network: Network = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(network, citymodel_builder::importer::import(&fs::read(&input).unwrap()).unwrap());
    assert!(!bytes.contains(&b'\n'), "compact JSON");
}

#[test]
fn import_says_what_it_cannot_read_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.json");
    let missing = dir.path().join("none.osm");
    let error = commands::import(&missing, &out).unwrap_err();
    assert!(error.starts_with(&format!("cannot read {}", missing.display())), "{error}");
    assert!(!out.exists());
    let junk = dir.path().join("junk.osm");
    fs::write(&junk, "not OSM").unwrap();
    let error = commands::import(&junk, &out).unwrap_err();
    assert!(error.starts_with(&format!("cannot import {}", junk.display())), "{error}");
    assert!(!out.exists());
}

#[test]
fn the_import_command_fails_with_the_error_on_stderr_and_no_output() {
    let dir = tempfile::tempdir().unwrap();
    let (missing, out) = (dir.path().join("none.osm"), dir.path().join("out.json"));
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_citymodel_builder")).arg("import").arg(&missing).arg(&out).output().unwrap();
    assert!(!run.status.success());
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains(&format!("cannot read {}", missing.display())), "{stderr}");
    assert!(!out.exists());

    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/importer/crossing.osm");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_citymodel_builder")).arg("import").arg(&input).arg(&out).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let network: Network = serde_json::from_slice(&fs::read(&out).unwrap()).unwrap();
    assert_eq!(network, citymodel_builder::importer::import(&fs::read(&input).unwrap()).unwrap());
}

#[test]
fn the_cli_parses_the_commands() {
    let parse = |args: &[&str]| Cli::try_parse_from(std::iter::once("citymodel_builder").chain(args.iter().copied()));
    assert!(parse(&["pack", "--area", "x", "--created", "0"]).is_ok());
    assert!(parse(&["pack"]).is_ok());
    assert!(parse(&["prepare"]).is_ok());
    assert!(parse(&["prepare", "montreal", "--refresh"]).is_ok());
    assert!(parse(&["tiles", "montreal", "work"]).is_ok());
    assert!(parse(&["tiles"]).is_err());
    // the tiles are no longer cut by osmium in batches
    assert!(parse(&["prepare", "--batch", "2"]).is_err());
    assert!(parse(&["tiles", "montreal", "--batch", "2"]).is_err());
    assert!(parse(&["fixture", "in.osm.pbf", "out.json"]).is_ok());
    assert!(parse(&["fixture", "in.osm.pbf"]).is_err());
    assert!(parse(&["import", "in.osm", "out.json"]).is_ok());
    assert!(parse(&["import", "in.osm"]).is_err());
    assert!(parse(&["basemap", "q.osm.pbf"]).is_err());
}

#[test]
fn tiles_writes_where_out_says_and_else_where_it_did() {
    let parse = |args: &[&str]| Cli::try_parse_from(std::iter::once("citymodel_builder").chain(args.iter().copied())).unwrap();
    match parse(&["tiles", "montreal", "work", "--out", "/tmp/metro"]).command {
        Command::Tiles { out, .. } => assert_eq!(out, Some(PathBuf::from("/tmp/metro"))),
        other => panic!("{other:?}"),
    }
    match parse(&["tiles", "montreal"]).command {
        Command::Tiles { out, workdir, .. } => assert_eq!((out, workdir), (None, None)),
        other => panic!("{other:?}"),
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap().to_path_buf();
    assert_eq!(commands::tiles_out(None), root.join("web/data/metro"));
    assert_eq!(commands::tiles_out(Some(PathBuf::from("/x/metro"))), PathBuf::from("/x/metro"));
}

/// The command the line `args` parses to, or clap's message.
fn parsed(args: &[&str]) -> Result<Command, String> {
    Cli::try_parse_from(std::iter::once("citymodel_builder").chain(args.iter().copied())).map(|cli| cli.command).map_err(|e| e.to_string())
}

#[test]
fn prepare_takes_a_city_and_the_extract_as_an_option() {
    match parsed(&["prepare", "montreal", "--extract", "q.osm.pbf", "--refresh"]) {
        Ok(Command::Prepare { city, extract, refresh }) => {
            assert_eq!((city.as_deref(), extract, refresh), (Some("montreal"), Some(PathBuf::from("q.osm.pbf")), true));
        }
        other => panic!("{other:?}"),
    }
    match parsed(&["prepare"]) {
        Ok(Command::Prepare { city, extract, refresh }) => assert_eq!((city, extract, refresh), (None, None, false)),
        other => panic!("{other:?}"),
    }
    assert!(parsed(&["prepare", "montreal", "q.osm.pbf"]).is_err());
}

#[test]
fn tiles_takes_a_city() {
    match parsed(&["tiles", "montreal", "work", "--extract", "q.osm.pbf", "--out", "o"]) {
        Ok(Command::Tiles { city, workdir, extract, out }) => {
            assert_eq!(city, "montreal");
            assert_eq!((workdir, extract, out), (Some(PathBuf::from("work")), Some(PathBuf::from("q.osm.pbf")), Some(PathBuf::from("o"))));
        }
        other => panic!("{other:?}"),
    }
    match parsed(&["tiles", "montreal"]) {
        Ok(Command::Tiles { city, workdir, extract, out }) => assert_eq!((city.as_str(), workdir, extract, out), ("montreal", None, None, None)),
        other => panic!("{other:?}"),
    }
    assert!(parsed(&["tiles"]).is_err());
}

#[test]
fn pack_takes_a_city() {
    match parsed(&["pack", "montreal"]) {
        Ok(Command::Pack(args)) => {
            assert_eq!(args.city.as_deref(), Some("montreal"));
            assert_eq!((args.area, args.out, args.metro, args.source), (None, None, None, None));
            assert_eq!((args.source_time, args.created), (0, None));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn tiles_run_takes_its_paths_from_the_city_unless_given() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_city(root, "testville", TESTVILLE);
    let run = commands::tiles_run(root, "testville", None, None).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(run.city.id, "testville");
    assert_eq!(run.source, root.join("data/testville-latest.osm.pbf"));
    assert_eq!(run.work, root.join("data/metro-work/testville"));

    // a path given is made absolute: one handed to a tool must not start with `-`
    let run = commands::tiles_run(root, "testville", Some("-q.osm.pbf".into()), Some("work".into())).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(run.source, std::path::absolute("-q.osm.pbf").unwrap());
    assert_eq!(run.work, std::path::absolute("work").unwrap());
}

#[test]
fn tiles_run_lists_the_known_cities() {
    let dir = tempfile::tempdir().unwrap();
    for id in ["beta", "alpha"] {
        write_city(dir.path(), id, &city_named(id));
    }
    let error = commands::tiles_run(dir.path(), "nowhere", None, None).err().expect("a city that is not there is refused");
    assert_eq!(error, r#"no city "nowhere" in cities/ (known cities: alpha, beta)"#);
}

/// `pack` with nothing given.
fn no_args() -> PackArgs {
    PackArgs { city: None, area: None, out: None, metro: None, source: None, source_time: 0, created: None }
}

/// `pack` of a fixture, without a city: the four flags it needs.
fn fixture_args() -> PackArgs {
    PackArgs {
        area: Some("plateau".into()),
        out: Some("/x/plateau.citymodel".into()),
        metro: Some("/x/metro".into()),
        source: Some("plateau.osm.pbf".into()),
        ..no_args()
    }
}

#[test]
fn pack_without_a_city_needs_its_four_flags() {
    let dir = tempfile::tempdir().unwrap();
    for id in ["beta", "alpha"] {
        write_city(dir.path(), id, &city_named(id));
    }
    assert!(commands::pack_job(dir.path(), fixture_args()).is_ok());
    let flags = ["--area", "--out", "--metro", "--source"];
    let without = |flag: &str| {
        let mut args = fixture_args();
        match flag {
            "--area" => args.area = None,
            "--out" => args.out = None,
            "--metro" => args.metro = None,
            _ => args.source = None,
        }
        args
    };
    for missing in flags {
        let error = commands::pack_job(dir.path(), without(missing)).err().unwrap_or_else(|| panic!("packed without {missing}"));
        for flag in flags {
            assert_eq!(error.contains(flag), flag == missing, "{missing} is missing: {error}");
        }
        assert!(error.contains("known cities: alpha, beta"), "{error}");
    }
    let error = commands::pack_job(dir.path(), no_args()).err().expect("packed with nothing given");
    assert!(flags.iter().all(|flag| error.contains(flag)), "{error}");
    assert!(error.contains("known cities: alpha, beta"), "{error}");
}

#[test]
fn pack_without_a_city_packs_a_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let metro = folders(dir.path(), FIXTURE, &["0_0"], &["0_0"]);
    let out = dir.path().join("areas/plateau.citymodel");
    // the line of scripts/e2e-archive.sh
    let line =
        ["pack", "--metro", metro.to_str().unwrap(), "--out", out.to_str().unwrap(), "--area", "plateau", "--source", "plateau.osm.pbf", "--created", "0"];
    let Ok(Command::Pack(args)) = parsed(&line) else { panic!("{:?}", parsed(&line)) };
    let job = commands::pack_job(dir.path(), args).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(job.grid, None);
    let (progress, _) = quiet_progress(commands::pack_steps(&job.out), dir.path());
    commands::run_pack(&progress, &job).unwrap();
    let header = header(&out);
    assert_eq!((header.area.as_str(), header.source.as_str(), header.created), ("plateau", "plateau.osm.pbf", 0));
    let (lon0, lat0, dlon, dlat) = FIXTURE;
    assert_eq!(header.grid, Grid { lon0, lat0, dlon, dlat, nx: 1, ny: 1 });
}

/// A root with the city Testville and, in `web/data/metro`, two tiles on the grid given.
fn testville_root(grid: (f64, f64, f64, f64)) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write_city(dir.path(), "testville", TESTVILLE);
    folders(&dir.path().join("web/data"), grid, &["0_0", "1_1"], &["0_0", "1_1"]);
    dir
}

const TESTVILLE_GRID: Grid = Grid { lon0: 10.0, lat0: 45.0, dlon: 0.02, dlat: 0.015, nx: 3, ny: 2 };

#[test]
fn pack_with_a_city_takes_its_name_paths_and_grid() {
    let dir = testville_root((10.0, 45.0, 0.02, 0.015));
    let root = dir.path();
    let job = commands::pack_job(root, PackArgs { city: Some("testville".into()), created: Some(7), ..no_args() }).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((job.area.as_str(), job.source.as_str(), job.source_time, job.created), ("testville", "testville-latest.osm.pbf", 0, 7));
    assert_eq!((&job.out, &job.metro), (&root.join("data/testville.citymodel"), &root.join("web/data/metro")));
    assert_eq!(job.grid, Some(TESTVILLE_GRID));
    let (progress, _) = quiet_progress(commands::pack_steps(&job.out), root);
    commands::run_pack(&progress, &job).unwrap();
    let header = header(&root.join("data/testville.citymodel"));
    assert_eq!((header.area.as_str(), header.source.as_str()), ("testville", "testville-latest.osm.pbf"));
    // the city's size, not the tiles' (2 by 2)
    assert_eq!(header.grid, TESTVILLE_GRID);

    // what is given wins
    let given = PackArgs { city: Some("testville".into()), source_time: 5, ..fixture_args() };
    let job = commands::pack_job(root, given).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((job.area.as_str(), job.source.as_str(), job.source_time), ("plateau", "plateau.osm.pbf", 5));
    assert_eq!((&job.out, &job.metro), (&PathBuf::from("/x/plateau.citymodel"), &PathBuf::from("/x/metro")));
    assert_eq!(job.grid, Some(TESTVILLE_GRID));

    let unknown = commands::pack_job(root, PackArgs { city: Some("nowhere".into()), ..no_args() }).err().expect("a city that is not there is refused");
    assert_eq!(unknown, r#"no city "nowhere" in cities/ (known cities: testville)"#);
}

#[test]
fn pack_with_a_city_refuses_another_grids_tiles() {
    // one number off, by however little
    for grid in [(10.0000000001, 45.0, 0.02, 0.015), (10.0, 45.0000000001, 0.02, 0.015), (10.0, 45.0, 0.0200000000001, 0.015), (10.0, 45.0, 0.02, 0.0257)] {
        let dir = testville_root(grid);
        let job = commands::pack_job(dir.path(), PackArgs { city: Some("testville".into()), ..no_args() }).unwrap_or_else(|e| panic!("{e}"));
        let (progress, _) = quiet_progress(commands::pack_steps(&job.out), dir.path());
        let error = commands::run_pack(&progress, &job).unwrap_err().lines.join(" ");
        assert!(error.contains(&dir.path().join("web/data/metro/index.json").display().to_string()), "{error}");
        assert!(error.contains("testville") && error.contains("grid"), "{error}");
        assert!(!job.out.exists(), "an archive was written of another grid's tiles");
    }
}
