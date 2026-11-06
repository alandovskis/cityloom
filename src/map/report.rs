//! The design review report as data: the cover and the summary, worded in the interface language. Pure: the
//! view-model gathers the `Facts` from the city and the network checks, and `build` words them. The writer of the
//! PDF is another module (`cityloom_report`) behind the `PdfWriter` port.

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
        Block::Paragraph { text: i18n.tr_now("map-report-exported", &Args::new().str("when", utc(when_ms))) },
        Block::Paragraph { text: i18n.tr_now("map-report-version", &Args::new().str("version", version)) },
        Block::Paragraph { text: say("map-report-scope") },
        Block::Paragraph { text: say("map-report-study-notice") },
        Block::Paragraph { text: say("map-report-placeholder-notice") },
    ];
    ReportData {
        lang: i18n.locale_now().tag().to_string(),
        title: i18n.tr_now("map-report-title", &Args::new().str("area", facts.area.clone())),
        pages: vec![cover, summary(facts, i18n)],
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
    page
}

/// `YYYY-MM-DD HH:MM` (UTC) for a time in milliseconds since 1970 (the civil-from-days algorithm, so no date crate).
fn utc(ms: u64) -> String {
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
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}", rest / 3600, rest % 3600 / 60)
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

    #[test]
    fn the_time_is_written_in_utc() {
        assert_eq!(utc(0), "1970-01-01 00:00");
        assert_eq!(utc(1_793_966_400_000), "2026-11-06 12:00");
        assert_eq!(utc(951_782_400_000 + 86_399_000), "2000-02-29 23:59");
    }

    #[test]
    fn the_slug_is_a_file_name_part() {
        assert_eq!(slug("Plateau Mont-Royal, Montréal"), "plateau-mont-royal-montreal");
        assert_eq!(slug("???"), "area");
        assert!(slug(&"ab ".repeat(100)).len() <= 60);
    }
}
