//! A real Ctrl-C to a real process. The test binary runs itself again, with `CHILD` set, as the process that gets the
//! signal: that works wherever the tests are built (plain `cargo test`, `cargo test --tests`, `cargo llvm-cov`), where
//! a separately built example is not always there.

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use cityloom_prepare::progress::{Progress, Spec, exit_on_ctrl_c};

const CHILD: &str = "CITYLOOM_CTRL_C_CHILD";
const CHILD_TEST: &str = "the_process_that_gets_ctrl_c";

/// Kills and reaps the child when dropped, so a failed assertion never leaves it running.
struct Guard(Child);

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Run by `ctrl_c_in_a_real_process_...` in a process of its own; does nothing in an ordinary run.
#[test]
fn the_process_that_gets_ctrl_c() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("cityloom-ctrl-c-{}", std::process::id()));
    let spec = Spec { key: "a", group: "g", subject: "first".into() };
    let out = Box::new(std::io::stdout());
    let clock = Arc::new(|| 0.0);
    let progress = Arc::new(Progress::with(vec![spec], dir.join("logs"), dir, out, false, clock));
    exit_on_ctrl_c(&progress).unwrap();
    let _ = progress.step("a", |_| {
        std::thread::sleep(std::time::Duration::from_secs(60));
        Ok(())
    });
}

#[test]
fn ctrl_c_in_a_real_process_fails_the_step_and_exits_with_status_130() {
    let exe = std::env::current_exe().unwrap();
    let mut child = Command::new(exe).args(["--exact", CHILD_TEST, "--nocapture", "--test-threads=1"]).env(CHILD, "1").stdout(Stdio::piped()).spawn().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let pid = child.id();
    let mut guard = Guard(child);
    // The harness's own lines come first; the step's line may follow "test <name> ... " on the same line.
    let mut line = String::new();
    loop {
        line.clear();
        assert_ne!(stdout.read_line(&mut line).unwrap(), 0, "the process ended before its step ran");
        if line.contains("RUNS g first") {
            break; // the step is running
        }
    }
    let sent = Command::new("kill").args(["-INT", &pid.to_string()]).status().unwrap();
    assert!(sent.success());
    let mut rest = String::new();
    stdout.read_to_string(&mut rest).unwrap();
    let status = guard.0.wait().unwrap();
    assert_eq!(status.code(), Some(130));
    assert!(rest.contains("FAIL g first"), "{rest}");
    assert!(rest.contains(" \u{2023} interrupted"), "{rest}");
}
