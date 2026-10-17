//! The command line: the commands, their arguments, and `pack`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};

use crate::archive::{self, PackInputs};
use crate::metro_index::Grid;
use crate::progress::{Progress, Spec, StepError, exit_on_ctrl_c, thousands};

/// The metropolitan Montréal grid the app's tiles are cut on (Task 10 moves it to `tiles::grid()`).
const MONTREAL: Grid = Grid { lon0: -74.4, lat0: 45.2, dlon: 0.0257, dlat: 0.018, nx: 51, ny: 34 };

#[derive(Parser, Debug)]
#[command(name = "cityloom_prepare", about = "Prepares CityLoom's area data: tiles, basemap and the .cityloom archive")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Download the extract if needed, cut the road tiles, build the basemap and pack the archive.
    Prepare {
        extract: Option<PathBuf>,
        /// Download a newer extract.
        #[arg(long)]
        refresh: bool,
        /// Tiles cut per osmium run (1 to 500).
        #[arg(long, value_parser = batch_size)]
        batch: Option<usize>,
    },
    /// Cut an extract into road tiles.
    Tiles {
        extract: PathBuf,
        workdir: Option<PathBuf>,
        #[arg(long, value_parser = batch_size)]
        batch: Option<usize>,
    },
    /// Build the basemap's PMTiles from an extract.
    Basemap {
        extract: PathBuf,
        output: Option<PathBuf>,
        /// West,South,East,North in degrees.
        #[arg(long, value_parser = bounds, allow_hyphen_values = true)]
        bounds: Option<String>,
        #[arg(long)]
        minzoom: Option<u8>,
        #[arg(long)]
        maxzoom: Option<u8>,
        /// Planetiler's heap, such as 4g.
        #[arg(long, value_parser = memory)]
        memory: Option<String>,
    },
    /// Pack the tiles and the basemap into a .cityloom archive.
    Pack {
        #[arg(long, default_value = "montreal")]
        area: String,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        metro: Option<PathBuf>,
        #[arg(long)]
        basemap: Option<PathBuf>,
        #[arg(long, default_value = "quebec-latest.osm.pbf")]
        source: String,
        /// The extract's replication time, Unix seconds.
        #[arg(long, default_value_t = 0)]
        source_time: i64,
        /// When the archive is made, Unix seconds (now by default).
        #[arg(long)]
        created: Option<i64>,
    },
}

fn batch_size(text: &str) -> Result<usize, String> {
    match text.parse::<usize>() {
        Ok(n) if (1..=500).contains(&n) => Ok(n),
        _ => Err(format!("{text:?} is not a number from 1 to 500")),
    }
}

fn is_decimal(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = match digits.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (digits, None),
    };
    let all = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    all(whole) && fraction.is_none_or(all)
}

fn bounds(text: &str) -> Result<String, String> {
    let parts: Vec<&str> = text.split(',').collect();
    if parts.len() == 4 && parts.iter().all(|p| is_decimal(p)) {
        Ok(text.to_string())
    } else {
        Err(format!("{text:?} is not W,S,E,N (four numbers separated by commas)"))
    }
}

fn memory(text: &str) -> Result<String, String> {
    match text.char_indices().last() {
        Some((at, unit)) if at > 0 && "kmgKMG".contains(unit) && text[..at].bytes().all(|b| b.is_ascii_digit()) => Ok(text.to_string()),
        _ => Err(format!("{text:?} is not a size such as 4g")),
    }
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
    pub basemap: PathBuf,
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

/// The pieces of `run_pack` that read the folders: the grid, the road tiles and the basemaps.
fn read_inputs(job: &PackJob) -> Result<(Grid, Vec<(u16, u16, PathBuf)>, Vec<PathBuf>), StepError> {
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
        if let Some(tile) = name.strip_suffix(".osm.pbf") {
            on_disk.push(tile.to_string());
        }
    }
    if let Some(tile) = listed.iter().find(|t| !on_disk.contains(t)) {
        return Err(fail(format!("tile {tile} is listed in index.json and {tile}.osm.pbf is not in {}", job.metro.display())));
    }
    if let Some(tile) = on_disk.iter().find(|t| !listed.contains(t)) {
        return Err(fail(format!("tile {tile} is in {} and not listed in index.json", job.metro.display())));
    }
    let mut roads = Vec::new();
    for name in &listed {
        let (x, y) = tile_name(name).ok_or_else(|| fail(format!("{name:?} is not a tile name (x_y)")))?;
        roads.push((x, y, job.metro.join(format!("{name}.osm.pbf"))));
    }
    roads.sort();
    if roads.is_empty() {
        return Err(fail(format!("no road tiles in {}", job.metro.display())));
    }

    let (lon0, lat0, dlon, dlat) = (number(&index, "lon0")?, number(&index, "lat0")?, number(&index, "dlon")?, number(&index, "dlat")?);
    let grid = match job.grid {
        Some(grid) => grid,
        None if same(lon0, MONTREAL.lon0) && same(lat0, MONTREAL.lat0) && same(dlon, MONTREAL.dlon) && same(dlat, MONTREAL.dlat) => MONTREAL,
        None => {
            let nx = roads.iter().map(|r| r.0).max().map_or(1, |m| m + 1);
            let ny = roads.iter().map(|r| r.1).max().map_or(1, |m| m + 1);
            Grid { lon0, lat0, dlon, dlat, nx, ny }
        }
    };

    let mut basemaps = Vec::new();
    if let Ok(dir) = fs::read_dir(&job.basemap) {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "pmtiles") && path.is_file() {
                basemaps.push(path);
            }
        }
    }
    basemaps.sort();
    if basemaps.is_empty() {
        return Err(fail(format!("no basemap: there is no .pmtiles file in {} (run the basemap step first)", job.basemap.display())));
    }
    Ok((grid, roads, basemaps))
}

pub fn run_pack(p: &Progress, job: &PackJob) -> Result<(), StepError> {
    p.step("pack", |ctx| {
        let (grid, roads, basemaps) = read_inputs(job)?;
        let inputs =
            PackInputs { area: &job.area, source: &job.source, source_time: job.source_time, created: job.created, grid, roads: &roads, basemaps: &basemaps };
        let mut shown = u64::MAX;
        let mut report = |done: u64, total: u64| {
            let mb = done / 1_000_000;
            if mb != shown {
                shown = mb;
                let fraction = if total == 0 { 1.0 } else { done as f64 / total as f64 };
                ctx.update(None, Some(&format!("{} of {} MB", thousands(mb), thousands(total / 1_000_000))), Some(fraction));
            }
        };
        let members = archive::pack(&inputs, &job.out, &mut report).map_err(|e| fail(e.to_string()))?;
        let size = fs::metadata(&job.out).map(|m| m.len()).unwrap_or(0);
        // The index is a member too.
        ctx.update(None, Some(&format!("{} members, {:.1} MB", members + 1, size as f64 / 1_000_000.0)), Some(1.0));
        Ok(())
    })
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the crate lies in the repository").to_path_buf()
}

/// An absolute path: a path handed to a tool must not start with `-`.
fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// Runs a command; the error is the exit code.
pub fn dispatch(cli: Cli) -> Result<(), i32> {
    match cli.command {
        Command::Pack { area, out, metro, basemap, source, source_time, created } => {
            let root = repo_root();
            let job = PackJob {
                out: absolute(&out.unwrap_or_else(|| root.join("data/montreal.cityloom"))),
                area,
                metro: absolute(&metro.unwrap_or_else(|| root.join("web/data/metro"))),
                basemap: absolute(&basemap.unwrap_or_else(|| root.join("web/data/basemap"))),
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
        Command::Prepare { .. } | Command::Tiles { .. } | Command::Basemap { .. } => {
            eprintln!("not implemented yet");
            Err(2)
        }
    }
}
