//! The design review report as data: the cover and the summary, worded in the interface language. Pure: the
//! view-model gathers the `Facts` from the city and the network checks, and `build` words them. The writer of the
//! PDF is another module (`cityloom_report_pdf`) behind the `PdfWriter` port.

use cityloom_report::{Block, ReportData};

use crate::shared::i18n::{Args, I18n};

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
}

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
        pages: [vec![cover, summary(facts, i18n)], sources(i18n)].concat(),
    }
}

/// The lines a page can hold before the writer cuts it, with room to spare: the writer draws 11 pt text with a
/// leading of 1.35 inside 54 pt margins (about 46 lines), and wraps about 90 characters a line.
const PAGE_LINES: usize = 36;
const LINE_CHARS: usize = 80;

/// What a block costs in lines of a page, rounded up and with its gap.
fn lines_of(block: &Block) -> usize {
    match block {
        Block::Heading { text, .. } => 2 + text.chars().count() / LINE_CHARS,
        Block::Paragraph { text } => 1 + text.chars().count().div_ceil(LINE_CHARS),
        Block::Table { .. } => PAGE_LINES,
    }
}

/// Lays groups of blocks (a rule's heading and paragraphs, kept together) on pages of at most `PAGE_LINES`, the first
/// page opened by `first` and the later ones by `later`, the last page closed by `last`. A group is never split; one
/// longer than a page gets a page of its own, and the writer cuts what does not fit, so groups are kept short.
fn paginate(first: Vec<Block>, later: &dyn Fn() -> Block, groups: Vec<Vec<Block>>, last: Block) -> Vec<Vec<Block>> {
    let mut pages = vec![first];
    let mut used: usize = pages[0].iter().map(lines_of).sum();
    let mut on_page = 0;
    for group in groups {
        let cost: usize = group.iter().map(lines_of).sum();
        if used + cost > PAGE_LINES && on_page > 0 {
            let head = later();
            used = lines_of(&head);
            on_page = 0;
            pages.push(vec![head]);
        }
        used += cost;
        on_page += 1;
        let last = pages.len() - 1;
        pages[last].extend(group);
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
    page
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
}
