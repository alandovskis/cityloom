//! The basemap of the Montréal metropolitan area, built from a Quebec OSM extract by Planetiler (OpenMapTiles
//! schema): a port of `scripts/basemap_tiles.py`, which stays in the tree until the Python is removed.
//!
//! Needs Java 21 or later. Planetiler's latest release is downloaded to `<tools>/planetiler.jar` on first use (delete
//! it to take a newer one), and the first run fetches about 1 GB of Natural Earth and water-polygon sources once,
//! into `data/sources`.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::download::download;
use crate::progress::{Progress, Spec, StepError, megabytes};

pub const JAR_URL: &str = "https://github.com/onthegomap/planetiler/releases/latest/download/planetiler.jar";
/// The grid `tiles` cuts: west, south, east, north.
pub const METRO_BOUNDS: &str = "-74.40,45.20,-73.09,45.81";
pub const MINZOOM: u32 = 0;
pub const MAXZOOM: u32 = 14;
pub const MEMORY: &str = "4g";
const EXTRACT_URL: &str = "https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf";

/// What the run does, in order, for the progress display.
pub fn steps(jar_name: &str, output_name: &str) -> Vec<Spec> {
    vec![
        Spec { key: "planetiler", group: "basemap", subject: jar_name.to_string() },
        Spec { key: "basemap", group: "basemap", subject: output_name.to_string() },
    ]
}

pub struct BasemapJob<'a> {
    pub extract: &'a Path,
    pub output: &'a Path,
    pub bounds: &'a str,
    pub minzoom: u32,
    pub maxzoom: u32,
    pub memory: &'a str,
    /// Where `planetiler.jar` is kept, with its `.source` and its temporary files.
    pub tools: &'a Path,
    /// The working directory of Java.
    pub root: &'a Path,
    pub java: &'a str,
    pub jar_url: &'a str,
}

/// Builds the basemap from `job.extract`, step by step as `steps` says, with Java's output in the steps' logs.
pub fn run(p: &Progress, job: &BasemapJob) -> Result<(), StepError> {
    let jar = job.tools.join("planetiler.jar");
    p.step("planetiler", |step| {
        if !job.extract.exists() {
            return Err(StepError::new([format!("{} is not there: get it from {EXTRACT_URL}", job.extract.display())]));
        }
        if jar.exists() {
            step.update(None, Some("kept"), None);
            return Ok(());
        }
        let mut say = |done: u64, total: Option<u64>| {
            step.update(None, Some(&megabytes(done, total)), total.map(|t| done as f64 / t as f64));
        };
        let release = download(job.jar_url, &jar, false, &mut say)?;
        let mut source = jar.clone().into_os_string();
        source.push(".source");
        fs::write(&source, format!("{}\n", release.as_deref().unwrap_or(job.jar_url)))?;
        step.update(None, Some(&megabytes(fs::metadata(&jar)?.len(), None)), None);
        Ok(())
    })?;
    p.step("basemap", |step| {
        if let Some(parent) = job.output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut java = Command::new(job.java);
        java.arg(format!("-Xmx{}", job.memory))
            .arg("-jar")
            .arg(&jar)
            .arg(format!("--osm-path={}", job.extract.display()))
            .arg(format!("--output={}", job.output.display()))
            .arg(format!("--bounds={}", job.bounds))
            .arg(format!("--minzoom={}", job.minzoom))
            .arg(format!("--maxzoom={}", job.maxzoom))
            .args(["--download", "--force"])
            .arg(format!("--tmpdir={}", job.tools.join("planetiler-tmp").display()))
            .current_dir(job.root);
        step.run(java)?;
        let size = fs::metadata(job.output).map_err(|e| StepError::new([format!("{}: {e}", job.output.display())]))?.len();
        let name = job.output.file_name().unwrap_or_default().to_string_lossy();
        step.update(Some(&name), Some(&format!("{:.1} MB", size as f64 / 1e6)), None);
        Ok(())
    })
}
