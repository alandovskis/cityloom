//! The design review report as data: the cover and the summary, worded in the interface language. Pure: the
//! view-model gathers the `Facts` from the city and the network checks, and `build` words them. The writer of the
//! PDF is another module (`cityloom_report_pdf`) behind the `PdfWriter` port.

use cityloom_report::{Block, Captioned, ReportData};

use crate::city::model::{City, CityView, street_note};
use crate::shared::catalogue::{Mode, class_key, mode_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::say_now;
use crate::shared::units::Units;

/// What the report says about the city, counted by the view-model from the city view and the cached checks.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    pub area: String,
    pub streets: usize,
    pub junctions: usize,
    /// Streets and junctions together; none means there is nothing to report.
    pub places: usize,
    pub edited: usize,
    pub failing: usize,
    pub problems: usize,
    pub notices: usize,
    /// The pages of the changed places (`place_pages`), after the summary.
    pub place_pages: Vec<Vec<Block>>,
    /// How many changed places `place_pages` left out, said at the end of the summary.
    pub left_out: usize,
}

/// The most changed places that get a page: a report stays a size a person reads.
pub const MAX_PLACES: usize = 40;
/// The most failing checks listed on a page; the rest are counted.
pub const MAX_FAILING_ROWS: usize = 8;
/// The box each figure is fitted into, in points (two sit side by side in the text width).
const FIGURE_W: f32 = 246.0;
const FIGURE_H: f32 = 190.0;

/// Words the report. `when_ms` is the time of the export, in milliseconds since 1970.
pub fn build(facts: &Facts, version: &str, when_ms: u64, i18n: &I18n) -> ReportData {
    let say = |key: &str| i18n.tr_now(key, &Args::new());
    let cover = vec![
        Block::Heading { level: 1, text: facts.area.clone() },
        Block::Paragraph { text: say("map-report-cover-title") },
        Block::Paragraph { text: i18n.tr_now("map-report-exported", &Args::new().str("when", utc(when_ms, i18n))) },
        Block::Paragraph { text: i18n.tr_now("map-report-version", &Args::new().str("version", version)) },
        Block::Paragraph { text: say("map-report-scope") },
        Block::Paragraph { text: say("map-report-study-notice") },
        Block::Paragraph { text: say("map-report-placeholder-notice") },
    ];
    ReportData {
        lang: i18n.locale_now().tag().to_string(),
        title: i18n.tr_now("map-report-title", &Args::new().str("area", facts.area.clone())),
        pages: [vec![cover, summary(facts, i18n)], facts.place_pages.clone()].concat(),
    }
}

fn summary(facts: &Facts, i18n: &I18n) -> Vec<Block> {
    let say = |key: &str| i18n.tr_now(key, &Args::new());
    let mut page = vec![Block::Heading { level: 2, text: say("map-report-summary") }];
    if facts.places == 0 {
        page.push(Block::Paragraph { text: say("map-report-nothing") });
        return page;
    }
    let (pass, fail, notice) = (say("map-report-pass"), say("map-report-fail"), say("map-report-notice"));
    let verdict = |n: usize, otherwise: &String| if n == 0 { pass.clone() } else { otherwise.clone() };
    let n = |v: usize| v.to_string();
    let city = Args::new().num("streets", facts.streets as i64).num("junctions", facts.junctions as i64);
    let row = |label: &str, figure: String, result: String| vec![say(label), figure, result];
    page.push(Block::Table {
        caption: say("map-report-summary-caption"),
        head: vec![say("map-report-col-measure"), say("map-report-col-figure"), say("map-report-col-result")],
        rows: vec![
            row("map-report-row-city", i18n.tr_now("map-report-figure-city", &city), String::new()),
            row("map-report-row-edited", n(facts.edited), String::new()),
            row("map-report-row-failing", n(facts.failing), verdict(facts.failing, &fail)),
            row("map-report-row-problems", n(facts.problems), verdict(facts.problems, &fail)),
            row("map-report-row-notices", n(facts.notices), verdict(facts.notices, &notice)),
        ],
    });
    if facts.problems == 0 && facts.notices == 0 {
        page.push(Block::Paragraph { text: i18n.tr_now("map-report-none-found", &Args::new().str("result", pass)) });
    }
    if facts.left_out > 0 {
        let args = Args::new().num("shown", MAX_PLACES as i64).num("left", facts.left_out as i64);
        page.push(Block::Paragraph { text: i18n.tr_now("map-report-place-left-out", &args) });
    }
    page
}

/// One page for each changed place, in the order of the Changes list (junctions, then streets), at most `MAX_PLACES`.
/// Returns the pages and how many changed places there were beyond them. Only the changed places' editors are read.
pub fn place_pages(city: &City, view: &CityView, region: usize, i18n: &I18n, units: Units) -> (Vec<Vec<Block>>, usize) {
    place_pages_up_to(MAX_PLACES, city, view, region, i18n, units)
}

/// `place_pages` with the cap given, so a test need not make 41 changes.
pub(crate) fn place_pages_up_to(max: usize, city: &City, view: &CityView, region: usize, i18n: &I18n, units: Units) -> (Vec<Vec<Block>>, usize) {
    let mut junctions: Vec<_> = view.nodes.iter().filter(|n| n.junction && n.edited).collect();
    junctions.sort_by_key(|n| n.number);
    let streets = view.edges.iter().filter(|e| e.edited);
    let mut pages = Vec::new();
    let total = junctions.len() + streets.clone().count();
    for n in junctions.into_iter().take(max) {
        pages.push(junction_page(city, n.uid, &say_now(i18n, units, &n.name), &n.failing, region, i18n, units));
    }
    for e in streets.take(max.saturating_sub(pages.len())) {
        let name = say_now(i18n, units, &street_note(e.called.clone(), e.ends.clone()));
        pages.push(street_page(city, e.uid, e.zones, name, region, i18n, units));
    }
    (pages, total.saturating_sub(max))
}

fn street_page(city: &City, uid: u32, zones: usize, name: String, region: usize, i18n: &I18n, units: Units) -> Vec<Block> {
    let tr = |key: &str, args: &Args| i18n.tr_now(key, args);
    let mut page = vec![Block::Heading { level: 2, text: name }];
    let (Some(before), Some(after)) = (city.street_editor_today(uid, region), city.street_editor(uid, region)) else {
        page.push(Block::Paragraph { text: tr("map-report-place-street-missing", &Args::new()) });
        return page;
    };
    let (before, after) = (before.view(), after.view());
    let locale = i18n.locale_now();
    if zones > 1 {
        page.push(Block::Paragraph { text: tr("map-report-place-zones", &Args::new().num("zones", zones as i64)) });
    }
    let length = |mm: i32| units.length_fine_in(mm, locale);
    let mut lines = Vec::new();
    let widths = Args::new().str("before", length(before.total_mm)).str("after", length(after.total_mm));
    if before.total_mm != after.total_mm {
        lines.push(tr("map-report-place-width", &widths));
    }
    if before.class != after.class {
        let class = |c| tr(class_key(c), &Args::new());
        lines.push(tr("map-report-place-class", &Args::new().str("before", class(before.class)).str("after", class(after.class))));
    }
    if before.speed_kmh != after.speed_kmh {
        let speed = |s: Option<u16>| match s {
            Some(k) => tr("st-speed-kmh", &Args::new().num("kmh", i64::from(k))),
            None => tr("st-speed-not-set", &Args::new()),
        };
        lines.push(tr("map-report-place-speed", &Args::new().str("before", speed(before.speed_kmh)).str("after", speed(after.speed_kmh))));
    }
    let added = after.segments.iter().filter(|a| !before.segments.iter().any(|b| b.uid == a.uid)).count();
    let removed = before.segments.iter().filter(|b| !after.segments.iter().any(|a| a.uid == b.uid)).count();
    if added + removed > 0 {
        lines.push(tr("map-report-place-pieces", &Args::new().num("added", added as i64).num("removed", removed as i64)));
    }
    if lines.is_empty() {
        lines.push(tr("map-report-place-other", &Args::new()));
    }
    page.extend(lines.into_iter().map(|text| Block::Paragraph { text }));
    let items = [
        captioned(
            "map-report-place-before",
            crate::street::drawing::alt(&before, i18n),
            crate::street::drawing::figure(&before, i18n, FIGURE_W, FIGURE_H),
            i18n,
        ),
        captioned("map-report-place-after", crate::street::drawing::alt(&after, i18n), crate::street::drawing::figure(&after, i18n, FIGURE_W, FIGURE_H), i18n),
    ];
    page.push(Block::Figures { items: items.to_vec() });
    let share = |shares: &[crate::street::model::Share], mode: Mode| {
        let s = shares.iter().find(|s| s.mode == mode);
        tr("map-report-place-share", &Args::new().str("length", length(s.map_or(0, |s| s.mm))).num("pct", i64::from(s.map_or(0, |s| s.pct))))
    };
    page.push(Block::Table {
        caption: tr("map-report-place-width-caption", &Args::new()),
        head: vec![tr("map-report-place-col-mode", &Args::new()), tr("map-report-place-before", &Args::new()), tr("map-report-place-after", &Args::new())],
        rows: Mode::ALL.iter().map(|&m| vec![tr(mode_key(m), &Args::new()), share(&before.outcomes.share, m), share(&after.outcomes.share, m)]).collect(),
    });
    let checks = after.checks.iter().map(|c| (c.ok, &c.label, &c.detail)).collect::<Vec<_>>();
    page.extend(checks_block(&checks, i18n, units));
    page
}

fn junction_page(city: &City, uid: u32, name: &str, failing: &[crate::shared::said::Said], region: usize, i18n: &I18n, units: Units) -> Vec<Block> {
    let tr = |key: &str, args: &Args| i18n.tr_now(key, args);
    let mut page = vec![Block::Heading { level: 2, text: name.to_string() }];
    let (Some(before), Some(after)) = (city.junction_editor_today(uid, region), city.junction_editor(uid, region)) else {
        page.push(Block::Paragraph { text: tr("map-report-place-junction-undrawn", &Args::new()) });
        if !failing.is_empty() {
            let list = failing.iter().map(|s| say_now(i18n, units, s)).collect::<Vec<_>>().join("; ");
            page.push(Block::Paragraph { text: tr("map-report-place-junction-failing", &Args::new().str("failing", list)) });
        }
        return page;
    };
    let (before, after) = (before.view(), after.view());
    let control = |v: &crate::junction::read_model::JView| tr(crate::junction::model::control_key(v.control_index), &Args::new());
    let key = if before.control_index == after.control_index { "map-report-place-control-same" } else { "map-report-place-control" };
    page.push(Block::Paragraph { text: tr(key, &Args::new().str("before", control(&before)).str("after", control(&after))) });
    let items = [
        captioned(
            "map-report-place-before",
            crate::junction::figure::alt(&before, i18n),
            crate::junction::figure::figure(&before, i18n, units, FIGURE_W, FIGURE_H),
            i18n,
        ),
        captioned(
            "map-report-place-after",
            crate::junction::figure::alt(&after, i18n),
            crate::junction::figure::figure(&after, i18n, units, FIGURE_W, FIGURE_H),
            i18n,
        ),
    ];
    page.push(Block::Figures { items: items.to_vec() });
    let checks = after.checks.iter().map(|c| (c.ok, &c.label, &c.detail)).collect::<Vec<_>>();
    page.extend(checks_block(&checks, i18n, units));
    page
}

/// A figure with its caption (Before or After, in words), its text alternative and, when shapes were left out of it, the
/// note that says so.
fn captioned(caption: &str, what: String, figure: cityloom_report::Figure, i18n: &I18n) -> Captioned {
    let tr = |key: &str, args: &Args| i18n.tr_now(key, args);
    let caption = tr(caption, &Args::new());
    let note = if figure.omitted > 0 { tr("map-report-place-omitted", &Args::new()) } else { String::new() };
    let alt = tr("map-report-place-alt", &Args::new().str("which", caption.clone()).str("what", what));
    Captioned { caption, alt, note, figure }
}

/// The checks of a place: a table of the failing ones (at most `MAX_FAILING_ROWS`, then a count of the rest), or one
/// sentence that all pass.
pub(crate) fn checks_block(checks: &[(bool, &crate::shared::said::Said, &crate::shared::said::Said)], i18n: &I18n, units: Units) -> Vec<Block> {
    let tr = |key: &str, args: &Args| i18n.tr_now(key, args);
    let failing: Vec<_> = checks.iter().filter(|c| !c.0).collect();
    if failing.is_empty() {
        let args = Args::new().num("n", checks.len() as i64).str("result", tr("map-report-pass", &Args::new()));
        return vec![Block::Paragraph { text: tr("map-report-place-all-pass", &args) }];
    }
    let fail = tr("map-report-fail", &Args::new());
    let mut rows: Vec<Vec<String>> =
        failing.iter().take(MAX_FAILING_ROWS).map(|c| vec![say_now(i18n, units, c.1), fail.clone(), say_now(i18n, units, c.2)]).collect();
    if failing.len() > MAX_FAILING_ROWS {
        let more = tr("map-report-place-more", &Args::new().num("n", (failing.len() - MAX_FAILING_ROWS) as i64));
        rows.push(vec![more, String::new(), String::new()]);
    }
    vec![Block::Table {
        caption: tr("map-report-place-checks-caption", &Args::new()),
        head: vec![tr("map-report-place-col-check", &Args::new()), tr("map-report-col-result", &Args::new()), tr("map-report-place-col-detail", &Args::new())],
        rows,
    }]
}

/// A time in milliseconds since 1970 as UTC (the civil-from-days algorithm, so no date crate): the ISO date and the
/// zero-padded hours and minutes are neutral, and `map-report-when` lays them out in the page's language.
fn utc(ms: u64, i18n: &I18n) -> String {
    let secs = ms / 1000;
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let (hours, minutes) = (rest / 3600, rest % 3600 / 60);
    let args = Args::new().str("date", format!("{year:04}-{month:02}-{day:02}")).str("hours", format!("{hours:02}")).str("minutes", format!("{minutes:02}"));
    i18n.tr_now("map-report-when", &args)
}

/// The area's name as a file-name part: lower-case letters and digits, one hyphen between runs, `area` if none.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if let Some(base) = unaccent(c) {
            out.push(base);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-');
    let out: String = out.chars().take(60).collect();
    let out = out.trim_end_matches('-');
    if out.is_empty() { "area".to_string() } else { out.to_string() }
}

fn unaccent(c: char) -> Option<char> {
    Some(match c {
        'à' | 'â' | 'ä' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'î' | 'ï' => 'i',
        'ô' | 'ö' => 'o',
        'ù' | 'û' | 'ü' => 'u',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;

    #[test]
    fn the_time_is_written_in_utc() {
        assert_eq!(utc(0, &crate::i18n_for(Locale::En)), "1970-01-01 00:00");
        assert_eq!(utc(1_793_966_400_000, &crate::i18n_for(Locale::En)), "2026-11-06 12:00");
        assert_eq!(utc(951_782_400_000 + 86_399_000, &crate::i18n_for(Locale::En)), "2000-02-29 23:59");
        assert_eq!(utc(1_793_966_400_000, &crate::i18n_for(Locale::FrCa)), "2026-11-06 12\u{a0}h\u{a0}00");
    }

    #[test]
    fn the_slug_is_a_file_name_part() {
        assert_eq!(slug("Plateau Mont-Royal, Montréal"), "plateau-mont-royal-montreal");
        assert_eq!(slug("???"), "area");
        assert!(slug(&"ab ".repeat(100)).len() <= 60);
    }
}
