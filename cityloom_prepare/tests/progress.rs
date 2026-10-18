//! Tests of `progress::Progress` over a shared in-memory writer (the pure parts are tested in `frame.rs`).

use std::process::Command;
use std::sync::Arc;

use cityloom_prepare::progress::testing::SharedBuf;
use cityloom_prepare::progress::{Progress, Spec, State, StepError};

fn spec(key: &'static str, subject: &str) -> Spec {
    Spec { key, group: "g", subject: subject.into() }
}

fn clock() -> Arc<dyn Fn() -> f64 + Send + Sync> {
    Arc::new(|| 0.0)
}

struct Run {
    dir: tempfile::TempDir,
    out: SharedBuf,
}

impl Run {
    fn new() -> Run {
        Run { dir: tempfile::tempdir().unwrap(), out: SharedBuf::default() }
    }

    fn progress(&self, specs: Vec<Spec>, fancy: bool) -> Progress {
        let root = self.dir.path().to_path_buf();
        Progress::with(specs, root.join("logs"), root, Box::new(self.out.clone()), fancy, clock())
    }
}

#[test]
fn not_a_terminal_prints_a_line_per_change_and_a_summary() {
    let run = Run::new();
    let p = run.progress(vec![spec("a", "first"), spec("b", "second")], false);
    p.step("a", |_| Ok(())).unwrap();
    p.step("b", |ctx| {
        ctx.update(None, Some("half"), Some(0.5));
        Ok(())
    })
    .unwrap();
    p.close();
    let text = run.out.text();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[..lines.len() - 1],
        ["RUNS g first", "PASS g first 0s", "RUNS g second", "RUNS g second half", "PASS g second half 0s", "Steps: 2 passed, 2 total"]
    );
    assert_eq!(lines.last().unwrap().strip_prefix("Time:  ").and_then(|t| t.strip_suffix('s')).map(|t| t.parse::<u32>().is_ok()), Some(true));
    assert!(!text.contains('\x1b'));
}

#[test]
fn a_failing_command_logs_its_output_and_shows_the_tail_and_the_log_path() {
    let run = Run::new();
    let p = run.progress(vec![spec("a", "first"), spec("b", "second")], false);
    let mut command = Command::new("sh");
    command.args(["-c", "for i in $(seq 0 29); do echo out $i; done; echo bad thing >&2; exit 3"]);
    let error = p.step("a", |ctx| ctx.run(command)).unwrap_err();
    p.close();
    assert_eq!(error.lines[0], "sh exited with status 3");
    let text = run.out.text();
    let lines: Vec<&str> = text.lines().collect();
    let at = lines.iter().position(|l| *l == "FAIL g first").unwrap();
    let after = &lines[at + 1..];
    assert_eq!(after[0], " \u{2023} sh exited with status 3");
    assert_eq!(after[1], " \u{2023} out 16"); // the last 15 lines: out 16 to out 29, then the error
    assert_eq!(after[15], " \u{2023} bad thing");
    assert_eq!(after[16], " \u{2023} full log: logs/a.log");
    assert!(lines.contains(&"Steps: 1 failed, 2 total"));
    let log = std::fs::read_to_string(run.dir.path().join("logs/a.log")).unwrap();
    assert!(log.starts_with("$ sh -c "));
    assert!(log.contains("out 0"));
}

#[test]
fn a_progress_update_of_carriage_returns_is_shown_as_lines() {
    let run = Run::new();
    let p = run.progress(vec![spec("a", "first")], false);
    let mut command = Command::new("sh");
    command.args(["-c", "printf '10%%\\r20%%\\r30%%\\n'; exit 1"]);
    let error = p.step("a", |ctx| ctx.run(command)).unwrap_err();
    assert_eq!(error.lines[2..5], ["10%", "20%", "30%"]);
}

#[test]
fn a_command_that_is_not_there_fails_the_step() {
    let run = Run::new();
    let p = run.progress(vec![spec("a", "first")], false);
    let error = p.step("a", |ctx| ctx.run(Command::new("no-such-program-here"))).unwrap_err();
    assert_eq!(error.lines, ["no-such-program-here: not found on PATH"]);
    p.close();
    assert!(run.out.text().contains(" \u{2023} no-such-program-here: not found"));
}

#[test]
fn an_error_fails_the_step_and_is_returned() {
    let run = Run::new();
    let p = run.progress(vec![spec("a", "first")], false);
    let error = p.step::<()>("a", |_| Err(StepError::new(["oops"]))).unwrap_err();
    assert_eq!(error.lines, ["oops"]);
    assert_eq!(error.to_string(), "oops");
    p.close();
    assert!(run.out.text().contains(" \u{2023} oops"));
}

#[test]
fn an_io_error_is_a_step_error() {
    let run = Run::new();
    let p = run.progress(vec![spec("a", "first")], false);
    let error = p.step::<()>("a", |_| Err(std::io::Error::other("disk full").into())).unwrap_err();
    assert_eq!(error.lines, ["disk full"]);
}

#[test]
fn leaving_the_block_early_still_passes_the_step() {
    fn leave(p: &Progress) -> Result<&'static str, StepError> {
        p.step("a", |_| return Ok("left"))
    }
    let run = Run::new();
    let p = run.progress(vec![spec("a", "first")], false);
    assert_eq!(leave(&p).unwrap(), "left");
    assert_eq!(p.steps()[0].state, State::Passed);
    assert!(run.out.text().contains("PASS g first 0s"));
}

#[test]
fn ctrl_c_fails_the_running_step_and_closes_the_display() {
    let run = Run::new();
    let p = run.progress(vec![spec("a", "first"), spec("b", "second")], false);
    // Whatever the closure returns, an interrupted step stays failed.
    p.step("a", |_| {
        p.interrupt();
        Ok(())
    })
    .unwrap();
    assert_eq!(p.steps()[0].state, State::Failed);
    assert_eq!(p.steps()[0].lines, ["interrupted"]);
    let text = run.out.text();
    assert!(text.contains("FAIL g first"));
    assert!(text.contains(" \u{2023} interrupted"));
    assert!(text.contains("Steps: 1 failed, 2 total"));
    assert!(!text.contains("PASS"));
    let before = text.len();
    p.close(); // idempotent: nothing more is drawn
    assert_eq!(run.out.text().len(), before);
}

#[test]
fn nothing_is_drawn_after_an_interrupt() {
    for fancy in [false, true] {
        let run = Run::new();
        let p = run.progress(vec![spec("a", "first"), spec("b", "second")], fancy);
        p.interrupt();
        let before = run.out.text();
        assert!(before.contains("Steps: 2 total"));
        p.step("b", |ctx| {
            ctx.update(None, Some("late"), Some(0.9));
            Ok(())
        })
        .unwrap();
        assert_eq!(run.out.text(), before, "fancy: {fancy}");
    }
}

#[test]
fn a_panic_in_a_step_fails_it() {
    let run = Run::new();
    let p = run.progress(vec![spec("a", "first")], false);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = p.step::<()>("a", |_| panic!("boom"));
    }));
    assert!(caught.is_err());
    assert_eq!(p.steps()[0].state, State::Failed);
    p.close();
    let text = run.out.text();
    assert!(text.contains("FAIL g first"));
    assert!(!text.contains("RUNS g first\nSteps"), "{text}");
}

#[test]
fn ctrl_c_in_a_real_process_fails_the_step_and_exits_with_status_130() {
    use std::io::{BufRead, BufReader, Read};
    use std::process::{Command, Stdio};
    let exe = std::env::current_exe().unwrap();
    let example = exe.parent().unwrap().parent().unwrap().join("examples/ctrl_c");
    let mut child = Command::new(example).stdout(Stdio::piped()).spawn().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert_eq!(line, "RUNS g first\n"); // the step is running
    let sent = Command::new("kill").args(["-INT", &child.id().to_string()]).status().unwrap();
    assert!(sent.success());
    let mut rest = String::new();
    stdout.read_to_string(&mut rest).unwrap();
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(130));
    assert!(rest.contains("FAIL g first"), "{rest}");
    assert!(rest.contains(" \u{2023} interrupted"), "{rest}");
}

#[test]
fn a_log_outside_the_project_is_shown_whole() {
    let out = SharedBuf::default();
    let p = Progress::with(vec![], "/logs".into(), "/project".into(), Box::new(out), false, clock());
    assert_eq!(p.shown(std::path::Path::new("/project/data/logs/a.log")), "data/logs/a.log");
    assert_eq!(p.shown(std::path::Path::new("/elsewhere/a.log")), "/elsewhere/a.log");
}

#[test]
fn a_run_on_a_terminal_hides_the_cursor_draws_its_steps_and_shows_the_cursor_again() {
    let run = Run::new();
    let root = run.dir.path().to_path_buf();
    let real: Arc<dyn Fn() -> f64 + Send + Sync> = {
        let start = std::time::Instant::now();
        Arc::new(move || start.elapsed().as_secs_f64())
    };
    let p = Progress::with(vec![spec("a", "first")], root.join("logs"), root, Box::new(run.out.clone()), true, real);
    p.step("a", |ctx| {
        ctx.update(None, Some("half"), Some(0.5));
        std::thread::sleep(std::time::Duration::from_millis(250)); // long enough for the ticker to draw once
        Ok(())
    })
    .unwrap();
    p.close();
    let text = run.out.text();
    assert!(text.starts_with("\x1b[?25l"));
    assert!(text.ends_with("\x1b[?25h"));
    let plain = strip_ansi(&text);
    assert!(plain.contains("RUNS  g  first  half"));
    assert!(plain.contains("PASS  g  first  half  0s"));
    assert!(plain.contains("Steps: 1 passed, 1 total"));
}

#[test]
fn redraws_over_the_live_lines_and_leaves_the_cursor_shown() {
    let run = Run::new();
    let p = run.progress(vec![spec("a", "first"), spec("b", "second")], true);
    p.step("a", |_| Ok(())).unwrap();
    p.step("b", |_| Ok(())).unwrap();
    p.close();
    let text = run.out.text();
    assert!(text.starts_with("\x1b[?25l"));
    assert!(text.contains("A\r\x1b[J"));
    assert!(text.ends_with("\x1b[?25h"));
}

#[test]
fn no_color_or_a_dumb_terminal_gets_plain_lines() {
    for (no_color, term) in [(Some("1"), None), (None, Some("dumb"))] {
        let run = Run::new();
        let root = run.dir.path().to_path_buf();
        let p = Progress::detect(vec![], root.join("logs"), root, Box::new(run.out.clone()), true, no_color, term);
        assert!(!p.fancy());
    }
    let run = Run::new();
    let root = run.dir.path().to_path_buf();
    let p = Progress::detect(vec![], root.join("logs"), root, Box::new(run.out.clone()), true, None, Some("xterm"));
    assert!(p.fancy());
    p.close();
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            for d in chars.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}
