#![allow(dead_code)]
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cityloom_prepare::progress::testing::SharedBuf;
use cityloom_prepare::progress::{Progress, Spec};

/// A script named `name` in `dir` that appends its arguments, one invocation per line, to `dir/<name>.log` and then
/// runs `body` (shell, `$LOG` is that file). Returns the directory to put on a command line as the program's path.
pub fn fake_tool(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    let log = dir.join(format!("{name}.log"));
    fs::write(&path, format!("#!/bin/sh\nLOG='{}'\necho \"$@\" >> \"$LOG\"\n{body}\n", log.display())).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

pub fn calls(dir: &Path, name: &str) -> Vec<String> {
    fs::read_to_string(dir.join(format!("{name}.log"))).unwrap_or_default().lines().map(str::to_string).collect()
}

/// A `Progress` that writes plain lines to a shared buffer and keeps its logs under `dir/logs`.
pub fn quiet_progress(specs: Vec<Spec>, dir: &Path) -> (Progress, SharedBuf) {
    let sink = SharedBuf::default();
    let clock = Arc::new(|| 0.0);
    let progress = Progress::with(specs, dir.join("logs"), dir.to_path_buf(), Box::new(sink.clone()), false, clock);
    (progress, sink)
}
