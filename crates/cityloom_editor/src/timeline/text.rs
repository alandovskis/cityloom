//! The words of the timeline that are made from data: when a milestone was made, what it holds, how a place differs
//! and why a change was refused.

use serde_json::Value;

use crate::shared::i18n::{Args, I18n};
use crate::sync::model::Kind;
use crate::timeline::model::{Milestone, Refused, Summary};
use crate::timeline::work::{Aspect, How};

/// The parts of a sync stamp (`2026-11-12T15:04:00.000Z`, UTC): year, month, day, hour, minute. None for what is not
/// a time, and for the epoch, which only says the time is not known.
fn parts(stamp: &str) -> Option<(String, i64, i64, String)> {
    if stamp == crate::sync::model::stamp(0) {
        return None;
    }
    let (date, time) = stamp.get(..16)?.split_once('T')?;
    let mut d = date.split('-');
    let (year, month, day) = (d.next()?, d.next()?.parse::<i64>().ok()?, d.next()?.parse::<i64>().ok()?);
    let known = year.len() == 4 && year.bytes().all(|b| b.is_ascii_digit()) && (1..=12).contains(&month) && (1..=31).contains(&day);
    let clock = time.len() == 5 && time.as_bytes()[2] == b':';
    (known && clock).then(|| (year.to_string(), month, day, time.to_string()))
}

/// When a milestone was made, in the page's language (tracked); empty where the time is not known.
pub fn when_text(i18n: &I18n, stamp: &str) -> String {
    let Some((year, month, day, time)) = parts(stamp) else { return String::new() };
    let date = i18n.tr("timeline-date", &Args::new().str("year", year).num("month", month).num("day", day));
    i18n.tr("timeline-when", &Args::new().str("date", date).str("time", time))
}

/// What a milestone holds, as a compact line (tracked).
pub fn summary_text(i18n: &I18n, summary: &Summary) -> String {
    if summary.is_empty() {
        return i18n.tr("timeline-summary-empty", &Args::new());
    }
    i18n.tr(
        "timeline-summary",
        &Args::new().num("streets", summary.streets as i64).num("junctions", summary.junctions as i64).num("rules", summary.rules as i64),
    )
}

/// The message saying why a change was refused.
pub fn refused_key(refused: Refused) -> &'static str {
    match refused {
        Refused::Empty => "timeline-refused-empty",
        Refused::TooLong => "timeline-refused-long",
        Refused::TooLarge => "timeline-refused-large",
        Refused::Full => "timeline-refused-full",
        Refused::Unknown => "timeline-gone",
        Refused::Unusable => "timeline-refused-unusable",
    }
}

pub fn how_key(how: How) -> &'static str {
    match how {
        How::Added => "timeline-how-added",
        How::Removed => "timeline-how-removed",
        How::Changed => "timeline-how-changed",
    }
}

pub fn aspect_key(aspect: Aspect) -> &'static str {
    match aspect {
        Aspect::Name => "timeline-aspect-name",
        Aspect::Class => "timeline-aspect-class",
        Aspect::Width => "timeline-aspect-width",
        Aspect::Lanes => "timeline-aspect-lanes",
        Aspect::Speed => "timeline-aspect-speed",
        Aspect::Zones => "timeline-aspect-zones",
        Aspect::Arms => "timeline-aspect-arms",
        Aspect::Control => "timeline-aspect-control",
        Aspect::Rule => "timeline-aspect-rule",
        Aspect::Outline => "timeline-aspect-outline",
        Aspect::Other => "timeline-aspect-other",
    }
}

fn text_at<'a>(record: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter().try_fold(record, |v, k| v.get(*k))?.as_str().map(str::trim).filter(|s| !s.is_empty())
}

/// What a place is called, from its record alone (the city is not loaded here): a street by its name, a junction by
/// the smallest OpenStreetMap node it was made from, a rule by its name (tracked).
pub fn place_label(i18n: &I18n, kind: Kind, key: &str, record: Option<&Value>) -> String {
    let none = Value::Null;
    let record = record.unwrap_or(&none);
    match kind {
        Kind::Street => match text_at(record, &["street", "name"]) {
            Some(name) => i18n.tr("timeline-street", &Args::new().str("name", name)),
            None => i18n.tr("timeline-street-unnamed", &Args::new()),
        },
        Kind::Junction => match record.get("nodes").and_then(Value::as_array).and_then(|n| n.iter().filter_map(Value::as_i64).min()) {
            Some(node) => i18n.tr("timeline-junction", &Args::new().str("node", node.to_string())),
            None => i18n.tr("timeline-junction-unknown", &Args::new()),
        },
        Kind::Rule => match text_at(record, &["name"]) {
            Some(name) => i18n.tr("timeline-rule", &Args::new().str("name", name)),
            None => i18n.tr("timeline-rule-unnamed", &Args::new()),
        },
        _ => key.to_string(),
    }
}

/// The author line of a milestone, empty where nobody was signed in (tracked).
pub fn author_text(i18n: &I18n, milestone: &Milestone) -> String {
    milestone.author.as_deref().map(|a| i18n.tr("timeline-by", &Args::new().str("author", a))).unwrap_or_default()
}
