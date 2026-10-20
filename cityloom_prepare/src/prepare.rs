//! The `prepare` command: the Quebec extract (downloaded if it is not kept), the road tiles, the basemap and the
//! archive, in one display. A port of `scripts/prepare.py` with the pack step added last.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::basemap::{self, BasemapJob};
use crate::commands::{self, PackJob};
use crate::download::download;
use crate::progress::{Progress, Spec, StepCtx, StepError, exit_on_ctrl_c};
use crate::tiles::{self, TilesJob};

const ARCHIVE_NAME: &str = "montreal.cityloom";
pub use crate::progress::megabytes;
pub const URL: &str = "https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf";

/// What the command was asked, as the command line says it.
pub struct PrepareArgs {
    pub extract: Option<PathBuf>,
    pub refresh: bool,
    pub batch: Option<usize>,
}

/// Everything `run_job` touches, so a test points the tools and the folders at temporary ones.
pub struct PrepareJob {
    pub extract: PathBuf,
    pub refresh: bool,
    pub url: String,
    pub batch: usize,
    pub osmium: String,
    pub java: String,
    /// osmium's intermediate files.
    pub work: PathBuf,
    /// The road tiles' folder.
    pub metro: PathBuf,
    pub basemap: PathBuf,
    /// Where `planetiler.jar` is kept.
    pub tools: PathBuf,
    /// The working directory of Java.
    pub root: PathBuf,
    pub jar_url: String,
    pub archive: PathBuf,
}

/// What the run does, in order: the script's steps and the archive last.
pub fn steps(extract: &Path) -> Vec<Spec> {
    let name = extract.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let mut all = vec![Spec { key: "extract", group: "extract", subject: name }];
    all.extend(tiles::steps());
    all.extend(basemap::steps("planetiler.jar", "montreal.pmtiles"));
    all.extend(commands::pack_steps(Path::new(ARCHIVE_NAME)));
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

/// The whole pipeline through `p`, whose steps are `steps(&job.extract)`.
pub fn run_job(p: &Progress, job: &PrepareJob) -> Result<(), StepError> {
    p.step("extract", |ctx| get_extract(ctx, &job.extract, job.refresh, &job.url))?;
    tiles::run(p, &TilesJob { source: &job.extract, work: &job.work, out: &job.metro, batch: job.batch, osmium: &job.osmium })?;
    basemap::run(
        p,
        &BasemapJob {
            extract: &job.extract,
            output: &job.basemap,
            bounds: basemap::METRO_BOUNDS,
            minzoom: basemap::MINZOOM,
            maxzoom: basemap::MAXZOOM,
            memory: basemap::MEMORY,
            tools: &job.tools,
            root: &job.root,
            java: &job.java,
            jar_url: &job.jar_url,
        },
    )?;
    let pack = PackJob {
        out: job.archive.clone(),
        area: "montreal".into(),
        metro: job.metro.clone(),
        basemap: job.basemap.parent().unwrap_or(&job.root).to_path_buf(),
        source: job.extract.file_name().unwrap_or_default().to_string_lossy().into_owned(),
        source_time: commands::source_time(&job.extract, &job.osmium),
        created: commands::now(),
        grid: Some(tiles::grid()),
    };
    commands::run_pack(p, &pack)
}

/// The command: the default job for `args`, run under a display on stderr. The error is the exit code.
pub fn run(args: PrepareArgs) -> Result<(), i32> {
    let root = commands::repo_root();
    let job = PrepareJob {
        // An absolute path, so it cannot be read by a tool it is handed to as an option.
        extract: commands::absolute(&args.extract.unwrap_or_else(|| root.join("data/quebec-latest.osm.pbf"))),
        refresh: args.refresh,
        url: URL.into(),
        batch: args.batch.unwrap_or(tiles::MAX_EXTRACTS),
        osmium: "osmium".into(),
        java: "java".into(),
        work: root.join("data/metro-work"),
        metro: root.join("web/data/metro"),
        basemap: root.join("web/data/basemap/montreal.pmtiles"),
        tools: root.join(".tools"),
        root: root.clone(),
        jar_url: basemap::JAR_URL.into(),
        archive: root.join("data").join(ARCHIVE_NAME),
    };
    let progress = Arc::new(Progress::stderr(steps(&job.extract), root.join("data/logs"), root));
    exit_on_ctrl_c(&progress).expect("the Ctrl-C handler");
    let result = run_job(&progress, &job);
    progress.close();
    result.map_err(|_| 1)
}
