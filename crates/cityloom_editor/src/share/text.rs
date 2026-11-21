//! The words of sharing that are made from data: when a link stops working.

use crate::shared::i18n::{Args, I18n};

/// The day of a sync stamp (`2027-04-15T08:00:00.000Z`, UTC): year, month, day. None for what is not one.
fn day(stamp: &str) -> Option<(String, i64, i64)> {
    let date = stamp.get(..10)?;
    let mut d = date.split('-');
    let (year, month, day) = (d.next()?, d.next()?.parse::<i64>().ok()?, d.next()?.parse::<i64>().ok()?);
    let known = year.len() == 4 && year.bytes().all(|b| b.is_ascii_digit()) && (1..=12).contains(&month) && (1..=31).contains(&day);
    known.then(|| (year.to_string(), month, day))
}

/// The day `stamp` falls on, in the page's language (tracked): `April 15, 2027` / `15 avril 2027`; empty for what is
/// not a time.
pub fn date_text(i18n: &I18n, stamp: &str) -> String {
    let Some((year, month, day)) = day(stamp) else { return String::new() };
    i18n.tr("share-date", &Args::new().str("year", year).num("month", month).num("day", day))
}

/// Until when a link works, or since when it no longer does, at `now` (a sync stamp: the two compare as text).
pub fn expiry_text(i18n: &I18n, expires_at: &str, now: &str) -> String {
    let key = if expires_at > now { "share-until" } else { "share-expired-on" };
    i18n.tr(key, &Args::new().str("date", date_text(i18n, expires_at)))
}
