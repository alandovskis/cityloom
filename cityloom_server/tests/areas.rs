//! The start-up sequence: the archive is verified and unpacked before the server listens.
use std::path::Path;
use std::time::Duration;

use cityloom_prepare::archive::{self, PackInputs};
use cityloom_prepare::metro_index::Grid;
use cityloom_server::config::Config;

const GRID: Grid = Grid { lon0: -73.6078, lat0: 45.5161, dlon: 0.0257, dlat: 0.018, nx: 4, ny: 3 };

fn pack_area(archives: &Path, tile: &[u8]) {
    let work = archives.parent().unwrap().join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("0_0.osm.pbf"), tile).unwrap();
    std::fs::write(work.join("montreal.pmtiles"), [7u8; 3000]).unwrap();
    let roads = vec![(0u16, 0u16, work.join("0_0.osm.pbf"))];
    let maps = vec![work.join("montreal.pmtiles")];
    let inputs = PackInputs { area: "montreal", source: "x", source_time: 0, created: 0, grid: GRID, roads: &roads, basemaps: &maps };
    archive::pack(&inputs, &archives.join("montreal.cityloom"), &mut |_, _| {}).unwrap();
}

fn config_with(tmp: &Path, data_root: &Path) -> Config {
    let vars = [
        ("CITYLOOM_AREAS", tmp.join("areas").display().to_string()),
        ("DATA_ROOT", data_root.display().to_string()),
        ("WEB_ROOT", tmp.display().to_string()),
        ("E2E_PORT", "0".to_string()),
    ];
    Config::from_lookup(&|k| vars.iter().find(|(name, _)| *name == k).map(|(_, v)| v.clone())).unwrap()
}

fn config(tmp: &Path) -> Config {
    config_with(tmp, &tmp.join("data"))
}

async fn run_refused(config: Config) -> String {
    // A bug that skipped the check would bind and serve for ever: the timeout turns that into a failure.
    tokio::time::timeout(Duration::from_secs(10), cityloom_server::run(config)).await.expect("run returned").unwrap_err().to_string()
}

#[tokio::test]
async fn run_refuses_a_missing_archive_and_a_tampered_one_before_it_listens() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("areas")).unwrap();
    assert!(run_refused(config(tmp.path())).await.contains("no .cityloom file"));
    pack_area(&tmp.path().join("areas"), b"a road tile of the plateau");
    let file = tmp.path().join("areas/montreal.cityloom");
    let mut bytes = std::fs::read(&file).unwrap();
    let at = bytes.windows(26).position(|w| w == b"a road tile of the plateau").unwrap();
    bytes[at] ^= 1;
    std::fs::write(&file, bytes).unwrap();
    let message = run_refused(config(tmp.path())).await;
    assert!(message.contains("roads/0_0.osm.pbf") && message.contains("checksum"), "{message}");
    assert!(!tmp.path().join("data/montreal").exists());
}

#[tokio::test]
async fn the_errors_name_the_variable_and_the_folder() {
    let tmp = tempfile::tempdir().unwrap();
    // The folder is missing.
    let message = run_refused(config(tmp.path())).await;
    assert!(message.contains("CITYLOOM_AREAS") && message.contains(tmp.path().join("areas").to_str().unwrap()), "{message}");
    // Two archives.
    std::fs::create_dir_all(tmp.path().join("areas")).unwrap();
    std::fs::write(tmp.path().join("areas/a.cityloom"), b"").unwrap();
    std::fs::write(tmp.path().join("areas/b.cityloom"), b"").unwrap();
    let message = run_refused(config(tmp.path())).await;
    assert!(message.contains("more than one .cityloom file") && message.contains("CITYLOOM_AREAS"), "{message}");
    // DATA_ROOT cannot be made: it would be inside a file.
    std::fs::remove_file(tmp.path().join("areas/b.cityloom")).unwrap();
    std::fs::remove_file(tmp.path().join("areas/a.cityloom")).unwrap();
    pack_area(&tmp.path().join("areas"), b"tile");
    std::fs::write(tmp.path().join("file"), b"x").unwrap();
    let message = run_refused(config_with(tmp.path(), &tmp.path().join("file/data"))).await;
    assert!(message.contains("DATA_ROOT") && message.contains("file/data"), "{message}");
}

#[tokio::test]
async fn start_up_sets_the_mount_the_router_will_use_and_reuses_it() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("areas")).unwrap();
    pack_area(&tmp.path().join("areas"), b"tile");
    let mut cfg = config(tmp.path());
    assert!(cfg.data_mount.is_none());
    cityloom_server::unpack_areas(&mut cfg).await.unwrap();
    let mount = cfg.data_mount.clone().unwrap();
    assert_eq!(mount, tmp.path().join("data/montreal"));
    assert!(mount.join("metro/index.json").is_file());
    assert!(mount.join("basemap/montreal.pmtiles").is_file());
    // A second start finds the same files (reused, not unpacked again) and sets the same mount.
    let mut again = config(tmp.path());
    cityloom_server::unpack_areas(&mut again).await.unwrap();
    assert_eq!(again.data_mount, Some(mount));
    // Without CITYLOOM_AREAS nothing is mounted and nothing is touched.
    let mut none = Config::from_lookup(&|_| None).unwrap();
    cityloom_server::unpack_areas(&mut none).await.unwrap();
    assert!(none.data_mount.is_none());
}

#[test]
fn a_good_archive_unpacks_into_the_mount_the_router_will_use() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("areas")).unwrap();
    pack_area(&tmp.path().join("areas"), b"tile");
    let cfg = config(tmp.path());
    let areas = cfg.areas.clone().unwrap();
    let unpacked = archive::unpack_folder(&areas.folder, &areas.data_root).unwrap();
    assert_eq!(unpacked.dir, tmp.path().join("data/montreal"));
    assert!(unpacked.dir.join("metro/index.json").is_file());
}
