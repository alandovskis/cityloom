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
        pages: [vec![cover, summary(facts, i18n)], facts.place_pages.clone(), sources(i18n)].concat(),
    }
}

/// The lines the Sources pages put on a page, with room to spare, so that a rule's heading and its paragraphs stay on
/// one page and each page is read the same (the writer would carry an overflow onto the next page itself, but could
/// leave a rule's heading alone at the foot): it draws 11 pt text with a leading of 1.35 inside 54 pt margins (about
/// 46 lines) and wraps about 90 characters a line.
const PAGE_LINES: usize = 36;
const LINE_CHARS: usize = 80;

/// What a block costs in lines of a page, rounded up and with its gap.
fn lines_of(block: &Block) -> usize {
    match block {
        Block::Heading { text, .. } => 2 + text.chars().count() / LINE_CHARS,
        Block::Paragraph { text } => 1 + text.chars().count().div_ceil(LINE_CHARS),
        Block::Table { .. } | Block::Figures { .. } => PAGE_LINES,
    }
}

/// Cuts a group taller than a page into parts that each fit one, at paragraph boundaries (a single paragraph is not cut: it
/// is a status line or one source, a few lines); every part after the first
/// repeats the group's heading, so a rule's sources are never left on a page that does not say whose they are.
fn split_group(group: Vec<Block>) -> Vec<Vec<Block>> {
    let cost = |blocks: &[Block]| blocks.iter().map(lines_of).sum::<usize>();
    if cost(&group) <= PAGE_LINES {
        return vec![group];
    }
    // The room a later page keeps for its continuation heading comes off the budget.
    let room = PAGE_LINES.saturating_sub(lines_of(&Block::Heading { level: 2, text: String::new() }));
    let heading = group.first().filter(|b| matches!(b, Block::Heading { .. })).cloned();
    let mut parts: Vec<Vec<Block>> = vec![];
    for block in group {
        let fits = parts.last().is_some_and(|part| cost(part) + lines_of(&block) <= room);
        if fits || parts.last().is_some_and(|part| part.len() == 1 && matches!(part[0], Block::Heading { .. })) {
            if let Some(part) = parts.last_mut() {
                part.push(block);
            }
        } else if parts.is_empty() {
            parts.push(vec![block]);
        } else {
            let mut part = heading.clone().into_iter().collect::<Vec<_>>();
            part.push(block);
            parts.push(part);
        }
    }
    parts
}

/// Lays groups of blocks (a rule's heading and paragraphs, kept together where they fit) on pages of at most
/// `PAGE_LINES`, the first page opened by `first` and the later ones by `later`, the last page closed by `last`. A
/// group longer than a page is cut by `split_group`. The estimate leaves slack, and
/// `the_real_sources_pages_fit_their_budget_in_both_languages` holds the registry to it.
fn paginate(first: Vec<Block>, later: &dyn Fn() -> Block, groups: Vec<Vec<Block>>, last: Block) -> Vec<Vec<Block>> {
    let mut pages = vec![first];
    let mut used: usize = pages[0].iter().map(lines_of).sum();
    let mut on_page = 0;
    for group in groups.into_iter().flat_map(split_group) {
        let cost: usize = group.iter().map(lines_of).sum();
        if used + cost > PAGE_LINES && on_page > 0 {
            let head = later();
            used = lines_of(&head);
            on_page = 0;
            pages.push(vec![head]);
        }
        used += cost;
        on_page += 1;
        let at = pages.len() - 1;
        pages[at].extend(group);
    }
    if used + lines_of(&last) > PAGE_LINES {
        pages.push(vec![later()]);
    }
    let end = pages.len() - 1;
    pages[end].push(last);
    pages
}

/// The sources of the rules that name one, then the notice that the others are placeholders: a heading and the
/// words of each rule, on as many pages as they need. Read from the registry the pages read (`note_now`), so the
/// report cannot cite what a page does not.
fn sources(i18n: &I18n) -> Vec<Vec<Block>> {
    let say = |key: &str| i18n.tr_now(key, &Args::new());
    let units = crate::shared::units::Units::Metres;
    let groups = crate::shared::sources::cited()
        .map(|(rule, label)| {
            let n = crate::shared::sources::note_now(rule, i18n, units);
            let mut group = vec![
                Block::Heading { level: 3, text: say(label) },
                Block::Paragraph { text: i18n.tr_now("map-report-source-line", &Args::new().str("status", n.summary).str("applies", n.applies)) },
            ];
            group.extend(n.sources.into_iter().map(|s| Block::Paragraph {
                text: match s.url {
                    Some(url) => format!("{} {url}", s.text),
                    None => s.text,
                },
            }));
            group
        })
        .collect();
    let first = vec![Block::Heading { level: 2, text: say("map-report-sources") }, Block::Paragraph { text: say("map-report-sources-intro") }];
    paginate(
        first,
        &|| Block::Heading { level: 2, text: say("map-report-sources-continued") },
        groups,
        Block::Paragraph { text: say("map-report-sources-placeholders") },
    )
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
    let (figure_before, figure_after) = crate::street::drawing::figures(&before, &after, i18n, FIGURE_W, FIGURE_H);
    let items = [
        captioned("map-report-place-before", crate::street::drawing::alt(&before, i18n, units), figure_before, i18n),
        captioned("map-report-place-after", crate::street::drawing::alt(&after, i18n, units), figure_after, i18n),
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
    fn read(c: &crate::street::model::Check) -> (&str, bool, &crate::shared::said::Said, &crate::shared::said::Said) {
        (c.id, c.ok, &c.label, &c.detail)
    }
    page.extend(checks_block(&pair_checks(&before.checks, &after.checks, read), i18n, units));
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
    let (figure_before, figure_after) = crate::junction::figure::figures(&before, &after, i18n, units, FIGURE_W, FIGURE_H);
    let items = [
        captioned("map-report-place-before", crate::junction::figure::alt(&before, i18n), figure_before, i18n),
        captioned("map-report-place-after", crate::junction::figure::alt(&after, i18n), figure_after, i18n),
    ];
    page.push(Block::Figures { items: items.to_vec() });
    fn read(c: &crate::junction::read_model::Check) -> (&str, bool, &crate::shared::said::Said, &crate::shared::said::Said) {
        (c.id, c.ok, &c.label, &c.detail)
    }
    page.extend(checks_block(&pair_checks(&before.checks, &after.checks, read), i18n, units));
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

/// One check of a place as it was imported and as it is now; `None` where the check does not exist in that state.
pub(crate) struct CheckPair<'a> {
    pub id: &'a str,
    pub before: Option<bool>,
    pub after: Option<bool>,
    pub label: &'a crate::shared::said::Said,
    pub detail: &'a crate::shared::said::Said,
}

/// Pairs the checks of the imported state with those of the current one by id: the current ones first, then those only the
/// imported state has.
pub(crate) fn pair_checks<'a, T>(
    before: &'a [T],
    after: &'a [T],
    read: impl Fn(&'a T) -> (&'a str, bool, &'a crate::shared::said::Said, &'a crate::shared::said::Said),
) -> Vec<CheckPair<'a>> {
    let mut pairs: Vec<CheckPair> = after
        .iter()
        .map(|c| {
            let (id, ok, label, detail) = read(c);
            let was = before.iter().map(&read).find(|b| b.0 == id).map(|b| b.1);
            CheckPair { id, before: was, after: Some(ok), label, detail }
        })
        .collect();
    for c in before {
        let (id, ok, label, detail) = read(c);
        if !pairs.iter().any(|p| p.id == id) {
            pairs.push(CheckPair { id, before: Some(ok), after: None, label, detail });
        }
    }
    pairs
}

/// The checks of a place, imported state beside the current one: a table of those that fail in either (the current
/// failures first, at most `MAX_FAILING_ROWS`, then a count of the rest), or one sentence that all pass. A short sentence
/// first says what Before is.
pub(crate) fn checks_block(checks: &[CheckPair], i18n: &I18n, units: Units) -> Vec<Block> {
    let tr = |key: &str, args: &Args| i18n.tr_now(key, args);
    let mut blocks = vec![Block::Paragraph { text: tr("map-report-place-checks-intro", &Args::new()) }];
    let fails = |c: &&CheckPair| c.after == Some(false) || c.before == Some(false);
    let mut failing: Vec<&CheckPair> = checks.iter().filter(fails).collect();
    failing.sort_by_key(|c| c.after != Some(false));
    if failing.is_empty() {
        let args = Args::new().num("n", checks.iter().filter(|c| c.after.is_some()).count() as i64).str("result", tr("map-report-pass", &Args::new()));
        blocks.push(Block::Paragraph { text: tr("map-report-place-all-pass", &args) });
        return blocks;
    }
    let word = |state: Option<bool>| match state {
        Some(true) => tr("map-report-pass", &Args::new()),
        Some(false) => tr("map-report-fail", &Args::new()),
        None => tr("map-report-place-na", &Args::new()),
    };
    let mut rows: Vec<Vec<String>> = failing
        .iter()
        .take(MAX_FAILING_ROWS)
        .map(|c| {
            let detail = if c.after == Some(false) { say_now(i18n, units, c.detail) } else { String::new() };
            vec![say_now(i18n, units, c.label), word(c.before), word(c.after), detail]
        })
        .collect();
    if failing.len() > MAX_FAILING_ROWS {
        let more = tr("map-report-place-more", &Args::new().num("n", (failing.len() - MAX_FAILING_ROWS) as i64));
        rows.push(vec![more, String::new(), String::new(), String::new()]);
    }
    blocks.push(Block::Table {
        caption: tr("map-report-place-checks-caption", &Args::new()),
        head: vec![
            tr("map-report-place-col-check", &Args::new()),
            tr("map-report-place-before", &Args::new()),
            tr("map-report-place-after", &Args::new()),
            tr("map-report-place-col-detail", &Args::new()),
        ],
        rows,
    });
    blocks
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

    #[test]
    fn many_cited_rules_are_spread_over_pages_none_over_full_and_none_lost() {
        let rule = |i: usize| {
            vec![
                Block::Heading { level: 3, text: format!("Rule {i}") },
                Block::Paragraph { text: "word ".repeat(60) },
                Block::Paragraph { text: "source ".repeat(40) },
            ]
        };
        let pages = paginate(
            vec![Block::Heading { level: 2, text: "Sources".into() }, Block::Paragraph { text: "intro".into() }],
            &|| Block::Heading { level: 2, text: "Sources (continued)".into() },
            (0..120).map(rule).collect(),
            Block::Paragraph { text: "closing".into() },
        );
        assert!(pages.len() > 10);
        for page in &pages {
            assert!(page.iter().map(lines_of).sum::<usize>() <= PAGE_LINES, "{page:?}");
        }
        let headings: Vec<_> = pages.iter().flatten().filter_map(|b| if let Block::Heading { text, .. } = b { Some(text.clone()) } else { None }).collect();
        assert!((0..120).all(|i| headings.contains(&format!("Rule {i}"))));
        assert!(matches!(pages.last().unwrap().last(), Some(Block::Paragraph { text }) if text == "closing"));
        assert!(matches!(&pages[1][0], Block::Heading { level: 2, text } if text == "Sources (continued)"));
    }

    #[test]
    fn the_real_sources_pages_fit_their_budget_in_both_languages_and_name_every_cited_rule() {
        for locale in [Locale::En, Locale::FrCa] {
            let i18n = crate::i18n_for(locale);
            let pages = sources(&i18n);
            for page in &pages {
                assert!(page.iter().map(lines_of).sum::<usize>() <= PAGE_LINES, "a Sources page is over the budget in {locale:?}: {page:?}");
            }
            let headings: Vec<_> =
                pages.iter().flatten().filter_map(|b| if let Block::Heading { level: 3, text } = b { Some(text.clone()) } else { None }).collect();
            let distinct: std::collections::BTreeSet<_> = headings.iter().collect();
            assert_eq!(distinct.len(), crate::shared::sources::cited().count(), "a cited rule has no heading, or two share one");
        }
    }

    #[test]
    fn a_rule_longer_than_a_page_is_cut_between_paragraphs_and_keeps_its_heading_and_every_source() {
        let long = vec![
            Block::Heading { level: 3, text: "Long rule".into() },
            Block::Paragraph { text: "status ".repeat(30) },
            Block::Paragraph { text: "source-a ".repeat(100) },
            Block::Paragraph { text: "source-b ".repeat(100) },
            Block::Paragraph { text: "source-c ".repeat(100) },
        ];
        let pages = paginate(
            vec![Block::Heading { level: 2, text: "Sources".into() }],
            &|| Block::Heading { level: 2, text: "Sources (continued)".into() },
            vec![long],
            Block::Paragraph { text: "closing".into() },
        );
        for page in &pages {
            assert!(page.iter().map(lines_of).sum::<usize>() <= PAGE_LINES, "{page:?}");
        }
        let all: Vec<&Block> = pages.iter().flatten().collect();
        assert!(pages.len() >= 2, "the rule should have been cut over pages");
        let paragraphs = |word: &str| all.iter().filter(|b| matches!(b, Block::Paragraph { text } if text.contains(word))).count();
        assert!(["status", "source-a", "source-b", "source-c"].iter().all(|w| paragraphs(w) == 1), "a paragraph was lost or repeated");
        assert!(all.iter().filter(|b| matches!(b, Block::Heading { level: 3, text } if text == "Long rule")).count() >= 2, "the heading is not repeated");
        for word in ["source-a", "source-b", "source-c", "status"] {
            assert!(all.iter().any(|b| matches!(b, Block::Paragraph { text } if text.contains(word))), "{word} was lost");
        }
        for page in &pages[1..] {
            let first_rule = page.iter().find(|b| matches!(b, Block::Heading { level: 3, .. }));
            let has_text = page.iter().any(|b| matches!(b, Block::Paragraph { text } if text.contains("source-")));
            assert!(!has_text || first_rule.is_some(), "a page of sources does not say whose: {page:?}");
        }
    }
}
