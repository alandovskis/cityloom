//! The words of the list of projects that are made from data: when a project changed, and why a change was refused.

use crate::project::model::Refused;
use crate::shared::i18n::{Args, I18n};

/// The year, month and day of a sync stamp (`2026-11-12T15:00:00.000Z`, UTC); None for one that is not a time, or
/// for the epoch, which the first project's record is made at and which says only that the time is not known.
fn date_of(stamp: &str) -> Option<(String, i64, i64)> {
    if stamp == crate::sync::model::stamp(0) {
        return None;
    }
    let date = stamp.get(..10)?;
    let mut parts = date.split('-');
    let (year, month, day) = (parts.next()?, parts.next()?.parse::<i64>().ok()?, parts.next()?.parse::<i64>().ok()?);
    let known = year.len() == 4 && year.bytes().all(|b| b.is_ascii_digit()) && (1..=12).contains(&month) && (1..=31).contains(&day);
    known.then(|| (year.to_string(), month, day))
}

/// When a project last changed, as a date in the page's language (tracked); empty where that is not known.
pub fn changed_text(i18n: &I18n, stamp: &str) -> String {
    let Some((year, month, day)) = date_of(stamp) else { return String::new() };
    let when = i18n.tr("project-date", &Args::new().str("year", year).num("month", month).num("day", day));
    i18n.tr("project-changed", &Args::new().str("when", when))
}

/// The message saying why a name or a change was not taken.
pub fn refused_key(refused: Refused) -> &'static str {
    match refused {
        Refused::Empty => "project-refused-empty",
        Refused::TooLong => "project-refused-long",
        Refused::Unknown => "project-gone",
        Refused::Unusable => "project-refused-unusable",
    }
}
