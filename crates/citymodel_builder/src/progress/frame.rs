//! What is drawn, as pure functions of the steps (a translation of the pure part of the former Python progress module).

use super::{State, Step};

pub const BAR_WIDTH: usize = 40;
pub const BLINK_SECONDS: f64 = 0.5;
pub const TAIL_LINES: usize = 15;
pub const PLAIN_EVERY_SECONDS: f64 = 30.0;
pub const BULLET: &str = "‣";
const GREY: &str = "48;2;128;128;128";

/// The label and the colour of a state's badge: black text, the background says the state.
fn badge_parts(state: State) -> (&'static str, &'static str) {
    match state {
        State::Running => ("RUNS", "30;43"),
        State::Passed => ("PASS", "30;42"),
        State::Failed => ("FAIL", "30;41"),
        State::Pending => ("", ""),
    }
}

pub fn paint(text: &str, code: &str, color: bool) -> String {
    if color { format!("\x1b[{code}m{text}\x1b[0m") } else { text.to_string() }
}

pub fn duration(seconds: f64) -> String {
    let s = seconds as u64;
    if s >= 60 { format!("{}m{:02}s", s / 60, s % 60) } else { format!("{s}s") }
}

pub fn badge(state: State, color: bool) -> String {
    let (label, code) = badge_parts(state);
    paint(&format!(" {label} "), code, color)
}

/// A badge, the group in the default colour, then the subject in bold, what it has to say and, once passed, its time.
pub fn row(step: &Step, color: bool) -> String {
    let subject = if color { format!("\x1b[1m{}\x1b[22m", step.subject) } else { step.subject.clone() };
    let mut parts = vec![format!("{} {}", badge(step.state, color), step.group), subject, step.detail.clone()];
    if step.state == State::Passed {
        if let Some(took) = step.took {
            parts.push(duration(took));
        }
    }
    parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join("  ")
}

/// How many steps failed, passed and run.
pub fn counts(steps: &[Step]) -> (usize, usize, usize) {
    let n = |s: State| steps.iter().filter(|x| x.state == s).count();
    (n(State::Failed), n(State::Passed), n(State::Running))
}

pub fn summary(steps: &[Step], color: bool) -> String {
    let (failed, passed, running) = counts(steps);
    let mut parts = Vec::new();
    for (n, word, code) in [(failed, "failed", "31"), (passed, "passed", "32"), (running, "running", "33")] {
        if n > 0 {
            parts.push(paint(&format!("{n} {word}"), code, color));
        }
    }
    parts.push(format!("{} total", steps.len()));
    format!("Steps: {}", parts.join(", "))
}

pub fn blink_on(elapsed: f64) -> bool {
    (elapsed / BLINK_SECONDS) as u64 % 2 == 0
}

/// 40 columns of coloured blanks: failed, passed, running (grey while it blinks off), then grey.
pub fn bar(steps: &[Step], blinking_on: bool) -> String {
    let (failed, passed, running) = counts(steps);
    let total = steps.len();
    let width = |n: usize| n * BAR_WIDTH / total;
    let mut parts = vec![("41", width(failed)), ("42", width(passed)), ("43", width(running))];
    if !blinking_on {
        parts[2].0 = GREY;
    }
    let used: usize = parts.iter().map(|p| p.1).sum();
    parts.push((GREY, BAR_WIDTH - used));
    parts.into_iter().filter(|p| p.1 > 0).map(|(code, w)| format!("\x1b[{code}m{}\x1b[0m", " ".repeat(w))).collect()
}

/// The live lines: a row per running step, the summary, the time and, while something runs, the bar. `after_pass`
/// is true when the last finished step passed: a blank line then separates the log from the rows.
pub fn frame(steps: &[Step], elapsed: f64, blinking_on: bool, color: bool, after_pass: bool) -> Vec<String> {
    let running: Vec<String> = steps.iter().filter(|s| s.state == State::Running).map(|s| row(s, color)).collect();
    let mut lines = Vec::new();
    if after_pass {
        lines.push(String::new());
    }
    let any = !running.is_empty();
    lines.extend(running);
    if any {
        lines.push(String::new());
    }
    lines.push(summary(steps, color));
    lines.push(format!("Time:  {}s", elapsed as u64));
    if any && color {
        lines.push(bar(steps, blinking_on));
    }
    lines
}

pub fn failure_lines(step: &Step) -> Vec<String> {
    step.lines.iter().map(|l| format!(" {BULLET} {l}")).collect()
}

/// What is printed once and kept when a step ends: its row, and for a failure its lines and a blank line.
pub fn finished_block(step: &Step, color: bool) -> Vec<String> {
    let mut block = vec![row(step, color)];
    if step.state == State::Failed {
        block.extend(failure_lines(step));
        block.push(String::new());
    }
    block
}

/// A terminal draws in place; not a terminal, `NO_COLOR` set (an empty one counts as unset) or `TERM=dumb` gets lines.
pub fn is_fancy(isatty: bool, no_color: Option<&str>, term: Option<&str>) -> bool {
    isatty && no_color.is_none_or(str::is_empty) && term != Some("dumb")
}

pub fn plain_line(step: &Step) -> String {
    let label = badge_parts(step.state).0;
    let took = match (step.state, step.took) {
        (State::Passed, Some(took)) => duration(took),
        _ => String::new(),
    };
    [label, &step.group, &step.subject, &step.detail, &took].iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join(" ")
}

pub fn plain_lines(step: &Step) -> Vec<String> {
    let mut lines = vec![plain_line(step)];
    if step.state == State::Failed {
        lines.extend(failure_lines(step));
    }
    lines
}

/// Says when a progress line is due: each 10% done, or each 30 seconds.
pub struct Throttle {
    decile: u32,
    last: f64,
}

impl Throttle {
    pub fn new(now: f64) -> Throttle {
        Throttle { decile: 0, last: now }
    }

    pub fn due(&mut self, fraction: Option<f64>, now: f64) -> bool {
        let decile = fraction.map_or(self.decile, |f| (f * 10.0) as u32);
        if decile > self.decile || now - self.last >= PLAIN_EVERY_SECONDS {
            self.decile = decile;
            self.last = now;
            return true;
        }
        false
    }
}

/// Splits like Python's `str.splitlines()` on `\n`, `\r\n` and a bare `\r` only (not on every separator Python knows:
/// those are what tools write; osmium's and Planetiler's progress updates are separated by `\r`).
pub fn splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' | b'\r' => {
                lines.push(&text[start..i]);
                if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// The last `limit` lines of `text` that are not empty.
pub fn tail(text: &str, limit: usize) -> Vec<String> {
    let lines: Vec<String> = splitlines(text).into_iter().filter(|l| !l.trim().is_empty()).map(|l| l.trim_end().to_string()).collect();
    lines[lines.len().saturating_sub(limit)..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ansi(text: &str) -> String {
        let mut out = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' && chars.peek() == Some(&'[') {
                for d in chars.by_ref() {
                    if d == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn plain(lines: Vec<String>) -> Vec<String> {
        lines.iter().map(|l| strip_ansi(l)).collect()
    }

    /// One step per state, named s0, s1, ..., each in the state given.
    fn steps(states: &[State]) -> Vec<Step> {
        states
            .iter()
            .enumerate()
            .map(|(i, &state)| {
                let mut s = Step::new(&format!("s{i}"), "group", &format!("subject {i}"));
                s.state = state;
                s.took = (state == State::Passed).then_some(3.0);
                s
            })
            .collect()
    }

    /// The widths of the coloured runs of a bar line, as {background code: columns}.
    fn bar_widths(line: &str) -> std::collections::BTreeMap<String, usize> {
        let mut widths = std::collections::BTreeMap::new();
        let mut rest = line;
        while let Some(at) = rest.find("\x1b[") {
            rest = &rest[at + 2..];
            let m = rest.find('m').unwrap();
            let code = rest[..m].to_string();
            rest = &rest[m + 1..];
            let blanks = rest.len() - rest.trim_start_matches(' ').len();
            if blanks > 0 {
                *widths.entry(code).or_insert(0) += blanks;
            }
        }
        widths
    }

    fn widths(pairs: &[(&str, usize)]) -> std::collections::BTreeMap<String, usize> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    use State::{Failed, Passed, Pending, Running};

    #[test]
    fn badge_has_a_space_each_side_black_on_a_colour_and_is_not_bold() {
        assert_eq!(badge(Running, true), "\x1b[30;43m RUNS \x1b[0m");
        assert_eq!(badge(Passed, true), "\x1b[30;42m PASS \x1b[0m");
        assert_eq!(badge(Failed, true), "\x1b[30;41m FAIL \x1b[0m");
        assert_eq!(badge(Passed, false), " PASS ");
    }

    #[test]
    fn row_is_group_in_the_default_colour_then_subject_in_bold() {
        let mut step = Step::new("extract", "extract", "quebec-latest.osm.pbf");
        step.state = Passed;
        step.detail = "1,213 MB".into();
        step.took = Some(42.0);
        assert_eq!(row(&step, true), "\x1b[30;42m PASS \x1b[0m extract  \x1b[1mquebec-latest.osm.pbf\x1b[22m  1,213 MB  42s");
    }

    #[test]
    fn running_row_shows_progress_but_no_time_and_failed_row_none_either() {
        let mut running = Step::new("tiles", "tiles", "cutting 1,734 tiles");
        running.state = Running;
        running.detail = "batch 9 of 14".into();
        running.took = Some(5.0);
        let mut failed = Step::new("basemap", "basemap", "planetiler");
        failed.state = Failed;
        failed.took = Some(5.0);
        assert_eq!(plain(vec![row(&running, true)]), [" RUNS  tiles  cutting 1,734 tiles  batch 9 of 14"]);
        assert_eq!(plain(vec![row(&failed, true)]), [" FAIL  basemap  planetiler"]);
    }

    #[test]
    fn durations() {
        assert_eq!([0.4, 42.9, 60.0, 192.0].map(duration), ["0s", "42s", "1m00s", "3m12s"]);
    }

    #[test]
    fn only_the_counts_that_are_not_zero_in_order_failed_passed_running_total() {
        let line = summary(&steps(&[Passed, Failed, Running, Pending, Passed, Pending]), true);
        assert_eq!(line, "Steps: \x1b[31m1 failed\x1b[0m, \x1b[32m2 passed\x1b[0m, \x1b[33m1 running\x1b[0m, 6 total");
    }

    #[test]
    fn zero_counts_are_left_out() {
        assert_eq!(summary(&steps(&[Passed, Passed, Pending]), false), "Steps: 2 passed, 3 total");
        assert_eq!(summary(&steps(&[Pending, Pending]), false), "Steps: 2 total");
    }

    #[test]
    fn bar_is_forty_columns_red_green_yellow_then_grey() {
        let line = bar(&steps(&[Failed, Passed, Passed, Running, Pending, Pending, Pending, Pending]), true);
        assert_eq!(bar_widths(&line), widths(&[("41", 5), ("42", 10), ("43", 5), (GREY, 20)]));
        assert_eq!(strip_ansi(&line).len(), BAR_WIDTH);
        let at = |code: &str| line.find(&format!("{code}m")).unwrap();
        assert!(at("41") < at("42"));
        assert!(at("42") < at("43"));
        assert!(at("43") < at(GREY));
    }

    #[test]
    fn each_part_rounds_down() {
        // 9 steps: 1 * 40 / 9 = 4 and 2 * 40 / 9 = 8; the rest is grey
        let mut states = vec![Failed, Passed, Passed, Running];
        states.extend([Pending; 5]);
        let line = bar(&steps(&states), true);
        assert_eq!(bar_widths(&line), widths(&[("41", 4), ("42", 8), ("43", 4), (GREY, 24)]));
    }

    #[test]
    fn yellow_blinks_to_grey() {
        let base = steps(&[Passed, Running, Pending, Pending]);
        assert_eq!(bar_widths(&bar(&base, true)), widths(&[("42", 10), ("43", 10), (GREY, 20)]));
        assert_eq!(bar_widths(&bar(&base, false)), widths(&[("42", 10), (GREY, 30)]));
    }

    #[test]
    fn blink_phase_changes_every_500_ms() {
        assert_eq!([0.0, 0.49, 0.5, 0.99, 1.0].map(blink_on), [true, true, false, false, true]);
    }

    #[test]
    fn running_only() {
        let mut made = steps(&[Running, Pending]);
        made[0].subject = "cutting 1,734 tiles".into();
        made[0].detail = "batch 9 of 14".into();
        assert_eq!(frame(&made, 7.9, true, false, false), [" RUNS  group  cutting 1,734 tiles  batch 9 of 14", "", "Steps: 1 running, 2 total", "Time:  7s"]);
    }

    #[test]
    fn frame_has_the_bar_as_third_summary_line_while_something_runs() {
        let lines = frame(&steps(&[Running, Pending]), 1.0, true, true, false);
        assert_eq!(lines.len(), 5);
        assert_eq!(strip_ansi(lines.last().unwrap()).len(), BAR_WIDTH);
    }

    #[test]
    fn blank_line_between_the_log_and_the_running_rows_after_a_pass_only() {
        let made = steps(&[Passed, Running]);
        assert_eq!(frame(&made, 1.0, true, false, true)[0], "");
        assert_ne!(frame(&made, 1.0, true, false, false)[0], "");
    }

    #[test]
    fn two_running_rows_then_one_blank_line() {
        let lines = plain(frame(&steps(&[Running, Running, Pending]), 1.0, true, true, false));
        assert_eq!(lines[2], "");
        assert!(lines[0].starts_with(" RUNS") && lines[1].starts_with(" RUNS"));
        assert!(lines[3].starts_with("Steps:"));
    }

    #[test]
    fn nothing_running_has_no_bar_and_no_rows() {
        let lines = frame(&steps(&[Passed, Failed, Pending]), 97.0, true, true, false);
        assert_eq!(plain(lines), ["Steps: 1 failed, 1 passed, 3 total", "Time:  97s"]);
    }

    #[test]
    fn finished_block_of_a_passed_step_is_its_row_only() {
        let step = steps(&[Passed]).remove(0);
        assert_eq!(finished_block(&step, false), [" PASS  group  subject 0  3s"]);
    }

    #[test]
    fn finished_block_of_a_failure_has_its_lines_with_bullets_then_a_blank_line() {
        let mut step = steps(&[Failed]).remove(0);
        step.lines = vec!["java: Planetiler needs Java 21, found 17".into(), "full log: data/logs/basemap.log".into()];
        assert_eq!(
            finished_block(&step, false),
            [" FAIL  group  subject 0", " ‣ java: Planetiler needs Java 21, found 17", " ‣ full log: data/logs/basemap.log", ""]
        );
    }

    #[test]
    fn plain_lines_have_no_colour_and_name_the_state() {
        let mut step = Step::new("tiles", "tiles", "cutting 1,734 tiles");
        step.state = Running;
        assert_eq!(plain_line(&step), "RUNS tiles cutting 1,734 tiles");
        step.detail = "batch 9 of 14".into();
        assert_eq!(plain_line(&step), "RUNS tiles cutting 1,734 tiles batch 9 of 14");
        step.state = Passed;
        step.subject = "1,700 tiles".into();
        step.detail = String::new();
        step.took = Some(192.0);
        assert_eq!(plain_line(&step), "PASS tiles 1,700 tiles 3m12s");
    }

    #[test]
    fn failure_lines_follow_a_failed_line() {
        let mut step = Step::new("basemap", "basemap", "planetiler");
        step.state = Failed;
        step.lines = vec!["boom".into()];
        assert_eq!(plain_lines(&step), ["FAIL basemap planetiler", " ‣ boom"]);
    }

    #[test]
    fn progress_lines_come_at_most_every_ten_percent_or_thirty_seconds() {
        let mut due = Throttle::new(0.0);
        assert!(!due.due(Some(0.05), 1.0));
        assert!(due.due(Some(0.10), 2.0));
        assert!(!due.due(Some(0.15), 3.0));
        assert!(!due.due(Some(0.19), 4.0));
        assert!(due.due(Some(0.20), 5.0));
        assert!(!due.due(Some(0.20), 6.0));
        assert!(due.due(Some(0.21), 40.0)); // 30 s since the last line
        assert!(!due.due(Some(0.22), 41.0));
    }

    #[test]
    fn without_a_fraction_only_the_thirty_seconds_count() {
        let mut due = Throttle::new(0.0);
        assert!(!due.due(None, 29.0));
        assert!(due.due(None, 30.0));
    }

    #[test]
    fn a_terminal_needs_a_tty_no_no_color_and_no_dumb_term() {
        assert!(is_fancy(true, None, Some("xterm-256color")));
        assert!(is_fancy(true, None, None));
        assert!(!is_fancy(false, None, Some("xterm")));
        assert!(!is_fancy(true, Some("1"), None));
        assert!(!is_fancy(true, None, Some("dumb")));
        assert!(is_fancy(true, Some(""), None));
    }

    #[test]
    fn tail_is_the_last_fifteen_non_empty_lines() {
        let text = (0..40).map(|i| if i % 3 == 0 { format!("line {i}\n") } else { format!("line {i}") }).collect::<Vec<_>>().join("\n") + "\n\n";
        let t = tail(&text, TAIL_LINES);
        assert_eq!(t.len(), 15);
        assert_eq!(t.last().unwrap(), "line 39");
        assert!(!t.contains(&String::new()));
    }

    #[test]
    fn tail_of_a_short_log_is_all_of_it() {
        assert_eq!(tail("a\n\n  \nb  \n", TAIL_LINES), ["a", "b"]);
        assert!(tail("", TAIL_LINES).is_empty());
    }

    #[test]
    fn a_bare_carriage_return_ends_a_line_as_it_does_for_python() {
        assert_eq!(tail("10%\r20%\r30%\r\ndone\n", TAIL_LINES), ["10%", "20%", "30%", "done"]);
        assert_eq!(splitlines("a\r\nb\rc\n\nd"), ["a", "b", "c", "", "d"]);
        assert_eq!(splitlines("a\n"), ["a"]);
        assert!(splitlines("").is_empty());
    }
}
