//! The two renderers: one plain line per change (a pipe, CI), or a terminal that redraws its live lines in place.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::frame::{Throttle, blink_on, finished_block, frame, plain_lines};
use super::{Clock, State, Step, Steps};

const TICK: Duration = Duration::from_millis(100);

pub trait View: Send {
    fn begin(&mut self);
    fn started(&mut self, step: &Step);
    fn updated(&mut self, step: &Step);
    fn finished(&mut self, step: &Step);
    fn close(&mut self);
}

/// One line per change, as for a pipe or CI.
pub struct Plain {
    out: Box<dyn Write + Send>,
    steps: Steps,
    clock: Clock,
    throttle: Option<Throttle>,
    t0: f64,
}

impl Plain {
    pub fn new(out: Box<dyn Write + Send>, steps: Steps, clock: Clock) -> Plain {
        let t0 = clock();
        Plain { out, steps, clock, throttle: None, t0 }
    }

    fn say(&mut self, lines: Vec<String>) {
        let text: String = lines.iter().map(|l| format!("{l}\n")).collect();
        let _ = self.out.write_all(text.as_bytes());
        let _ = self.out.flush();
    }
}

impl View for Plain {
    fn begin(&mut self) {}

    fn started(&mut self, step: &Step) {
        self.throttle = Some(Throttle::new((self.clock)()));
        self.say(plain_lines(step));
    }

    fn updated(&mut self, step: &Step) {
        let now = (self.clock)();
        if self.throttle.as_mut().is_some_and(|t| t.due(step.fraction, now)) {
            self.say(plain_lines(step));
        }
    }

    fn finished(&mut self, step: &Step) {
        self.say(plain_lines(step));
    }

    fn close(&mut self) {
        let steps = self.steps.lock().unwrap().clone();
        let lines = frame(&steps, (self.clock)() - self.t0, true, false, false);
        self.say(lines);
    }
}

#[derive(Default)]
struct Drawn {
    pending: Vec<String>,
    after_pass: bool,
    last: Option<Vec<String>>,
    lines: usize,
}

struct Inner {
    out: Mutex<Box<dyn Write + Send>>,
    steps: Steps,
    clock: Clock,
    t0: f64,
    drawn: Mutex<Drawn>,
    stop: AtomicBool,
}

impl Inner {
    fn write(&self, text: &str) {
        let mut out = self.out.lock().unwrap();
        let _ = out.write_all(text.as_bytes());
        let _ = out.flush();
    }

    fn draw(&self) {
        let steps = self.steps.lock().unwrap().clone();
        let elapsed = (self.clock)() - self.t0;
        let mut d = self.drawn.lock().unwrap();
        let live = frame(&steps, elapsed, blink_on(elapsed), true, d.after_pass);
        if d.pending.is_empty() && d.last.as_ref() == Some(&live) {
            return;
        }
        let mut text = if d.lines > 0 { format!("\x1b[{}A\r\x1b[J", d.lines) } else { String::new() };
        for line in d.pending.iter().chain(live.iter()) {
            text.push_str(line);
            text.push('\n');
        }
        self.write(&text);
        d.pending.clear();
        d.lines = live.len();
        d.last = Some(live);
    }
}

/// Finished steps are printed once and kept; the live lines below them are drawn again in place.
pub struct Terminal {
    inner: Arc<Inner>,
    ticker: Option<JoinHandle<()>>,
}

impl Terminal {
    pub fn new(out: Box<dyn Write + Send>, steps: Steps, clock: Clock) -> Terminal {
        let t0 = clock();
        let inner = Inner { out: Mutex::new(out), steps, clock, t0, drawn: Mutex::new(Drawn::default()), stop: AtomicBool::new(false) };
        Terminal { inner: Arc::new(inner), ticker: None }
    }

    #[cfg(test)]
    fn draw(&self) {
        self.inner.draw();
    }

    /// How many live lines were drawn last.
    #[cfg(test)]
    fn drawn_lines(&self) -> usize {
        self.inner.drawn.lock().unwrap().lines
    }
}

impl View for Terminal {
    fn begin(&mut self) {
        self.inner.write("\x1b[?25l"); // hide the cursor
        let inner = Arc::clone(&self.inner);
        self.ticker = Some(thread::spawn(move || {
            while !inner.stop.load(Ordering::SeqCst) {
                thread::park_timeout(TICK);
                if !inner.stop.load(Ordering::SeqCst) {
                    inner.draw();
                }
            }
        }));
    }

    fn started(&mut self, _step: &Step) {
        self.inner.draw();
    }

    fn updated(&mut self, _step: &Step) {} // the ticker draws it

    fn finished(&mut self, step: &Step) {
        // Not drawn yet: the next step's start does it, so the live lines do not flicker between two steps.
        let mut d = self.inner.drawn.lock().unwrap();
        d.pending.extend(finished_block(step, true));
        d.after_pass = step.state == State::Passed;
    }

    fn close(&mut self) {
        self.inner.stop.store(true, Ordering::SeqCst);
        if let Some(ticker) = self.ticker.take() {
            ticker.thread().unpark();
            let _ = ticker.join();
        }
        self.inner.draw();
        self.inner.write("\x1b[?25h"); // show the cursor again
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::testing::SharedBuf;

    #[test]
    fn redraws_over_the_live_lines_and_leaves_the_cursor_shown() {
        let out = SharedBuf::default();
        let steps: Steps = Arc::new(Mutex::new(vec![Step::new("a", "g", "first"), Step::new("b", "g", "second")]));
        let clock: Clock = Arc::new(|| 0.0);
        let mut term = Terminal::new(Box::new(out.clone()), Arc::clone(&steps), clock);
        // begin() also starts the ticker, which a test leaves out
        term.inner.write("\x1b[?25l");
        steps.lock().unwrap()[0].state = State::Running;
        term.draw();
        assert!(out.text().contains("\x1b[?25l"));
        let lines_drawn = term.drawn_lines();
        assert!(lines_drawn > 0);
        let first = {
            let mut s = steps.lock().unwrap();
            s[0].state = State::Passed;
            s[0].took = Some(1.0);
            s[0].clone()
        };
        term.finished(&first);
        steps.lock().unwrap()[1].state = State::Running;
        term.draw();
        assert!(out.text().contains(&format!("\x1b[{lines_drawn}A")));
        term.close();
        assert!(out.text().ends_with("\x1b[?25h"));
    }

    #[test]
    fn plain_view_says_nothing_for_an_update_before_a_step_started() {
        let out = SharedBuf::default();
        let steps: Steps = Arc::new(Mutex::new(vec![Step::new("a", "g", "first")]));
        let mut plain = Plain::new(Box::new(out.clone()), steps, Arc::new(|| 0.0));
        let mut step = Step::new("a", "g", "first");
        step.fraction = Some(0.5);
        plain.updated(&step);
        assert_eq!(out.text(), "");
    }
}
