//! Cuts the roads of the Montréal metropolitan area out of a Quebec OSM extract into tiles: a port of
//! the former Python `metro_tiles`.
//!
//! A tile is a rectangle (its core) holding every road that reaches into the core widened by a margin, whole, with
//! the nodes it needs. The margin is wider than half an area (400 m), so a place whose centre is in a tile's core is
//! read from that one tile. Writes `<out>/X_Y.osm.pbf` and `<out>/index.json`, which the app reads.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::metro_index::{Grid, index_json, short_checksum};
use crate::progress::{Progress, Spec, StepError, thousands};

// The grid. The south-west corner of tile 0_0, and the size of a core: about 2 km square at this latitude.
pub const LON0: f64 = -74.40;
pub const LAT0: f64 = 45.20;
pub const DLON: f64 = 0.0257;
pub const DLAT: f64 = 0.018;
/// To -73.09 east and 45.81 north: the metropolitan area and some country round it.
pub const NX: u16 = 51;
pub const NY: u16 = 34;
/// 0.5 km: wider than the 0.4 km an area reaches from its centre.
pub const MARGIN_LON: f64 = 0.0064;
pub const MARGIN_LAT: f64 = 0.0045;

pub const HIGHWAYS: &str = "motorway,motorway_link,trunk,trunk_link,primary,primary_link,secondary,secondary_link,\
tertiary,tertiary_link,unclassified,residential,living_street,service,pedestrian";
/// The bike tracks mapped as ways of their own, which the importer reads to tell a two-way track from a one-way one
/// and leaves out of the network. A cycleway is taken whole; a path or footway only when designated for bicycles,
/// which osmium cannot say in one expression (its expressions are ORed).
pub const TRACKS: &str = "cycleway";
pub const OFF_ROAD: &str = "path,footway";
/// Extracts per osmium run, which allows 500 at most. osmium keeps a set of ids for each extract it cuts at once, a
/// gigabyte or more apiece for the dense tiles of the city, so a machine with less than about 32 GB needs a smaller batch.
pub const MAX_EXTRACTS: usize = 500;
/// An empty tile is a file with only a header.
pub const EMPTY_BYTES: u64 = 400;

const SUFFIX: &str = ".osm.pbf";

pub fn grid() -> Grid {
    Grid { lon0: LON0, lat0: LAT0, dlon: DLON, dlat: DLAT, nx: NX, ny: NY }
}

pub fn steps() -> Vec<Spec> {
    let spec = |key, group, subject: &str| Spec { key, group, subject: subject.to_string() };
    vec![
        spec("area", "roads", "metro area"),
        spec("highways", "roads", "highways"),
        spec("tracks", "roads", "designated tracks"),
        spec("merge", "roads", "merge"),
        spec("tiles", "tiles", &format!("cutting {} tiles", thousands(NX as u32 * NY as u32))),
        spec("index", "tiles", "index"),
    ]
}

pub struct Extract {
    pub output: String,
    /// West, south, east, north.
    pub bbox: [f64; 4],
}

/// Every tile, `x` outer and `y` inner, each box computed with the operations the script used, so the same doubles.
pub fn extracts() -> Vec<Extract> {
    let mut all = Vec::with_capacity(NX as usize * NY as usize);
    for x in 0..NX {
        for y in 0..NY {
            let output = format!("{x}_{y}{SUFFIX}");
            let (x, y) = (f64::from(x), f64::from(y));
            all.push(Extract {
                output,
                bbox: [LON0 + x * DLON - MARGIN_LON, LAT0 + y * DLAT - MARGIN_LAT, LON0 + (x + 1.0) * DLON + MARGIN_LON, LAT0 + (y + 1.0) * DLAT + MARGIN_LAT],
            });
        }
    }
    all
}

/// The box of the whole metro area, `W,S,E,N`, as osmium's `-b` reads it.
pub fn metro_bounds() -> String {
    let (west, south) = (LON0 - MARGIN_LON, LAT0 - MARGIN_LAT);
    let (east, north) = (LON0 + f64::from(NX) * DLON + MARGIN_LON, LAT0 + f64::from(NY) * DLAT + MARGIN_LAT);
    format!("{west},{south},{east},{north}")
}

pub struct TilesJob<'a> {
    pub source: &'a Path,
    pub work: &'a Path,
    pub out: &'a Path,
    pub batch: usize,
    pub osmium: &'a str,
}

/// The osmium command with arguments that are strings and paths alike.
fn command(job: &TilesJob, args: &[&dyn AsRef<std::ffi::OsStr>]) -> Command {
    let mut command = Command::new(job.osmium);
    command.args(args.iter().map(|a| a.as_ref()));
    command
}

/// The tiles in `dir`, by file name.
fn tile_files(dir: &Path) -> Result<Vec<PathBuf>, StepError> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.file_name().is_some_and(|n| n.to_string_lossy().ends_with(SUFFIX)) {
                files.push(path);
            }
        }
    }
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(files)
}

/// osmium has all of a run's extracts open at once.
fn raise_open_files(wanted: usize) {
    if let Ok((soft, hard)) = rlimit::Resource::NOFILE.get() {
        let _ = rlimit::Resource::NOFILE.set(soft.max(wanted as u64).min(hard), hard);
    }
}

fn config_json(out: &Path, chunk: &[Extract]) -> String {
    let extracts: Vec<_> = chunk.iter().map(|e| serde_json::json!({"output": e.output, "output_format": "pbf", "bbox": e.bbox})).collect();
    serde_json::json!({"directory": out.to_string_lossy(), "extracts": extracts}).to_string()
}

fn checksum(path: &Path) -> Result<String, StepError> {
    let digest: [u8; 32] = Sha256::digest(fs::read(path)?).into();
    Ok(short_checksum(&digest))
}

/// Cuts the tiles from `job.source`, step by step as `steps` says, with osmium's output in the steps' logs.
pub fn run(p: &Progress, job: &TilesJob) -> Result<(), StepError> {
    let (metro, roads) = (job.work.join("metro.osm.pbf"), job.work.join("roads.osm.pbf"));
    let (designated, tracks) = (job.work.join("designated.osm.pbf"), job.work.join("designated-tracks.osm.pbf"));
    let streets = job.work.join("streets.osm.pbf");
    let overwrite = "--overwrite";

    p.step("area", |s| {
        if !job.source.is_file() {
            return Err(StepError::new([
                format!("{} does not exist. Download the Quebec extract first (about 1.2 GB):", job.source.display()),
                "curl -L -o quebec-latest.osm.pbf https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf".to_string(),
            ]));
        }
        fs::create_dir_all(job.work)?;
        s.run(command(job, &[&"extract", &"-b", &metro_bounds(), &job.source, &"-o", &metro, &overwrite]))
    })?;
    p.step("highways", |s| s.run(command(job, &[&"tags-filter", &metro, &format!("w/highway={HIGHWAYS},{TRACKS}"), &"-o", &streets, &overwrite])))?;
    p.step("tracks", |s| {
        // a path or footway designated for bicycles: every way tagged so, then those of them that are paths or footways
        // (the sidewalks and crossings, the many footways of a city, are not kept)
        s.run(command(job, &[&"tags-filter", &metro, &"w/bicycle=designated", &"-o", &designated, &overwrite]))?;
        s.run(command(job, &[&"tags-filter", &designated, &format!("w/highway={OFF_ROAD}"), &"-o", &tracks, &overwrite]))
    })?;
    p.step("merge", |s| {
        // a node or way in both files is written once
        s.run(command(job, &[&"merge", &streets, &tracks, &"-o", &roads, &overwrite]))
    })?;

    let kept = p.step("tiles", |s| {
        for old in tile_files(job.out)? {
            fs::remove_file(old)?;
        }
        fs::create_dir_all(job.out)?;
        raise_open_files(job.batch + 64);
        let all = extracts();
        let batches: Vec<&[Extract]> = all.chunks(job.batch).collect();
        let config = job.work.join("tiles.json");
        for (n, chunk) in batches.iter().enumerate() {
            s.update(None, Some(&format!("batch {} of {}", n + 1, batches.len())), Some(n as f64 / batches.len() as f64));
            fs::write(&config, config_json(job.out, chunk))?;
            s.run(command(job, &[&"extract", &"--config", &config, &roads, &overwrite]))?;
        }
        let mut kept = Vec::new();
        let mut total = 0;
        for path in tile_files(job.out)? {
            let size = fs::metadata(&path)?.len();
            if size > EMPTY_BYTES {
                total += size;
                kept.push(path);
            } else {
                fs::remove_file(&path)?;
            }
        }
        s.update(Some(&format!("{} tiles", thousands(kept.len()))), Some(&format!("{:.1} MB", total as f64 / 1e6)), None);
        Ok(kept)
    })?;
    p.step("index", |_| {
        let mut named = Vec::new();
        for path in &kept {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            named.push((name.strip_suffix(SUFFIX).unwrap_or(&name).to_string(), checksum(path)?));
        }
        fs::write(job.out.join("index.json"), index_json(&grid(), &named))?;
        Ok(())
    })
}
