//! The command line: the commands, their arguments, and `pack`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use cityloom_model::NetworkTile;
use clap::{Parser, Subcommand};

use crate::archive::{self, PackInputs};
use crate::metro_index::Grid;
use crate::prepare::{self, PrepareArgs};
use crate::progress::{Progress, Spec, StepError, exit_on_ctrl_c, megabytes};
use crate::tiles::{self, TilesJob};

#[derive(Parser, Debug)]
#[command(name = "citymodel_builder", about = "Prepares CityLoom's area data: tiles and the .citymodel archive")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Download the extract if needed, make the network tiles and pack the archive.
    Prepare {
        extract: Option<PathBuf>,
        /// Download a newer extract.
        #[arg(long)]
        refresh: bool,
    },
    /// Keep an extract's roads, import them and cut the network into tiles.
    Tiles {
        extract: PathBuf,
        workdir: Option<PathBuf>,
        /// The tiles' folder (web/data/metro by default).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Pack the network tiles into a .citymodel archive.
    Pack {
        #[arg(long, default_value = "montreal")]
        area: String,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        metro: Option<PathBuf>,
        #[arg(long, default_value = "quebec-latest.osm.pbf")]
        source: String,
        /// The extract's replication time, Unix seconds.
        #[arg(long, default_value_t = 0)]
        source_time: i64,
        /// When the archive is made, Unix seconds (now by default).
        #[arg(long)]
        created: Option<i64>,
    },
    /// Import a whole OSM file (XML or PBF) and write it as one network tile, in the data's own frame: a fixture.
    Fixture { input: PathBuf, out: PathBuf },
}

/// `2026-10-16T20:21:15Z` as Unix seconds (osmium's replication timestamp), or None.
pub fn parse_timestamp(text: &str) -> Option<i64> {
    let b = text.trim().as_bytes();
    if b.len() != 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' || b[19] != b'Z' {
        return None;
    }
    let num = |from: usize, to: usize| std::str::from_utf8(&b[from..to]).ok()?.parse::<i64>().ok();
    let (y, m, d, hh, mm, ss) = (num(0, 4)?, num(5, 7)?, num(8, 10)?, num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    // days from civil (H. Hinnant)
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hh * 3600 + mm * 60 + ss)
}

/// The extract's replication time, asked of osmium; 0 when it cannot be had.
pub fn source_time(extract: &Path, osmium: &str) -> i64 {
    let output = Process::new(osmium).args(["fileinfo", "-g", "header.option.osmosis_replication_timestamp"]).arg(extract).output();
    match output {
        Ok(out) if out.status.success() => parse_timestamp(&String::from_utf8_lossy(&out.stdout)).unwrap_or(0),
        _ => 0,
    }
}

pub fn pack_steps(out: &Path) -> Vec<Spec> {
    let name = out.file_name().and_then(|n| n.to_str()).unwrap_or("archive");
    vec![Spec { key: "pack", group: "archive", subject: name.to_string() }]
}

pub struct PackJob {
    pub out: PathBuf,
    pub area: String,
    pub metro: PathBuf,
    pub source: String,
    pub source_time: i64,
    pub created: i64,
    /// The grid's size in tiles, when the caller knows it; else the built-in grid's or the tiles' extent.
    pub grid: Option<Grid>,
}

fn fail(line: impl Into<String>) -> StepError {
    StepError::new([line.into()])
}

fn tile_name(name: &str) -> Option<(u16, u16)> {
    let (x, y) = name.split_once('_')?;
    Some((x.parse().ok()?, y.parse().ok()?))
}

fn number(index: &serde_json::Value, key: &str) -> Result<f64, StepError> {
    index[key].as_f64().ok_or_else(|| fail(format!("index.json has no number {key:?}")))
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// The pieces of `run_pack` that read the folders: the grid, the network tiles.
fn read_inputs(job: &PackJob) -> Result<(Grid, Vec<(u16, u16, PathBuf)>), StepError> {
    let index_path = job.metro.join("index.json");
    let text = fs::read_to_string(&index_path).map_err(|e| fail(format!("cannot read {}: {e}", index_path.display())))?;
    let index: serde_json::Value = serde_json::from_str(&text).map_err(|e| fail(format!("{} is not JSON: {e}", index_path.display())))?;
    let listed: Vec<String> = index["tiles"]
        .as_array()
        .ok_or_else(|| fail("index.json has no list of tiles"))?
        .iter()
        .map(|t| t.as_str().map(str::to_string).ok_or_else(|| fail("index.json lists a tile that is not a name")))
        .collect::<Result<_, _>>()?;
    let mut on_disk = Vec::new();
    for entry in fs::read_dir(&job.metro).map_err(|e| fail(format!("cannot read {}: {e}", job.metro.display())))? {
        let name = entry.map_err(|e| fail(e.to_string()))?.file_name().to_string_lossy().into_owned();
        if let Some(tile) = name.strip_suffix(".json").filter(|t| *t != "index") {
            on_disk.push(tile.to_string());
        }
    }
    if let Some(tile) = listed.iter().find(|t| !on_disk.contains(t)) {
        return Err(fail(format!("tile {tile} is listed in index.json and {tile}.json is not in {}", job.metro.display())));
    }
    if let Some(tile) = on_disk.iter().find(|t| !listed.contains(t)) {
        return Err(fail(format!("tile {tile} is in {} and not listed in index.json", job.metro.display())));
    }
    let mut roads = Vec::new();
    for name in &listed {
        let (x, y) = tile_name(name).ok_or_else(|| fail(format!("{name:?} is not a tile name (x_y)")))?;
        roads.push((x, y, job.metro.join(format!("{name}.json"))));
    }
    roads.sort();
    if roads.is_empty() {
        return Err(fail(format!("no network tiles in {}", job.metro.display())));
    }

    let (lon0, lat0, dlon, dlat) = (number(&index, "lon0")?, number(&index, "lat0")?, number(&index, "dlon")?, number(&index, "dlat")?);
    let montreal = tiles::grid();
    let grid = match job.grid {
        Some(grid) => grid,
        None if same(lon0, montreal.lon0) && same(lat0, montreal.lat0) && same(dlon, montreal.dlon) && same(dlat, montreal.dlat) => montreal,
        None => {
            let nx = roads.iter().map(|r| r.0).max().map_or(1, |m| m + 1);
            let ny = roads.iter().map(|r| r.1).max().map_or(1, |m| m + 1);
            Grid { lon0, lat0, dlon, dlat, nx, ny }
        }
    };

    Ok((grid, roads))
}

pub fn run_pack(p: &Progress, job: &PackJob) -> Result<(), StepError> {
    p.step("pack", |ctx| {
        let (grid, roads) = read_inputs(job)?;
        let inputs = PackInputs { area: &job.area, source: &job.source, source_time: job.source_time, created: job.created, grid, networks: &roads };
        let mut shown = u64::MAX;
        let mut report = |done: u64, total: u64| {
            let mb = done / 1_000_000;
            if mb != shown {
                shown = mb;
                let fraction = if total == 0 { 1.0 } else { done as f64 / total as f64 };
                ctx.update(None, Some(&megabytes(done, Some(total))), Some(fraction));
            }
        };
        let members = archive::pack(&inputs, &job.out, &mut report).map_err(|e| fail(e.to_string()))?;
        let size = fs::metadata(&job.out).map(|m| m.len()).unwrap_or(0);
        // The index is a member too.
        ctx.update(None, Some(&format!("{} members, {:.1} MB", members + 1, size as f64 / 1_000_000.0)), Some(1.0));
        Ok(())
    })
}

/// `citymodel_builder fixture`: the whole of `input`, imported, as one `NetworkTile` in compact JSON at `out`.
pub fn fixture(input: &Path, out: &Path) -> Result<(), String> {
    let osm = fs::read(input).map_err(|e| format!("cannot read {}: {e}", input.display()))?;
    let (network, frame) = crate::importer::import_whole(&osm).map_err(|e| format!("cannot import {}: {e}", input.display()))?;
    let json = serde_json::to_vec(&NetworkTile { frame, network }).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
    fs::write(out, json).map_err(|e| format!("cannot write {}: {e}", out.display()))
}

/// The folder `tiles` writes to: `out`, else the app's `web/data/metro`.
pub fn tiles_out(out: Option<PathBuf>) -> PathBuf {
    absolute(&out.unwrap_or_else(|| repo_root().join("web/data/metro")))
}

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).expect("the crate lies in the repository's crates folder").to_path_buf()
}

/// An absolute path: a path handed to a tool must not start with `-`.
pub(crate) fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// Runs a command; the error is the exit code.
pub fn dispatch(cli: Cli) -> Result<(), i32> {
    match cli.command {
        Command::Pack { area, out, metro, source, source_time, created } => {
            let root = repo_root();
            let job = PackJob {
                out: absolute(&out.unwrap_or_else(|| root.join("data/montreal.citymodel"))),
                area,
                metro: absolute(&metro.unwrap_or_else(|| root.join("web/data/metro"))),
                source,
                source_time,
                created: created.unwrap_or_else(now),
                grid: None,
            };
            let progress = Arc::new(Progress::stderr(pack_steps(&job.out), root.join("data/logs"), root));
            exit_on_ctrl_c(&progress).expect("the Ctrl-C handler");
            let result = run_pack(&progress, &job);
            progress.close();
            result.map_err(|_| 1)
        }
        Command::Tiles { extract, workdir, out } => {
            let root = repo_root();
            let (source, work, out) = (absolute(&extract), absolute(&workdir.unwrap_or_else(|| root.join("data/metro-work"))), tiles_out(out));
            let job = TilesJob { source: &source, work: &work, out: &out, osmium: "osmium" };
            let progress = Arc::new(Progress::stderr(tiles::steps(), root.join("data/logs"), root));
            exit_on_ctrl_c(&progress).expect("the Ctrl-C handler");
            let result = tiles::run(&progress, &job);
            progress.close();
            result.map_err(|_| 1)
        }
        Command::Prepare { extract, refresh } => prepare::run(PrepareArgs { extract, refresh }),
        Command::Fixture { input, out } => fixture(&input, &out).map_err(|error| {
            eprintln!("{error}");
            1
        }),
    }
}
