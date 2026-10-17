//! How `cityloom_prepare` shows its progress: the look of a Jest run (as in JakeWharton/mosaic's sample), in the
//! terminal. A port of `scripts/include/progress.py`.
//!
//! ```text
//!  PASS  extract  quebec-latest.osm.pbf  1,213 MB  42s
//!  RUNS  tiles  cutting 1,734 tiles  batch 9 of 14
//!
//! Steps: 1 passed, 1 running, 6 total
//! Time:  97s
//! [40 columns: failed in red, passed in green, running in blinking yellow, the rest grey]
//! ```
//!
//! A run lists its steps up front (`Spec`), so the total is right from the start, then does each in `Progress::step`.
//! The output of the tools it runs goes to a log under `data/logs` and a failure shows the log's last lines. Not a
//! terminal (a pipe, a file, CI), or `NO_COLOR`, or `TERM=dumb`: one plain line per change, nothing redrawn.
//! What is drawn is a pure function of the steps (`frame`), so the tests read it.

pub mod frame;
pub mod testing;
mod view;

use std::fmt::{self, Display};
use std::fs::{self, File, OpenOptions};
use std::io::{self, IsTerminal, Read, Seek, SeekFrom, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use frame::{TAIL_LINES, tail};
use view::{Plain, Terminal, View};

pub type Clock = Arc<dyn Fn() -> f64 + Send + Sync>;
type Steps = Arc<Mutex<Vec<Step>>>;

/// How many bytes at the end of a log are read for a failure's tail.
const TAIL_BYTES: u64 = 1 << 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Pending,
    Running,
    Passed,
    Failed,
}

/// A step a run will do: `key` names its log, `group` and `subject` are what the row says.
pub struct Spec {
    pub key: &'static str,
    pub group: &'static str,
    pub subject: String,
}

#[derive(Clone, Debug)]
pub struct Step {
    pub key: String,
    pub group: String,
    pub subject: String,
    pub state: State,
    pub detail: String,
    pub fraction: Option<f64>,
    pub started: f64,
    pub took: Option<f64>,
    pub lines: Vec<String>,
}

impl Step {
    pub fn new(key: &str, group: &str, subject: &str) -> Step {
        Step {
            key: key.into(),
            group: group.into(),
            subject: subject.into(),
            state: State::Pending,
            detail: String::new(),
            fraction: None,
            started: 0.0,
            took: None,
            lines: Vec::new(),
        }
    }
}

/// A step failed; `lines` are what to tell the user, one per line.
#[derive(Clone, Debug)]
pub struct StepError {
    pub lines: Vec<String>,
}

impl StepError {
    pub fn new(lines: impl IntoIterator<Item = impl Into<String>>) -> StepError {
        StepError { lines: lines.into_iter().map(Into::into).collect() }
    }
}

impl From<io::Error> for StepError {
    fn from(error: io::Error) -> StepError {
        StepError::new([error.to_string()])
    }
}

impl Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.lines.join("; "))
    }
}

impl std::error::Error for StepError {}

/// A number with its thousands grouped by commas, as Python's `{:,}` and `{:,.0f}` write it. It takes anything
/// that displays as a number: `thousands(1_234_567)` is `1,234,567`, `thousands(format!("{x:.0}"))` groups a rounded
/// float. A sign and a decimal part are kept as they are.
pub fn thousands(value: impl Display) -> String {
    let text = value.to_string();
    let (sign, rest) = match text.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", text.as_str()),
    };
    let (whole, decimals) = rest.split_once('.').map_or((rest, ""), |(w, d)| (w, d));
    let mut grouped = String::new();
    for (i, digit) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    let point = if decimals.is_empty() && !rest.contains('.') { "" } else { "." };
    format!("{sign}{grouped}{point}{decimals}")
}

/// The steps of one run. Each step is done in `step`; the display is closed with `close` (also when it is dropped).
pub struct Progress {
    steps: Steps,
    view: Mutex<Box<dyn View>>,
    logs: PathBuf,
    root: PathBuf,
    clock: Clock,
    fancy: bool,
    closed: AtomicBool,
}

impl Progress {
    /// On standard error: a terminal when it is one, `NO_COLOR` is unset and `TERM` is not `dumb`, else plain lines.
    pub fn stderr(specs: Vec<Spec>, logs: PathBuf, root: PathBuf) -> Progress {
        let no_color = std::env::var("NO_COLOR").ok();
        let term = std::env::var("TERM").ok();
        Progress::detect(specs, logs, root, Box::new(io::stderr()), io::stderr().is_terminal(), no_color.as_deref(), term.as_deref())
    }

    /// Chooses the view as `stderr` does, from what the caller says of its output and environment.
    pub fn detect(
        specs: Vec<Spec>,
        logs: PathBuf,
        root: PathBuf,
        out: Box<dyn Write + Send>,
        isatty: bool,
        no_color: Option<&str>,
        term: Option<&str>,
    ) -> Progress {
        let start = Instant::now();
        let clock: Clock = Arc::new(move || start.elapsed().as_secs_f64());
        Progress::with(specs, logs, root, out, frame::is_fancy(isatty, no_color, term), clock)
    }

    pub fn with(specs: Vec<Spec>, logs: PathBuf, root: PathBuf, out: Box<dyn Write + Send>, fancy: bool, clock: Clock) -> Progress {
        let steps: Steps = Arc::new(Mutex::new(specs.iter().map(|s| Step::new(s.key, s.group, &s.subject)).collect()));
        let mut view: Box<dyn View> = if fancy {
            Box::new(Terminal::new(out, Arc::clone(&steps), Arc::clone(&clock)))
        } else {
            Box::new(Plain::new(out, Arc::clone(&steps), Arc::clone(&clock)))
        };
        view.begin();
        Progress { steps, view: Mutex::new(view), logs, root, clock, fancy, closed: AtomicBool::new(false) }
    }

    /// Whether the display is drawn in place (a terminal) and not one line per change.
    pub fn fancy(&self) -> bool {
        self.fancy
    }

    /// A copy of the steps as they are now.
    pub fn steps(&self) -> Vec<Step> {
        self.steps.lock().unwrap().clone()
    }

    pub fn log_path(&self, key: &str) -> PathBuf {
        self.logs.join(format!("{key}.log"))
    }

    /// A path as the user reads it: relative to the project when it is under it, else whole.
    pub fn shown(&self, path: &Path) -> String {
        path.strip_prefix(&self.root).unwrap_or(path).display().to_string()
    }

    fn mutate(&self, key: &str, change: impl FnOnce(&mut Step)) -> Step {
        let mut steps = self.steps.lock().unwrap();
        let step = steps.iter_mut().find(|s| s.key == key).unwrap_or_else(|| panic!("no step named {key}"));
        change(step);
        step.clone()
    }

    /// Runs a step: it passes when `f` returns `Ok` (with its time) and fails with `StepError.lines` when it returns
    /// `Err`, which is returned. Whatever way `f` is left, the step ends in a state and the display is told.
    pub fn step<T>(&self, key: &str, f: impl FnOnce(&StepCtx) -> Result<T, StepError>) -> Result<T, StepError> {
        let _ = fs::create_dir_all(&self.logs);
        let _ = fs::remove_file(self.log_path(key));
        let now = (self.clock)();
        let snapshot = self.mutate(key, |s| {
            s.state = State::Running;
            s.started = now;
        });
        self.view.lock().unwrap().started(&snapshot);
        let result = f(&StepCtx { progress: self, key: key.to_string() });
        let now = (self.clock)();
        let snapshot = self.mutate(key, |s| match &result {
            Ok(_) => {
                s.state = State::Passed;
                s.took = Some(now - s.started);
            }
            Err(error) => {
                s.state = State::Failed;
                s.lines = error.lines.clone();
            }
        });
        self.view.lock().unwrap().finished(&snapshot);
        result
    }

    /// Ctrl-C: every running step fails as "interrupted", the closing frame is drawn and the cursor shown. The caller
    /// then exits with status 130 (`exit_on_ctrl_c` does).
    pub fn interrupt(&self) {
        let running: Vec<String> = self.steps().into_iter().filter(|s| s.state == State::Running).map(|s| s.key).collect();
        for key in running {
            let snapshot = self.mutate(&key, |s| {
                s.state = State::Failed;
                s.lines = vec!["interrupted".into()];
            });
            self.view.lock().unwrap().finished(&snapshot);
        }
        self.close();
    }

    /// Stops the ticker, draws the final frame and shows the cursor. Idempotent; also done when it is dropped.
    pub fn close(&self) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            self.view.lock().unwrap().close();
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.close();
    }
}

/// Installs the Ctrl-C handler: it ends the display (`interrupt`) and exits with status 130, as the Python did.
pub fn exit_on_ctrl_c(progress: &Arc<Progress>) -> Result<(), ctrlc::Error> {
    let progress = Arc::clone(progress);
    ctrlc::set_handler(move || {
        progress.interrupt();
        std::process::exit(130);
    })
}

/// What a step does with its row and its tools.
pub struct StepCtx<'a> {
    progress: &'a Progress,
    key: String,
}

impl StepCtx<'_> {
    /// Changes what the row says; `fraction` (0 to 1) paces the plain view's progress lines.
    pub fn update(&self, subject: Option<&str>, detail: Option<&str>, fraction: Option<f64>) {
        let snapshot = self.progress.mutate(&self.key, |s| {
            if let Some(subject) = subject {
                s.subject = subject.into();
            }
            if let Some(detail) = detail {
                s.detail = detail.into();
            }
            s.fraction = fraction;
        });
        self.progress.view.lock().unwrap().updated(&snapshot);
    }

    /// Runs `command` with its output in the step's log; a failure shows the log's last lines and its path.
    pub fn run(&self, mut command: Command) -> Result<(), StepError> {
        let log_path = self.progress.log_path(&self.key);
        let program = command.get_program().to_os_string();
        let name = Path::new(&program).file_name().unwrap_or(&program).to_string_lossy().into_owned();
        let line: Vec<String> =
            std::iter::once(program.to_string_lossy().into_owned()).chain(command.get_args().map(|a| a.to_string_lossy().into_owned())).collect();
        let mut log = OpenOptions::new().create(true).append(true).open(&log_path)?;
        writeln!(log, "$ {}", line.join(" "))?;
        log.flush()?;
        command.stdout(Stdio::from(log.try_clone()?)).stderr(Stdio::from(log));
        let status = match command.status() {
            Ok(status) => status,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(StepError::new([format!("{name}: not found on PATH")]));
            }
            Err(e) => return Err(e.into()),
        };
        if status.success() {
            return Ok(());
        }
        let code = status.code().or_else(|| status.signal().map(|s| -s)).unwrap_or(-1);
        let mut lines = vec![format!("{name} exited with status {code}")];
        lines.extend(tail(&read_tail(&log_path), TAIL_LINES));
        lines.push(format!("full log: {}", self.progress.shown(&log_path)));
        Err(StepError::new(lines))
    }
}

fn read_tail(path: &Path) -> String {
    let read = || -> io::Result<Vec<u8>> {
        let mut log = File::open(path)?;
        let len = log.seek(SeekFrom::End(0))?;
        log.seek(SeekFrom::Start(len.saturating_sub(TAIL_BYTES)))?;
        let mut bytes = Vec::new();
        log.read_to_end(&mut bytes)?;
        Ok(bytes)
    };
    String::from_utf8_lossy(&read().unwrap_or_default()).into_owned()
}

#[cfg(test)]
mod tests {
    use super::thousands;

    #[test]
    fn numbers_are_grouped_by_thousands_as_python_writes_them() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(1_213_456_789u64), "1,213,456,789");
        assert_eq!(thousands(-1234567), "-1,234,567");
        assert_eq!(thousands(format!("{:.0}", 1_733.6_f64)), "1,734");
        assert_eq!(thousands(format!("{:.1}", 12345.25_f64)), "12,345.2");
    }
}
