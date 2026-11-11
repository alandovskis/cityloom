//! Makes the network tiles of the Montréal metropolitan area from a Quebec OSM extract: osmium keeps the roads (as
//! the former Python `metro_tiles` did), the importer reads them once into the whole area's network, and that is cut
//! along the grid (`network_tiles`).
//!
//! A tile is a rectangle (its core) holding every road that reaches into the core widened by a margin, whole, with
//! the junctions it uses. The margin is wider than half an area (400 m), so a place whose centre is in a tile's core
//! is read from that one tile. Writes `<out>/X_Y.json` and `<out>/index.json`, which the app reads.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::metro_index::{Grid, index_json};
use crate::network_tiles;
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

const SUFFIX: &str = ".json";
/// The tiles osmium cut before the importer made them; removed with the old tiles.
const OLD_SUFFIX: &str = ".osm.pbf";

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
        spec("import", "network", "whole area"),
        spec("tiles", "tiles", &format!("cutting {} tiles", thousands(NX as u32 * NY as u32))),
        spec("index", "tiles", "index"),
    ]
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
    pub osmium: &'a str,
}

/// The osmium command with arguments that are strings and paths alike.
fn command(job: &TilesJob, args: &[&dyn AsRef<std::ffi::OsStr>]) -> Command {
    let mut command = Command::new(job.osmium);
    command.args(args.iter().map(|a| a.as_ref()));
    command
}

/// The tiles in `dir`: network tiles and the PBF tiles of before (not `index.json`, which is written again after).
fn tile_files(dir: &Path) -> Result<Vec<PathBuf>, StepError> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let tile = name.strip_suffix(SUFFIX).or_else(|| name.strip_suffix(OLD_SUFFIX));
            if tile.is_some_and(|t| t != "index") {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// The `index.json` of the tiles `named` (name, short checksum), listed by name as text: the order of the archive's
/// members, from which the server writes its `index.json` when it unpacks, so the two are the same bytes.
pub fn index(mut named: Vec<(String, String)>) -> String {
    named.sort();
    index_json(&grid(), &named)
}

/// Makes the tiles from `job.source`, step by step as `steps` says, with osmium's output in the steps' logs.
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

    let (network, frame) = p.step("import", |s| {
        // the importer takes the file's bytes; they go once it has read them
        let osm = fs::read(&roads).map_err(|e| StepError::new([format!("cannot read {}: {e}", roads.display())]))?;
        let (network, frame) = crate::importer::import_whole(&osm).map_err(|e| StepError::new([format!("cannot import {}: {e}", roads.display())]))?;
        let detail = format!("{} roads, {} junctions", thousands(network.roads.len()), thousands(network.junctions.len()));
        s.update(None, Some(&detail), None);
        Ok((network, frame))
    })?;
    let named = p.step("tiles", |s| {
        for old in tile_files(job.out)? {
            fs::remove_file(old)?;
        }
        fs::create_dir_all(job.out)?;
        let tiles = network_tiles::cut(&network, &frame, &grid());
        // the tiles hold all of the network they need, so it goes before they are written
        drop(network);
        let named = network_tiles::write(job.out, &tiles)?;
        let mut total = 0;
        for (name, _) in &named {
            total += fs::metadata(job.out.join(format!("{name}{SUFFIX}")))?.len();
        }
        s.update(Some(&format!("{} tiles", thousands(named.len()))), Some(&format!("{:.1} MB", total as f64 / 1e6)), None);
        Ok(named)
    })?;
    p.step("index", |_| {
        fs::write(job.out.join("index.json"), index(named))?;
        Ok(())
    })
}
