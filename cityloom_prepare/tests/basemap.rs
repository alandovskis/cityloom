mod support;

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use cityloom_prepare::basemap::{self, BasemapJob};
use cityloom_prepare::progress::StepError;
use support::{Reply, calls, fake_tool, quiet_progress, serve};

const GEOFABRIK: &str = "https://download.geofabrik.de/north-america/canada/quebec-latest.osm.pbf";

struct Setup {
    dir: tempfile::TempDir,
}

impl Setup {
    fn new() -> Setup {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("quebec.osm.pbf"), b"x").unwrap();
        fs::create_dir_all(dir.path().join("tools")).unwrap();
        Setup { dir }
    }
    fn p(&self, name: &str) -> String {
        self.dir.path().join(name).display().to_string()
    }
    fn run(&self, java_body: &str, jar_url: &str) -> (Result<(), StepError>, String) {
        let java = fake_tool(self.dir.path(), "java", java_body);
        let (progress, sink) = quiet_progress(basemap::steps("planetiler.jar", "montreal.pmtiles"), self.dir.path());
        let (extract, output, tools) = (self.dir.path().join("quebec.osm.pbf"), self.dir.path().join("out/montreal.pmtiles"), self.dir.path().join("tools"));
        let job = BasemapJob {
            extract: &extract,
            output: &output,
            bounds: basemap::METRO_BOUNDS,
            minzoom: basemap::MINZOOM,
            maxzoom: basemap::MAXZOOM,
            memory: basemap::MEMORY,
            tools: &tools,
            root: self.dir.path(),
            java: java.to_str().unwrap(),
            jar_url,
        };
        let result = basemap::run(&progress, &job);
        progress.close();
        (result, sink.text())
    }
}

/// A java that writes the output, as Planetiler does.
fn writes_output(s: &Setup, bytes: usize) -> String {
    format!("head -c {bytes} /dev/zero > '{}/out/montreal.pmtiles'", s.dir.path().display())
}

#[test]
fn the_steps_are_the_scripts() {
    let steps: Vec<_> = basemap::steps("planetiler.jar", "montreal.pmtiles").into_iter().map(|s| (s.key, s.group, s.subject)).collect();
    assert_eq!(steps, [("planetiler", "basemap", "planetiler.jar".to_string()), ("basemap", "basemap", "montreal.pmtiles".to_string())]);
    assert_eq!(basemap::METRO_BOUNDS, "-74.40,45.20,-73.09,45.81");
    assert_eq!((basemap::MINZOOM, basemap::MAXZOOM, basemap::MEMORY), (0, 14, "4g"));
    assert_eq!(basemap::JAR_URL, "https://github.com/onthegomap/planetiler/releases/latest/download/planetiler.jar");
}

#[test]
fn java_is_called_with_the_scripts_arguments() {
    let s = Setup::new();
    fs::write(s.dir.path().join("tools/planetiler.jar"), b"jar").unwrap();
    // the working directory is the root
    let body = format!("pwd >> '{}/pwd.log'; {}", s.dir.path().display(), writes_output(&s, 2_500_000));
    let (result, text) = s.run(&body, "http://127.0.0.1:1/never");
    result.unwrap();
    let (extract, out, tools) = (s.p("quebec.osm.pbf"), s.p("out/montreal.pmtiles"), s.p("tools"));
    assert_eq!(
        calls(s.dir.path(), "java"),
        [format!(
            "-Xmx4g -jar {tools}/planetiler.jar --osm-path={extract} --output={out} --bounds=-74.40,45.20,-73.09,45.81 --minzoom=0 --maxzoom=14 --download --force --tmpdir={tools}/planetiler-tmp"
        )]
    );
    let pwd = fs::read_to_string(s.dir.path().join("pwd.log")).unwrap();
    assert_eq!(fs::canonicalize(pwd.trim()).unwrap(), fs::canonicalize(s.dir.path()).unwrap());
    assert!(text.contains("kept"), "{text}");
    assert!(text.contains("montreal.pmtiles") && text.contains("2.5 MB"), "{text}");
}

#[test]
fn the_jar_is_downloaded_once_and_its_release_kept() {
    let s = Setup::new();
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    let jar = vec![7u8; 1_500_000];
    let (url, _) = serve(move |request| {
        count.fetch_add(1, Ordering::SeqCst);
        match request.path.as_str() {
            "/latest/planetiler.jar" => Reply::Redirect("/o/releases/download/v9/planetiler.jar".into()),
            "/o/releases/download/v9/planetiler.jar" => Reply::Redirect("/signed".into()),
            "/signed" => Reply::Body { status: 200, body: jar.clone(), content_length: Some(jar.len()) },
            _ => Reply::Status(404),
        }
    });
    let jar_url = format!("{url}/latest/planetiler.jar");
    let java = writes_output(&s, 10);
    let (result, text) = s.run(&java, &jar_url);
    result.unwrap();
    let tools = s.dir.path().join("tools");
    assert_eq!(fs::metadata(tools.join("planetiler.jar")).unwrap().len(), 1_500_000);
    assert!(!tools.join("planetiler.jar.part").exists());
    assert_eq!(fs::read_to_string(tools.join("planetiler.jar.source")).unwrap(), format!("{url}/o/releases/download/v9/planetiler.jar\n"));
    assert!(text.contains("2 MB"), "{text}");
    assert_eq!(requests.load(Ordering::SeqCst), 3);
    let (result, text) = s.run(&java, &jar_url);
    result.unwrap();
    assert_eq!(requests.load(Ordering::SeqCst), 3);
    assert!(text.contains("kept"), "{text}");
}

#[test]
fn a_jar_with_no_release_redirect_is_sourced_from_the_url_asked() {
    let s = Setup::new();
    let (url, _) = serve(|_| Reply::Body { status: 200, body: b"jar".to_vec(), content_length: Some(3) });
    let jar_url = format!("{url}/planetiler.jar");
    s.run(&writes_output(&s, 10), &jar_url).0.unwrap();
    assert_eq!(fs::read_to_string(s.dir.path().join("tools/planetiler.jar.source")).unwrap(), format!("{jar_url}\n"));
}

#[test]
fn a_failed_jar_download_leaves_no_jar_and_no_part() {
    let s = Setup::new();
    let (url, _) = serve(|_| Reply::Body { status: 200, body: vec![1; 10], content_length: Some(100) });
    let (result, _) = s.run("true", &format!("{url}/planetiler.jar"));
    assert!(result.unwrap_err().lines[0].contains("got 10 bytes of 100"));
    assert_eq!(fs::read_dir(s.dir.path().join("tools")).unwrap().count(), 0);
    assert!(calls(s.dir.path(), "java").is_empty());
}

#[test]
fn a_missing_extract_is_a_clear_error() {
    let s = Setup::new();
    fs::remove_file(s.dir.path().join("quebec.osm.pbf")).unwrap();
    let (result, _) = s.run("true", "http://127.0.0.1:1/never");
    assert_eq!(result.unwrap_err().lines, [format!("{} is not there: get it from {GEOFABRIK}", s.p("quebec.osm.pbf"))]);
    assert!(calls(s.dir.path(), "java").is_empty());
}

#[test]
fn a_failing_java_shows_its_log() {
    let s = Setup::new();
    fs::write(s.dir.path().join("tools/planetiler.jar"), b"jar").unwrap();
    let (result, text) = s.run("echo boom; exit 3", "http://127.0.0.1:1/never");
    let error = result.unwrap_err();
    assert!(error.lines.iter().any(|l| l == "java exited with status 3"), "{error}");
    assert!(error.lines.iter().any(|l| l == "boom"));
    assert!(error.lines.iter().any(|l| l.starts_with("full log:")));
    assert!(text.contains("FAIL basemap"), "{text}");
}
