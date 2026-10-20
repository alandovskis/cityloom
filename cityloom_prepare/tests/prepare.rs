mod support;

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use cityloom_prepare::archive::{self, read_index};
use cityloom_prepare::prepare::{self, PrepareJob};
use cityloom_prepare::progress::StepError;
use support::{Reply, fake_tool, quiet_progress, serve};

#[test]
fn the_steps_are_the_scripts_and_pack_is_last() {
    let steps = prepare::steps(std::path::Path::new("/data/quebec-latest.osm.pbf"));
    let keys: Vec<_> = steps.iter().map(|s| s.key).collect();
    assert_eq!(keys, ["extract", "area", "highways", "tracks", "merge", "tiles", "index", "planetiler", "basemap", "pack"]);
    let groups: Vec<_> = steps.iter().map(|s| (s.group, s.subject.as_str())).collect();
    assert_eq!(
        groups,
        [
            ("extract", "quebec-latest.osm.pbf"),
            ("roads", "metro area"),
            ("roads", "highways"),
            ("roads", "designated tracks"),
            ("roads", "merge"),
            ("tiles", "cutting 1,734 tiles"),
            ("tiles", "index"),
            ("basemap", "planetiler.jar"),
            ("basemap", "montreal.pmtiles"),
            ("archive", "montreal.cityloom"),
        ]
    );
}

#[test]
fn megabytes_reads_like_the_script() {
    assert_eq!(prepare::megabytes(1_234_567, Some(2_345_678_901)), "1 of 2,346 MB");
    assert_eq!(prepare::megabytes(999_999, None), "1 MB");
}

/// Runs `get_extract` as the only step of a display; returns the result and the plain output.
fn get(dir: &std::path::Path, refresh: bool, url: &str) -> (Result<(), StepError>, String) {
    let extract = dir.join("data/quebec-latest.osm.pbf");
    let (progress, sink) = quiet_progress(prepare::steps(&extract), dir);
    let result = progress.step("extract", |ctx| prepare::get_extract(ctx, &extract, refresh, url));
    progress.close();
    (result, sink.text())
}

#[test]
fn a_kept_extract_is_not_downloaded() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::write(dir.path().join("data/quebec-latest.osm.pbf"), vec![1u8; 2_000_000]).unwrap();
    let asked = Arc::new(AtomicUsize::new(0));
    let count = asked.clone();
    let (url, _) = serve(move |_| {
        count.fetch_add(1, Ordering::SeqCst);
        Reply::Status(500)
    });
    let (result, text) = get(dir.path(), false, &format!("{url}/q"));
    result.unwrap();
    assert_eq!(asked.load(Ordering::SeqCst), 0);
    assert!(text.contains("2 MB, kept"), "{text}");
}

#[test]
fn refresh_downloads_again() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::write(dir.path().join("data/quebec-latest.osm.pbf"), b"old").unwrap();
    let (url, _) = serve(|_| Reply::Body { status: 200, body: vec![9u8; 1_500_000], content_length: Some(1_500_000) });
    let (result, text) = get(dir.path(), true, &format!("{url}/q"));
    result.unwrap();
    assert_eq!(fs::read(dir.path().join("data/quebec-latest.osm.pbf")).unwrap(), vec![9u8; 1_500_000]);
    assert!(text.contains("PASS extract") && text.contains("2 MB"), "{text}");
    assert!(!text.contains("kept"), "{text}");
}

#[test]
fn a_missing_extract_is_downloaded_with_progress() {
    let dir = tempfile::tempdir().unwrap();
    let (url, _) = serve(|_| Reply::Body { status: 200, body: vec![3u8; 3_000_000], content_length: Some(3_000_000) });
    let (result, text) = get(dir.path(), false, &format!("{url}/q"));
    result.unwrap();
    assert_eq!(fs::metadata(dir.path().join("data/quebec-latest.osm.pbf")).unwrap().len(), 3_000_000);
    assert!(text.contains("1 of 3 MB") || text.contains("2 of 3 MB"), "{text}");
    assert!(text.lines().any(|l| l.starts_with("PASS extract") && l.contains(" 3 MB") && !l.contains("of")), "{text}");
}

#[test]
fn the_whole_pipeline_runs_in_order_against_fake_tools() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let tools = root.join("tools");
    fs::create_dir_all(&tools).unwrap();
    fs::write(tools.join("planetiler.jar"), b"jar").unwrap();
    fs::create_dir_all(root.join("data")).unwrap();
    fs::write(root.join("data/quebec-latest.osm.pbf"), b"x").unwrap();
    let osmium = fake_tool(
        root,
        "osmium",
        r#"if [ "$1" = extract ] && [ "$2" = --config ]; then
  d=$(sed -n 's/.*"directory":"\([^"]*\)".*/\1/p' "$3"); head -c 500 /dev/zero > "$d/0_0.osm.pbf"
fi"#,
    );
    let out = root.join("out");
    let java = fake_tool(root, "java", &format!("mkdir -p '{0}/basemap' && head -c 1000 /dev/zero > '{0}/basemap/montreal.pmtiles'", out.display()));
    // A file another run left in the basemap folder is not the basemap this run built.
    fs::create_dir_all(out.join("basemap")).unwrap();
    fs::write(out.join("basemap/other.pmtiles"), b"stray").unwrap();
    let job = PrepareJob {
        extract: root.join("data/quebec-latest.osm.pbf"),
        refresh: false,
        url: "http://127.0.0.1:1/never".into(),
        batch: 500,
        osmium: osmium.display().to_string(),
        java: java.display().to_string(),
        work: root.join("work"),
        metro: out.join("metro"),
        basemap: out.join("basemap/montreal.pmtiles"),
        tools,
        root: root.to_path_buf(),
        jar_url: "http://127.0.0.1:1/never".into(),
        archive: out.join("montreal.cityloom"),
    };
    let (progress, sink) = quiet_progress(prepare::steps(&job.extract), root);
    prepare::run_job(&progress, &job).unwrap();
    progress.close();
    let text = sink.text();

    let data = root.join("unpacked");
    let unpacked = archive::unpack(&job.archive, &data).unwrap();
    assert_eq!(unpacked.tiles, 1);
    assert_eq!(fs::metadata(data.join("montreal/metro/0_0.osm.pbf")).unwrap().len(), 500);
    assert_eq!(fs::metadata(data.join("montreal/basemap/montreal.pmtiles")).unwrap().len(), 1000);
    assert!(!data.join("montreal/basemap/other.pmtiles").exists(), "a stray basemap went into the archive");
    assert_eq!(fs::read(data.join("montreal/metro/index.json")).unwrap(), fs::read(job.metro.join("index.json")).unwrap());
    let mut tar = tar::Archive::new(fs::File::open(&job.archive).unwrap());
    let mut first = tar.entries().unwrap().next().unwrap().unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut first, &mut bytes).unwrap();
    let header = read_index(&bytes).unwrap().header;
    assert_eq!(header.source, "quebec-latest.osm.pbf");
    assert_eq!((header.grid.nx, header.grid.ny), (51, 34));

    let passes: Vec<_> = text.lines().filter(|l| l.starts_with("PASS ")).map(|l| l.split(' ').nth(1).unwrap().to_string()).collect();
    let groups: Vec<_> = prepare::steps(&job.extract).iter().map(|s| s.group.to_string()).collect();
    assert_eq!(passes.len(), 10, "{text}");
    assert_eq!(passes, groups, "{text}");
    let subjects: Vec<_> = text.lines().filter(|l| l.starts_with("PASS ")).map(|l| l.split(' ').nth(2).unwrap().to_string()).collect();
    assert_eq!(subjects.first().map(String::as_str), Some("quebec-latest.osm.pbf"));
    assert!(subjects.last().unwrap().starts_with("montreal.cityloom"), "{text}");
}

#[test]
fn a_failed_step_stops_the_pipeline_before_pack() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let tools = root.join("tools");
    fs::create_dir_all(&tools).unwrap();
    fs::write(tools.join("planetiler.jar"), b"jar").unwrap();
    fs::create_dir_all(root.join("data")).unwrap();
    fs::write(root.join("data/quebec-latest.osm.pbf"), b"x").unwrap();
    // osmium fails, so the road tiles cannot be made.
    let osmium = fake_tool(root, "osmium", "echo boom >&2; exit 1");
    let marker = root.join("java-ran");
    let java = fake_tool(root, "java", &format!("touch '{}'", marker.display()));
    let out = root.join("out");
    let job = PrepareJob {
        extract: root.join("data/quebec-latest.osm.pbf"),
        refresh: false,
        url: "http://127.0.0.1:1/never".into(),
        batch: 500,
        osmium: osmium.display().to_string(),
        java: java.display().to_string(),
        work: root.join("work"),
        metro: out.join("metro"),
        basemap: out.join("basemap/montreal.pmtiles"),
        tools,
        root: root.to_path_buf(),
        jar_url: "http://127.0.0.1:1/never".into(),
        archive: out.join("montreal.cityloom"),
    };
    let (progress, sink) = quiet_progress(prepare::steps(&job.extract), root);
    let result = prepare::run_job(&progress, &job);
    progress.close();
    let text = sink.text();
    assert!(result.is_err(), "{text}");
    assert!(!job.archive.exists(), "an archive was written after a failed step");
    assert!(!marker.exists(), "a later step ran after the failure");
    assert!(!text.contains("PASS pack") && !text.contains("PASS basemap"), "{text}");
}
