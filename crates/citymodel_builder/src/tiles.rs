//! Makes a city's network tiles from its OSM extract: osmium keeps the roads (as the former Python `metro_tiles`
//! did), the importer reads them once into the whole area's network, and that is cut along the city's grid
//! (`network_tiles`).
//!
//! A tile is a rectangle (its core) holding every road that reaches into the core widened by a margin, whole, with
//! the junctions it uses. The margin is wider than half an area (400 m), so a place whose centre is in a tile's core
//! is read from that one tile. Writes `<out>/X_Y.json` and `<out>/index.json`, which the app reads.
//!
//! The grid and the margin come from the city's config (`city`), as `CityGrid` and `Margin`; everything that turns
//! their numbers into tiles is here and in `network_tiles`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cityloom_model::Frame;
use serde::Deserialize;

use crate::metro_index::{Grid, index_json};
use crate::network_tiles;
use crate::progress::{Progress, Spec, StepError, thousands};

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

/// How far an area reaches from its centre (the editor's `place::area::HALF_M`), which a margin must be wide enough for.
pub const AREA_REACH_M: f64 = 400.0;

/// A city's grid, as its config file gives it: the south-west corner of tile 0_0, the size of a tile's core (degrees)
/// and the number of tiles.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CityGrid {
    pub west: f64,
    pub south: f64,
    pub tile_width: f64,
    pub tile_height: f64,
    pub columns: u16,
    pub rows: u16,
}

impl CityGrid {
    /// The grid of `index.json` and the archive: the same numbers.
    pub fn grid(&self) -> Grid {
        Grid { lon0: self.west, lat0: self.south, dlon: self.tile_width, dlat: self.tile_height, nx: self.columns, ny: self.rows }
    }
}

/// The margin round a tile's core, in degrees.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Margin {
    pub lon: f64,
    pub lat: f64,
}

/// What is wrong with a city's config: the field, as `grid.west`, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldError {
    pub field: &'static str,
    pub reason: String,
}

fn refuse(field: &'static str, reason: impl Into<String>) -> Result<(), FieldError> {
    Err(FieldError { field, reason: reason.into() })
}

/// Whether tiles can be made of this grid and margin: finite numbers, at least one tile of some size, a grid on the
/// earth, and a margin at least as wide as an area reaches from its centre (`AREA_REACH_M`), measured where the grid
/// is nearest a pole, where a degree of longitude is the narrowest.
pub fn check(grid: &CityGrid, margin: &Margin) -> Result<(), FieldError> {
    let numbers = [
        ("grid.west", grid.west),
        ("grid.south", grid.south),
        ("grid.tile_width", grid.tile_width),
        ("grid.tile_height", grid.tile_height),
        ("margin.lon", margin.lon),
        ("margin.lat", margin.lat),
    ];
    if let Some((field, _)) = numbers.iter().find(|(_, value)| !value.is_finite()) {
        return refuse(field, "is not a finite number");
    }
    if grid.columns < 1 {
        return refuse("grid.columns", "must be at least 1");
    }
    if grid.rows < 1 {
        return refuse("grid.rows", "must be at least 1");
    }
    if grid.tile_width <= 0.0 {
        return refuse("grid.tile_width", "must be over 0");
    }
    if grid.tile_height <= 0.0 {
        return refuse("grid.tile_height", "must be over 0");
    }
    if !(-180.0..=180.0).contains(&grid.west) {
        return refuse("grid.west", "must lie from -180 to 180 degrees");
    }
    if !(-90.0..=90.0).contains(&grid.south) {
        return refuse("grid.south", "must lie from -90 to 90 degrees");
    }
    let east = grid.west + f64::from(grid.columns) * grid.tile_width;
    if east > 180.0 {
        return refuse("grid.columns", format!("takes the grid's east edge to {east} degrees, past 180"));
    }
    let north = grid.south + f64::from(grid.rows) * grid.tile_height;
    if north > 90.0 {
        return refuse("grid.rows", format!("takes the grid's north edge to {north} degrees, past 90"));
    }
    if margin.lat <= 0.0 {
        return refuse("margin.lat", "must be over 0");
    }
    if margin.lon <= 0.0 {
        return refuse("margin.lon", "must be over 0");
    }
    let tall = Frame { south: grid.south, west: grid.west, north: grid.south + margin.lat, east: grid.west }.height_m();
    if tall < AREA_REACH_M {
        return refuse("margin.lat", format!("is {tall:.0} m, narrower than the {AREA_REACH_M} m an area reaches from its centre"));
    }
    let poleward = if north.abs() > grid.south.abs() { north } else { grid.south };
    let wide = Frame { south: poleward, west: grid.west, north: poleward, east: grid.west + margin.lon }.width_m();
    if wide < AREA_REACH_M {
        return refuse("margin.lon", format!("is {wide:.0} m at {poleward} degrees, narrower than the {AREA_REACH_M} m an area reaches from its centre"));
    }
    Ok(())
}

pub fn steps(grid: &Grid) -> Vec<Spec> {
    let spec = |key, group, subject: &str| Spec { key, group, subject: subject.to_string() };
    vec![
        spec("area", "roads", "metro area"),
        spec("highways", "roads", "highways"),
        spec("tracks", "roads", "designated tracks"),
        spec("merge", "roads", "merge"),
        spec("import", "network", "whole area"),
        spec("tiles", "tiles", &format!("cutting {} tiles", thousands(grid.nx as u32 * grid.ny as u32))),
        spec("index", "tiles", "index"),
    ]
}

/// The box of the whole metro area, `W,S,E,N`, as osmium's `-b` reads it.
pub fn metro_bounds(grid: &Grid, margin: &Margin) -> String {
    let (west, south) = (grid.lon0 - margin.lon, grid.lat0 - margin.lat);
    let (east, north) = (grid.lon0 + f64::from(grid.nx) * grid.dlon + margin.lon, grid.lat0 + f64::from(grid.ny) * grid.dlat + margin.lat);
    format!("{west},{south},{east},{north}")
}

pub struct TilesJob<'a> {
    pub source: &'a Path,
    pub work: &'a Path,
    pub out: &'a Path,
    pub osmium: &'a str,
    pub grid: Grid,
    pub margin: Margin,
    /// Where the extract is downloaded from, and its file name: for the error when it is missing.
    pub url: &'a str,
    pub extract_name: &'a str,
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
pub fn index(grid: &Grid, mut named: Vec<(String, String)>) -> String {
    named.sort();
    index_json(grid, &named)
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
                format!("{} does not exist. Download the extract first:", job.source.display()),
                format!("curl -L -o {} {}", job.extract_name, job.url),
            ]));
        }
        fs::create_dir_all(job.work)?;
        s.run(command(job, &[&"extract", &"-b", &metro_bounds(&job.grid, &job.margin), &job.source, &"-o", &metro, &overwrite]))
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
        let tiles = network_tiles::cut(&network, &frame, &job.grid, &job.margin);
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
        fs::write(job.out.join("index.json"), index(&job.grid, named))?;
        Ok(())
    })
}
