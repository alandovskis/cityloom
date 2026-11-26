//! The `prepare` command: a city's extract (downloaded if it is not kept), its network tiles and its archive, in one
//! display, all as the city's config (`city`) says. It replaces the former Python `prepare`, with the pack step added
//! last.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::city::{self, City};
use crate::commands::{self, PackJob};
use crate::download::download;
use crate::progress::{Progress, Spec, StepCtx, StepError, exit_on_ctrl_c};
use crate::tiles::{self, TilesJob};

pub use crate::progress::megabytes;

/// What the command was asked, as the command line says it.
pub struct PrepareArgs {
    pub city: Option<String>,
    pub extract: Option<PathBuf>,
    pub refresh: bool,
}

/// Everything `run_job` touches, so a test points the tools and the folders at temporary ones.
pub struct PrepareJob {
    pub city: City,
    pub extract: PathBuf,
    pub refresh: bool,
    pub osmium: String,
    /// osmium's intermediate files.
    pub work: PathBuf,
    /// The network tiles' folder.
    pub metro: PathBuf,
    pub root: PathBuf,
    pub archive: PathBuf,
}

/// What the run does, in order: the script's steps and the archive last.
pub fn steps(job: &PrepareJob) -> Vec<Spec> {
    let name = job.extract.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let mut all = vec![Spec { key: "extract", group: "extract", subject: name }];
    all.extend(tiles::steps(&job.city.grid.grid()));
    all.extend(commands::pack_steps(&job.archive));
    all
}

/// The extract, downloaded unless it is kept (and not `refresh`); the step says its size, and "kept" if it was.
pub fn get_extract(ctx: &StepCtx, extract: &Path, refresh: bool, url: &str) -> Result<(), StepError> {
    let downloading = refresh || !extract.exists();
    if downloading {
        let mut say = |done: u64, total: Option<u64>| {
            ctx.update(None, Some(&megabytes(done, total)), total.filter(|t| *t > 0).map(|t| done as f64 / t as f64));
        };
        download(url, extract, true, &mut say)?;
    }
    let size = megabytes(fs::metadata(extract)?.len(), None);
    ctx.update(None, Some(&if downloading { size } else { format!("{size}, kept") }), None);
    Ok(())
}

/// The whole pipeline through `p`, whose steps are `steps(job)`.
pub fn run_job(p: &Progress, job: &PrepareJob) -> Result<(), StepError> {
    p.step("extract", |ctx| get_extract(ctx, &job.extract, job.refresh, &job.city.extract_url))?;
    let tiles_job = TilesJob {
        source: &job.extract,
        work: &job.work,
        out: &job.metro,
        osmium: &job.osmium,
        grid: job.city.grid.grid(),
        margin: job.city.margin,
        url: &job.city.extract_url,
        extract_name: job.city.extract_name(),
    };
    tiles::run(p, &tiles_job)?;
    let pack = PackJob {
        out: job.archive.clone(),
        area: job.city.id.clone(),
        metro: job.metro.clone(),
        source: job.extract.file_name().unwrap_or_default().to_string_lossy().into_owned(),
        source_time: commands::source_time(&job.extract, &job.osmium),
        created: commands::now(),
        grid: Some(job.city.grid.grid()),
    };
    commands::run_pack(p, &pack)
}

/// The job the command line asks for, with the city's paths under `root`; nothing is downloaded or written.
pub fn job(root: &Path, args: PrepareArgs) -> Result<PrepareJob, String> {
    let city = city::choose(root, args.city.as_deref())?;
    // An absolute path, so it cannot be read by a tool it is handed to as an option.
    let extract = args.extract.map_or_else(|| city.extract(root), |given| commands::absolute(&given));
    Ok(PrepareJob {
        extract,
        refresh: args.refresh,
        osmium: "osmium".into(),
        work: city.work(root),
        metro: root.join("web/data/metro"),
        root: root.to_path_buf(),
        archive: city.archive(root),
        city,
    })
}

/// The command in `root`: the job for `args`, run under a display on stderr. A city that cannot be had is said
/// before anything is shown, downloaded or written. The error is the exit code.
pub fn run_at(root: &Path, args: PrepareArgs) -> Result<(), i32> {
    let job = job(root, args).map_err(|error| {
        eprintln!("{error}");
        1
    })?;
    let progress = Arc::new(Progress::stderr(steps(&job), root.join("data/logs"), root.to_path_buf()));
    exit_on_ctrl_c(&progress).expect("the Ctrl-C handler");
    let result = run_job(&progress, &job);
    progress.close();
    result.map_err(|_| 1)
}

/// The command in the repository. The error is the exit code.
pub fn run(args: PrepareArgs) -> Result<(), i32> {
    run_at(&commands::repo_root(), args)
}
