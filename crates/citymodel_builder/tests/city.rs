//! A city is its config file (`cities/<id>.toml`): what a file must hold, and that nothing else names a city.
mod support;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cityloom_model::{Frame, NetworkTile};
use citymodel_builder::archive::read_index;
use citymodel_builder::city::{self, City};
use citymodel_builder::metro_index::{Grid, index_json, short_checksum};
use citymodel_builder::prepare::{self, PrepareArgs, PrepareJob};
use citymodel_builder::tiles::{self, Margin, TilesJob};
use sha2::{Digest, Sha256};
use support::{MONTREAL_GRID, MONTREAL_MARGIN, Reply, TESTVILLE, calls, city_named, fake_tool, quiet_progress, serve, write_city};
use tempfile::TempDir;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap().to_path_buf()
}

/// `prepare <id>` in `root`.
fn job(root: &Path, id: &str) -> Result<PrepareJob, String> {
    prepare::job(root, PrepareArgs { city: Some(id.into()), extract: None, refresh: false })
}

fn loaded(root: &Path, id: &str) -> PrepareJob {
    job(root, id).unwrap_or_else(|e| panic!("{e}"))
}

/// A root whose one city is Testville.
fn testville() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    write_city(dir.path(), "testville", TESTVILLE);
    dir
}

/// Testville's config with its line `key = …` changed to `line` (a key is in one section only).
fn with(key: &str, line: &str) -> String {
    let start = format!("{key} = ");
    assert_eq!(TESTVILLE.lines().filter(|l| l.starts_with(&start)).count(), 1, "{key}");
    let text: Vec<&str> = TESTVILLE.lines().map(|l| if l.starts_with(&start) { line } else { l }).collect();
    text.join("\n")
}

/// Testville's config without its line `key = …`.
fn without(key: &str) -> String {
    with(key, "")
}

fn set(key: &str, value: &str) -> String {
    with(key, &format!("{key} = {value}"))
}

/// What is said of `text` as `cities/<id>.toml`, after the file's path. The unchanged config loads first, so what is
/// refused is the change.
fn refusal_of(id: &str, text: &str) -> String {
    let dir = testville();
    assert_eq!(loaded(dir.path(), "testville").city.id, "testville");
    let path = write_city(dir.path(), id, text);
    let error = job(dir.path(), id).err().unwrap_or_else(|| panic!("accepted: {text}"));
    let start = format!("{}: ", path.display());
    assert!(error.starts_with(&start), "{error:?} does not start with the file's path");
    error[start.len()..].to_string()
}

/// `text` is refused and its field named, by the checks after parsing.
fn refused(text: &str, field: &str) {
    let said = refusal_of("testville", text);
    assert!(said.starts_with(&format!("{field}: ")), "{said:?} does not name {field}: {text}");
}

/// `text` is refused by the parser, whose message shows `shown`.
fn refused_by_parser(text: &str, shown: &str) {
    let said = refusal_of("testville", text);
    assert!(said.contains(shown), "{said:?} does not show {shown}: {text}");
}

fn accepted(text: &str) {
    let dir = testville();
    write_city(dir.path(), "testville", text);
    loaded(dir.path(), "testville");
}

// ---- Montréal, and every city of the repository ----

#[test]
fn montreals_config_is_its_numbers_bit_for_bit() {
    let city = city::load(&repository(), "montreal").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(city.id, "montreal");
    assert_eq!(city.extract_url, "https://download.geofabrik.de/north-america/canada/quebec/quebec-latest.osm.pbf");
    assert_eq!(city.extract_name(), "quebec-latest.osm.pbf");
    let bits = |numbers: [f64; 6]| numbers.map(f64::to_bits);
    let (g, m) = (city.grid, city.margin);
    assert_eq!(bits([g.west, g.south, g.tile_width, g.tile_height, m.lon, m.lat]), bits([-74.4, 45.2, 0.0257, 0.018, 0.0064, 0.0045]));
    assert_eq!((g.columns, g.rows), (51, 34));
    let grid = g.grid();
    assert_eq!(bits([grid.lon0, grid.lat0, grid.dlon, grid.dlat, 0.0, 0.0]), bits([-74.4, 45.2, 0.0257, 0.018, 0.0, 0.0]));
    assert_eq!((grid.nx, grid.ny), (51, 34));
    assert_eq!(tiles::metro_bounds(&grid, &m), "-74.4064,45.1955,-73.08290000000001,45.816500000000005");
}

/// Part of the Plateau, in Montréal, which the fake osmium's `merge` writes as the roads.
fn plateau() -> PathBuf {
    repository().join("tests/e2e/fixtures/plateau.osm.pbf")
}

fn fake_osmium(dir: &Path) -> String {
    let body = format!(r#"if [ "$1" = merge ]; then cp '{}' "$5"; fi"#, plateau().display());
    fake_tool(dir, "osmium", &body).display().to_string()
}

/// Every file the plateau's roads make on this grid with this margin, by name: each tile's roads are those that reach
/// its core widened by the margin, in the whole area's frame. Worked out from the numbers alone.
fn expected_files(grid: &Grid, margin: &Margin) -> BTreeMap<String, Vec<u8>> {
    let (network, frame) = citymodel_builder::importer::import_whole(&fs::read(plateau()).unwrap()).unwrap();
    let mut files = BTreeMap::new();
    for x in 0..grid.nx {
        for y in 0..grid.ny {
            let (i, j) = (f64::from(x), f64::from(y));
            let reach = Frame {
                south: grid.lat0 + j * grid.dlat - margin.lat,
                west: grid.lon0 + i * grid.dlon - margin.lon,
                north: grid.lat0 + (j + 1.0) * grid.dlat + margin.lat,
                east: grid.lon0 + (i + 1.0) * grid.dlon + margin.lon,
            };
            let held = network.reaching(&frame, &reach);
            if !held.roads.is_empty() {
                files.insert(format!("{x}_{y}"), serde_json::to_vec(&NetworkTile { frame, network: held }).unwrap());
            }
        }
    }
    assert!(files.len() >= 2, "{} tiles", files.len());
    // by name as text, as the map is ordered
    let named: Vec<(String, String)> = files
        .iter()
        .map(|(name, bytes)| {
            let digest: [u8; 32] = Sha256::digest(bytes).into();
            (name.clone(), short_checksum(&digest))
        })
        .collect();
    let index = index_json(grid, &named).into_bytes();
    let mut all: BTreeMap<String, Vec<u8>> = files.into_iter().map(|(name, bytes)| (format!("{name}.json"), bytes)).collect();
    all.insert("index.json".into(), index);
    all
}

fn files_in(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (e.file_name().to_string_lossy().into_owned(), fs::read(e.path()).unwrap())
        })
        .collect()
}

/// The same files with the same bytes (a tile is too long to print).
fn assert_same_files(made: &BTreeMap<String, Vec<u8>>, expected: &BTreeMap<String, Vec<u8>>) {
    assert_eq!(made.keys().collect::<Vec<_>>(), expected.keys().collect::<Vec<_>>());
    for (name, bytes) in expected {
        assert!(made[name] == *bytes, "{name} is not the bytes expected");
    }
}

#[test]
fn montreals_config_makes_the_tiles_and_index_of_the_literal_grid_and_margin() {
    let city = city::load(&repository(), "montreal").unwrap_or_else(|e| panic!("{e}"));
    let dir = tempfile::tempdir().unwrap();
    let (source, work, out) = (dir.path().join("q.osm.pbf"), dir.path().join("work"), dir.path().join("out"));
    fs::write(&source, "x").unwrap();
    let osmium = fake_osmium(dir.path());
    let grid = city.grid.grid();
    let job = TilesJob {
        source: &source,
        work: &work,
        out: &out,
        osmium: &osmium,
        grid,
        margin: city.margin,
        url: &city.extract_url,
        extract_name: city.extract_name(),
    };
    let (progress, _) = quiet_progress(tiles::steps(&grid), dir.path());
    tiles::run(&progress, &job).unwrap();
    progress.close();
    assert_same_files(&files_in(&out), &expected_files(&MONTREAL_GRID, &MONTREAL_MARGIN));
}

#[test]
fn every_city_file_loads() {
    let mut ids = Vec::new();
    if let Ok(entries) = fs::read_dir(repository().join("cities")) {
        for entry in entries {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "toml") {
                ids.push(path.file_stem().unwrap().to_str().unwrap().to_string());
            }
        }
    }
    assert!(!ids.is_empty(), "cities/ holds no city");
    for id in ids {
        assert_eq!(loaded(&repository(), &id).city.id, id);
    }
}

// ---- which city ----

/// A root with the cities alpha and beta, and files that are not cities.
fn two_cities() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    for id in ["beta", "alpha"] {
        write_city(dir.path(), id, &city_named(id));
    }
    fs::write(dir.path().join("cities/README.md"), "# Cities\n").unwrap();
    fs::write(dir.path().join("cities/gamma.toml.bak"), city_named("gamma")).unwrap();
    // valid, and not in cities/
    fs::write(dir.path().join("x.toml"), city_named("x")).unwrap();
    dir
}

#[test]
fn the_known_cities_are_the_config_files_in_order() {
    assert_eq!(city::known(two_cities().path()), ["alpha", "beta"]);
    assert!(city::known(tempfile::tempdir().unwrap().path()).is_empty());
}

#[test]
fn no_city_lists_the_known_cities() {
    let dir = two_cities();
    let said = "which city? (known cities: alpha, beta)";
    assert_eq!(city::choose(dir.path(), None), Err(said.to_string()));
    let error = prepare::job(dir.path(), PrepareArgs { city: None, extract: None, refresh: true }).err();
    assert_eq!(error.as_deref(), Some(said));
    assert_eq!(city::choose(dir.path(), Some("beta")).map(|c| c.id), Ok("beta".to_string()));
}

#[test]
fn an_unknown_city_lists_the_known_cities() {
    let dir = two_cities();
    // an id is compared with the known ones, never made into a path
    for id in ["nowhere", "../x", "alpha.toml", "ALPHA", ""] {
        let said = format!("no city \"{id}\" in cities/ (known cities: alpha, beta)");
        assert_eq!(city::load(dir.path(), id), Err(said.clone()));
        assert_eq!(city::choose(dir.path(), Some(id)), Err(said.clone()));
        assert_eq!(job(dir.path(), id).err(), Some(said));
    }
}

#[test]
fn no_cities_at_all_is_said() {
    let empty = tempfile::tempdir().unwrap();
    fs::create_dir(empty.path().join("cities")).unwrap();
    let none = tempfile::tempdir().unwrap();
    for root in [empty.path(), none.path()] {
        for id in [None, Some("alpha")] {
            let error = city::choose(root, id).unwrap_err();
            assert!(error.contains("cities/ holds no city"), "{error}");
        }
    }
}

#[test]
fn a_bad_request_for_a_city_downloads_and_writes_nothing() {
    let asked = Arc::new(AtomicUsize::new(0));
    let count = asked.clone();
    let (url, _) = serve(move |_| {
        count.fetch_add(1, Ordering::SeqCst);
        Reply::Status(500)
    });
    let dir = tempfile::tempdir().unwrap();
    let at_server = TESTVILLE.replace("http://127.0.0.1:1", &url);
    write_city(dir.path(), "testville", &at_server);
    write_city(dir.path(), "broken", &at_server.replace("testville", "broken").replace("columns = 3", "columns = 0"));
    for id in [None, Some("nowhere"), Some("broken")] {
        let args = PrepareArgs { city: id.map(str::to_string), extract: None, refresh: true };
        assert_eq!(prepare::run_at(dir.path(), args), Err(1), "{id:?}");
        assert_eq!(asked.load(Ordering::SeqCst), 0, "{id:?} was downloaded");
        let left: Vec<_> = fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(left, ["cities"], "{id:?} wrote something");
    }
}

// ---- what a config must hold ----

#[test]
fn a_missing_top_level_field_is_named() {
    refused_by_parser(&without("id"), "`id`");
    refused_by_parser(&without("extract_url"), "`extract_url`");
    let cut = |from: &str, to: &str| {
        let start = TESTVILLE.find(from).unwrap();
        let end = TESTVILLE[start..].find(to).map_or(TESTVILLE.len(), |n| start + n);
        format!("{}{}", &TESTVILLE[..start], &TESTVILLE[end..])
    };
    refused_by_parser(&cut("[grid]", "[margin]"), "`grid`");
    refused_by_parser(&cut("[margin]", "[nothing after]"), "`margin`");
}

#[test]
fn a_missing_grid_field_is_named() {
    for key in ["west", "south", "tile_width", "tile_height", "columns", "rows"] {
        refused_by_parser(&without(key), &format!("`{key}`"));
    }
}

#[test]
fn a_missing_margin_field_is_named() {
    for key in ["lon", "lat"] {
        refused_by_parser(&without(key), &format!("`{key}`"));
    }
}

#[test]
fn an_unknown_top_level_field_is_named() {
    refused_by_parser(&format!("country = \"nowhere\"\n{TESTVILLE}"), "country");
    refused_by_parser(&format!("{TESTVILLE}\n[mapping]\n"), "mapping");
}

#[test]
fn an_unknown_grid_field_is_named() {
    refused_by_parser(&TESTVILLE.replace("[grid]", "[grid]\nmeridian = 1.0"), "meridian");
}

#[test]
fn an_unknown_margin_field_is_named() {
    refused_by_parser(&TESTVILLE.replace("[margin]", "[margin]\nmetres = 500.0"), "metres");
}

#[test]
fn a_value_of_the_wrong_type_is_named() {
    for (key, value) in [
        ("id", "7"),
        ("extract_url", "true"),
        ("west", "\"ten\""),
        ("south", "\"45.0\""),
        ("tile_width", "[0.02]"),
        ("tile_height", "false"),
        ("columns", "70000"),
        ("columns", "3.5"),
        ("rows", "-2"),
        ("lon", "\"0.008\""),
        ("lat", "{}"),
    ] {
        // the line itself is shown
        refused_by_parser(&set(key, value), &format!("{key} = {value}"));
    }
}

#[test]
fn a_file_that_is_not_toml_is_named() {
    refused_by_parser(&with("south", "south = = 45.0"), "south = = 45.0");
    refused_by_parser(&with("tile_width", "tile_width 0.02"), "tile_width 0.02");
    // a field given twice
    refused_by_parser(&with("rows", "rows = 2\nrows = 3"), "rows");
}

#[test]
fn an_id_the_archive_refuses_is_named() {
    let long = "a".repeat(65);
    for id in ["test_ville", "Testcity", "test.ville", "test ville", "montréal", &long] {
        let said = refusal_of(id, &city_named(id).replace(&format!("/{id}-latest"), "/plain-latest"));
        assert!(said.starts_with("id: "), "{said}");
    }
    let dir = testville();
    let longest = "a".repeat(64);
    write_city(dir.path(), &longest, &city_named(&longest));
    assert_eq!(loaded(dir.path(), &longest).city.id, longest);
}

#[test]
fn an_id_that_is_not_the_files_name_is_named() {
    refused(&set("id", "\"another\""), "id");
    refused(&set("id", "\"\""), "id");
}

#[test]
fn a_url_that_is_not_http_is_named() {
    for url in [
        "",
        "testville-latest.osm.pbf",
        "/extracts/testville-latest.osm.pbf",
        "127.0.0.1:1/testville-latest.osm.pbf",
        "ftp://127.0.0.1:1/testville-latest.osm.pbf",
        "file:///extracts/testville-latest.osm.pbf",
        // no host
        "http:///testville-latest.osm.pbf",
        "https://",
        // no path
        "http://127.0.0.1:1",
        "http://testville-latest.osm.pbf",
    ] {
        refused(&set("extract_url", &format!("\"{url}\"")), "extract_url");
    }
    accepted(&set("extract_url", "\"https://127.0.0.1:1/testville-latest.osm.pbf\""));
}

#[test]
fn a_url_whose_last_part_is_not_a_plain_osm_pbf_name_is_named() {
    for last in [
        "testville.osm.pbf?fresh=1",
        "testville.osm.pbf?from=/other.osm.pbf",
        "testville.osm.pbf#top",
        "",
        "..",
        "testville.osm.bz2",
        "testville.pbf",
        "testville.osm.pbf.md5",
        "testville.OSM.PBF",
        ".osm.pbf",
        "..osm.pbf",
        "-testville.osm.pbf",
        "_testville.osm.pbf",
        "test ville.osm.pbf",
        "test%20ville.osm.pbf",
        "test\\\\ville.osm.pbf",
        "montréal.osm.pbf",
    ] {
        refused(&set("extract_url", &format!("\"http://127.0.0.1:1/extracts/{last}\"")), "extract_url");
    }
    accepted(&set("extract_url", "\"http://127.0.0.1:1/extracts/9.Test_ville-2.osm.pbf\""));
}

#[test]
fn a_number_that_is_not_finite_is_named() {
    for (table, key) in [("grid", "west"), ("grid", "south"), ("grid", "tile_width"), ("grid", "tile_height"), ("margin", "lon"), ("margin", "lat")] {
        for value in ["nan", "inf", "-inf"] {
            refused(&set(key, value), &format!("{table}.{key}"));
        }
    }
}

#[test]
fn a_grid_of_no_columns_is_named() {
    refused(&set("columns", "0"), "grid.columns");
}

#[test]
fn a_grid_of_no_rows_is_named() {
    refused(&set("rows", "0"), "grid.rows");
}

#[test]
fn a_tile_of_no_size_is_named() {
    for value in ["0.0", "-0.02"] {
        refused(&set("tile_width", value), "grid.tile_width");
        refused(&set("tile_height", value), "grid.tile_height");
    }
}

#[test]
fn a_grid_off_the_earth_is_named() {
    for value in ["-180.1", "180.1"] {
        refused(&set("west", value), "grid.west");
    }
    for value in ["-90.1", "90.1"] {
        refused(&set("south", value), "grid.south");
    }
    // 3 tiles of 0.02 to 180.05, 2 of 0.015 to 90.02
    refused(&set("west", "179.99"), "grid.columns");
    refused(&set("south", "89.99"), "grid.rows");
    // at the edges and not past them
    accepted(&set("west", "-180.0"));
    accepted(&set("west", "179.0").replace("tile_width = 0.02", "tile_width = 0.25").replace("columns = 3", "columns = 4"));
}

#[test]
fn a_margin_narrower_than_an_area_reaches_is_named() {
    // an area reaches 400 m: 0.0035 degrees of latitude are 389 m and 0.0037 are 411 m
    refused(&set("lat", "0.0035"), "margin.lat");
    accepted(&set("lat", "0.0037"));
    // of longitude, at 45 degrees, 0.0049 are 385 m and 0.0053 are 416 m
    refused(&set("lon", "0.0049"), "margin.lon");
    accepted(&set("lon", "0.0053"));
}

#[test]
fn a_negative_margin_is_named() {
    for value in ["-0.008", "0.0", "-1.0"] {
        refused(&set("lat", value), "margin.lat");
        refused(&set("lon", value), "margin.lon");
    }
}

#[test]
fn a_margin_is_measured_at_the_grids_most_poleward_edge() {
    let from = |south: &str, rows: &str| {
        let text = set("south", south).replace("lon = 0.008", "lon = 0.0078").replace("tile_height = 0.015", "tile_height = 0.5");
        text.replace("rows = 2", &format!("rows = {rows}"))
    };
    // 0.0078 degrees of longitude are 434 m at 60 degrees, 428 m at 60.5 and 353 m at 66
    accepted(&from("60.0", "1"));
    refused(&from("60.0", "12"), "margin.lon");
    accepted(&from("-60.5", "1"));
    refused(&from("-66.0", "12"), "margin.lon");
}

#[test]
fn the_reach_is_the_editors() {
    assert_eq!(tiles::AREA_REACH_M, cityloom_editor::place::area::HALF_M);
}

// ---- each city's own files ----

#[test]
fn two_cities_have_their_own_extract_work_folder_and_archive() {
    let dir = two_cities();
    let root = dir.path();
    for id in ["alpha", "beta"] {
        let job = loaded(root, id);
        assert_eq!(job.extract, root.join(format!("data/{id}-latest.osm.pbf")));
        assert_eq!(job.work, root.join(format!("data/metro-work/{id}")));
        assert_eq!(job.archive, root.join(format!("data/{id}.citymodel")));
        assert_eq!((&job.metro, &job.root), (&root.join("web/data/metro"), &root.to_path_buf()));
        assert_eq!((job.osmium.as_str(), job.refresh), ("osmium", false));
        let city: &City = &job.city;
        assert_eq!(city.extract_name(), format!("{id}-latest.osm.pbf"));
        assert_eq!((city.extract(root), city.work(root), city.archive(root)), (job.extract.clone(), job.work.clone(), job.archive.clone()));
    }
}

#[test]
fn cities_with_the_same_url_share_their_extract() {
    let dir = two_cities();
    write_city(dir.path(), "beta", &city_named("beta").replace("beta-latest", "alpha-latest"));
    let (alpha, beta) = (loaded(dir.path(), "alpha"), loaded(dir.path(), "beta"));
    assert_eq!(alpha.city.extract_url, beta.city.extract_url);
    assert_eq!(alpha.extract, dir.path().join("data/alpha-latest.osm.pbf"));
    assert_eq!(alpha.extract, beta.extract);
    assert_ne!(alpha.work, beta.work);
    assert_ne!(alpha.archive, beta.archive);
}

#[test]
fn different_urls_of_one_extract_name_are_refused() {
    // the same name, and names a disk that ignores case takes for the same
    for name in ["alpha-latest", "Alpha-Latest"] {
        let dir = two_cities();
        let root = dir.path();
        let path = |id: &str| root.join(format!("cities/{id}.toml")).display().to_string();
        assert_ne!(loaded(root, "alpha").extract, loaded(root, "beta").extract);
        write_city(root, "beta", &city_named("beta").replace("extracts/beta-latest", &format!("elsewhere/{name}")));
        for (id, other) in [("alpha", "beta"), ("beta", "alpha")] {
            let error = job(root, id).err().unwrap_or_else(|| panic!("{id} would overwrite {other}'s extract"));
            assert!(error.starts_with(&format!("{}: extract_url: ", path(id))), "{error}");
            assert!(error.contains(&path(other)), "{error}");
        }
    }
    // a file that cannot be read as a city is refused itself, and is no reason to refuse another
    let dir = two_cities();
    write_city(dir.path(), "beta", "not a city");
    loaded(dir.path(), "alpha");
    assert!(job(dir.path(), "beta").is_err());
}

#[test]
fn a_second_city_is_prepared_from_its_config_alone() {
    let asked: Arc<Mutex<Vec<String>>> = Arc::default();
    let paths = asked.clone();
    let (url, _) = serve(move |request| {
        paths.lock().unwrap().push(request.path.clone());
        Reply::Body { status: 200, body: b"an extract".to_vec(), content_length: Some(10) }
    });
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let config = format!(
        r#"
id = "second-city"
extract_url = "{url}/somewhere/second-latest.osm.pbf"

[grid]
west = -73.6078
south = 45.5161
tile_width = 0.02
tile_height = 0.015
columns = 3
rows = 2

[margin]
lon = 0.008
lat = 0.006
"#
    );
    write_city(root, "second-city", &config);
    let osmium = fake_osmium(root);
    let job = PrepareJob { osmium, ..loaded(root, "second-city") };

    let subjects: Vec<String> = prepare::steps(&job).into_iter().map(|s| s.subject).collect();
    for label in ["second-latest.osm.pbf", "cutting 6 tiles", "second-city.citymodel"] {
        assert!(subjects.iter().any(|s| s == label), "no {label} in {subjects:?}");
    }
    let (progress, _) = quiet_progress(prepare::steps(&job), root);
    prepare::run_job(&progress, &job).unwrap();
    progress.close();

    assert_eq!(*asked.lock().unwrap(), ["/somewhere/second-latest.osm.pbf"]);
    assert_eq!(fs::read(root.join("data/second-latest.osm.pbf")).unwrap(), b"an extract");
    let (data, work) = (root.join("data"), root.join("data/metro-work/second-city"));
    assert_eq!(
        calls(root, "osmium")[0],
        format!("extract -b -73.6158,45.5101,-73.5398,45.5521 {}/second-latest.osm.pbf -o {}/metro.osm.pbf --overwrite", data.display(), work.display())
    );
    assert!(work.join("roads.osm.pbf").is_file());

    let grid = Grid { lon0: -73.6078, lat0: 45.5161, dlon: 0.02, dlat: 0.015, nx: 3, ny: 2 };
    let expected = expected_files(&grid, &Margin { lon: 0.008, lat: 0.006 });
    // the margin shows in the tiles: another one makes others
    assert_ne!(expected, expected_files(&grid, &MONTREAL_MARGIN));
    assert_same_files(&files_in(&root.join("web/data/metro")), &expected);

    let archive = root.join("data/second-city.citymodel");
    let mut tar = tar::Archive::new(fs::File::open(&archive).unwrap());
    let mut first = tar.entries().unwrap().next().unwrap().unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut first, &mut bytes).unwrap();
    let header = read_index(&bytes).unwrap().header;
    assert_eq!((header.area.as_str(), header.source.as_str()), ("second-city", "second-latest.osm.pbf"));
    assert_eq!(header.grid, grid);
}

#[test]
fn the_extract_option_replaces_the_citys_extract() {
    let dir = testville();
    let root = dir.path();
    let given = |path: &str| {
        let args = PrepareArgs { city: Some("testville".into()), extract: Some(path.into()), refresh: true };
        prepare::job(root, args).unwrap_or_else(|e| panic!("{e}"))
    };
    let job = given("-kept.osm.pbf");
    // absolute, so a tool it is handed to cannot read it as an option
    assert_eq!(job.extract, std::path::absolute("-kept.osm.pbf").unwrap());
    assert_eq!((job.work, job.archive), (root.join("data/metro-work/testville"), root.join("data/testville.citymodel")));
    assert_eq!((job.city.id.as_str(), job.refresh), ("testville", true));
    assert_eq!(given("/x/kept.osm.pbf").extract, PathBuf::from("/x/kept.osm.pbf"));
}

// ---- no city in the source ----

/// Rust source without its comments and its `#[cfg(test)]` items.
fn product_code(source: &str) -> String {
    without_test_items(&marked(source))
}

/// Each character of `source` outside its comments with whether it is code (true) or in a string or character
/// literal (false); a block comment leaves one space.
fn marked(source: &str) -> Vec<(char, bool)> {
    let c: Vec<char> = source.chars().collect();
    let mut kept: Vec<(char, bool)> = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if at(&c, i, "//") {
            i = line_end(&c, i);
        } else if at(&c, i, "/*") {
            i = block_comment_end(&c, i);
            kept.push((' ', true));
        } else if let Some(end) = literal_end(&c, i) {
            kept.extend(c[i..end.min(c.len())].iter().map(|ch| (*ch, false)));
            i = end;
        } else {
            kept.push((c[i], true));
            i += 1;
        }
    }
    kept
}

/// Whether `text` starts at `i` in `c`.
fn at(c: &[char], i: usize, text: &str) -> bool {
    text.chars().enumerate().all(|(k, t)| c.get(i + k) == Some(&t))
}

fn line_end(c: &[char], mut i: usize) -> usize {
    while i < c.len() && c[i] != '\n' {
        i += 1;
    }
    i
}

/// Past the block comment opened at `i`, its nested comments with it.
fn block_comment_end(c: &[char], mut i: usize) -> usize {
    let mut depth = 0;
    loop {
        if at(c, i, "/*") {
            (depth, i) = (depth + 1, i + 2);
        } else if at(c, i, "*/") {
            (depth, i) = (depth - 1, i + 2);
        } else {
            i += 1;
        }
        if depth == 0 || i >= c.len() {
            return i;
        }
    }
}

/// Past the string or character literal that starts at `i`, if one does.
fn literal_end(c: &[char], i: usize) -> Option<usize> {
    if let Some(hashes) = raw_string_hashes(c, i) {
        Some(raw_string_end(c, i, hashes))
    } else if c[i] == '"' {
        Some(string_end(c, i))
    } else if c[i] == '\'' && c.get(i + 1) == Some(&'\\') {
        Some(escaped_char_end(c, i))
    } else if c[i] == '\'' && c.get(i + 2) == Some(&'\'') {
        Some(i + 3)
    } else {
        None
    }
}

/// The number of `#` of the raw string (`r"`, `r#"`, `br"`) that starts at `i`, if one does.
fn raw_string_hashes(c: &[char], i: usize) -> Option<usize> {
    let hashes = c[i + 1..].iter().take_while(|h| **h == '#').count();
    let opens = c[i] == 'r' && c.get(i + 1 + hashes) == Some(&'"');
    let in_a_word = i > 0 && (c[i - 1].is_alphanumeric() || c[i - 1] == '_');
    (opens && (!in_a_word || byte_prefix(c, i))).then_some(hashes)
}

/// Whether the `r` at `i` follows a lone `b` (`br"…"`); `i` is past the first character.
fn byte_prefix(c: &[char], i: usize) -> bool {
    c[i - 1] == 'b' && (i < 2 || !c[i - 2].is_alphanumeric())
}

fn raw_string_end(c: &[char], i: usize, hashes: usize) -> usize {
    let close = format!("\"{}", "#".repeat(hashes));
    let mut i = i + 2 + hashes;
    while i < c.len() && !at(c, i, &close) {
        i += 1;
    }
    i + close.len()
}

fn string_end(c: &[char], i: usize) -> usize {
    let mut i = i + 1;
    while i < c.len() && c[i] != '"' {
        i += if c[i] == '\\' { 2 } else { 1 };
    }
    i + 1
}

fn escaped_char_end(c: &[char], i: usize) -> usize {
    let mut i = i + 3;
    while i < c.len() && c[i] != '\'' {
        i += 1;
    }
    i + 1
}

/// The characters of `kept` without its `#[cfg(test)]` items, looked for in its code only.
fn without_test_items(kept: &[(char, bool)]) -> String {
    let code: Vec<char> = kept.iter().map(|(ch, is_code)| if *is_code { *ch } else { '\0' }).collect();
    let marker: Vec<char> = "#[cfg(test)]".chars().collect();
    let (mut out, mut i) = (String::new(), 0);
    while i < kept.len() {
        if code[i..].starts_with(&marker) {
            i = item_end(&code, i + marker.len());
        } else {
            out.push(kept[i].0);
            i += 1;
        }
    }
    out
}

/// Past the item that starts at `i`: to its `;`, or the end of its braces.
fn item_end(code: &[char], mut i: usize) -> usize {
    let mut depth = 0;
    while i < code.len() {
        let ch = code[i];
        i += 1;
        match ch {
            '{' | '(' | '[' => depth += 1,
            '}' | ')' | ']' => depth -= 1,
            _ => {}
        }
        if depth == 0 && (ch == ';' || ch == '}') {
            break;
        }
    }
    i
}

/// Whether `text` holds `name` as a word or a part of a name (`paris`, `PARIS_GRID`, `ParisGrid`; not `comparison`).
fn names(text: &str, name: &str) -> bool {
    let t: Vec<char> = text.chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    let same = |a: char, b: char| a.to_lowercase().eq(b.to_lowercase());
    (0..t.len().saturating_sub(n.len() - 1)).any(|i| {
        let end = i + n.len();
        let starts = i == 0 || !t[i - 1].is_alphanumeric() || (t[i - 1].is_lowercase() && t[i].is_uppercase());
        let ends = end == t.len() || !t[end].is_alphanumeric() || (t[end - 1].is_lowercase() && t[end].is_uppercase());
        starts && ends && t[i..end].iter().zip(&n).all(|(a, b)| same(*a, *b))
    })
}

fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn no_city_is_named_in_the_source() {
    let sample = r##"// a
const U: &str = "http://b"; /* c /* d */ */ const Q: char = '"'; const R: &str = r#"//e"#;
fn f<'a>(x: &'a str) -> u8 { b'{' } // g
#[cfg(test)]
#[allow(x)]
mod tests { const T: &str = "}h"; fn t() { } }
const Z: &str = "i";"##;
    let code = product_code(sample);
    for (text, kept) in [
        ("// a", false),
        ("http://b", true),
        ("/* c", false),
        ("*/", false),
        ("//e", true),
        ("fn f<'a>", true),
        ("// g", false),
        ("h\"", false),
        ("\"i\"", true),
    ] {
        assert_eq!(code.contains(text), kept, "{text}: {code}");
    }
    assert!(names("PARIS_GRID", "paris") && names("the ParisGrid", "paris") && names("\"paris-latest\"", "paris-latest"));
    assert!(!names("comparison", "paris"));

    let mut files = Vec::new();
    sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    assert!(files.len() > 10, "{files:?}");
    let mut cities = Vec::new();
    if let Ok(entries) = fs::read_dir(repository().join("cities")) {
        for path in entries.map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "toml")) {
            let id = path.file_stem().unwrap().to_str().unwrap().to_string();
            let city = city::load(&repository(), &id).unwrap_or_else(|e| panic!("{e}"));
            cities.push(id);
            cities.push(city.extract_name().strip_suffix(".osm.pbf").unwrap().to_string());
        }
    }
    let mut found = Vec::new();
    for path in &files {
        let code = product_code(&fs::read_to_string(path).unwrap());
        let lower = code.to_lowercase();
        for word in ["montreal", "montréal", "quebec", "québec", "canada", "geofabrik"] {
            if lower.contains(word) {
                found.push(format!("{} names {word}", path.display()));
            }
        }
        for name in &cities {
            if names(&code, name) {
                found.push(format!("{} names the city or extract {name}", path.display()));
            }
        }
    }
    assert!(found.is_empty(), "{found:#?}");
}
