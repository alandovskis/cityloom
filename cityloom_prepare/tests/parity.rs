//! Until the Python scripts are removed: the Rust `tiles` step writes the files `scripts/metro_tiles.py` writes,
//! byte for byte, from the same input. Needs osmium and python3; run with
//! `cargo test -p cityloom_prepare --test parity -- --ignored --nocapture`.
mod support;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cityloom_prepare::tiles::{self, TilesJob};
use support::quiet_progress;

const BATCH: usize = 500;

/// The Python side: the script's own `run`, with its output folder moved (the module reads `OUT` as a global).
const PYTHON: &str = "import sys; from pathlib import Path; sys.path.insert(0, sys.argv[5]); \
import metro_tiles as m; from include.progress import Progress; \
m.OUT = Path(sys.argv[1]); p = Progress(m.STEPS, Path(sys.argv[2]), m.ROOT); \
m.run(p, Path(sys.argv[3]), Path(sys.argv[4]), 500)";

fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), fs::read(&p).unwrap())).collect()
}

#[test]
#[ignore = "needs osmium and python3, and the scripts that Task 14 removes"]
fn rust_and_python_cut_the_same_tiles_from_the_plateau_fixture() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let source = root.join("e2e/fixtures/plateau.osm.pbf");
    let dir = tempfile::tempdir().unwrap();
    let (py_out, py_work, py_logs) = (dir.path().join("py"), dir.path().join("py-work"), dir.path().join("py-logs"));
    let (rs_out, rs_work) = (dir.path().join("rs"), dir.path().join("rs-work"));

    let status = Command::new("python3").args(["-c", PYTHON]).arg(&py_out).arg(&py_logs).arg(&source).arg(&py_work).arg(root.join("scripts")).status().unwrap();
    assert!(status.success(), "scripts/metro_tiles.py failed");

    let (progress, _) = quiet_progress(tiles::steps(), dir.path());
    tiles::run(&progress, &TilesJob { source: &source, work: &rs_work, out: &rs_out, batch: BATCH, osmium: "osmium" }).unwrap();

    let (py, rs) = (files(&py_out), files(&rs_out));
    let tile_count = py.keys().filter(|n| n.ends_with(".osm.pbf")).count();
    println!("{tile_count} tiles");
    assert!(tile_count > 0, "the fixture yields no tile, so the comparison would prove nothing");
    assert!(py.iter().any(|(n, b)| n.ends_with(".osm.pbf") && b.len() > 400));
    assert_eq!(py.keys().collect::<Vec<_>>(), rs.keys().collect::<Vec<_>>(), "the two folders hold different files");
    for (name, bytes) in &py {
        assert!(bytes == &rs[name], "{name} differs");
    }
}
