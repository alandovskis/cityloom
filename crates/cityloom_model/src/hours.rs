use serde::{Deserialize, Serialize};

use crate::LaneKind;

/// How finely a time of day is kept, in minutes: a window starts on the quarter hour before its time and ends on the
/// one after.
const SLOT_MIN: i32 = 15;

/// A stretch of the day, on some days of the week.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hours {
    /// The days, as OSM opening hours write them before their times: `Mo-Fr`, `Sa,Su`; none for every day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days: Option<String>,
    /// Minutes after midnight, on a quarter hour.
    pub from_min: i32,
    /// Minutes after midnight, on a quarter hour; 0 where the window runs to the end of the day.
    pub to_min: i32,
}

/// Something a lane is for part of the day: during `hours` it is of this kind, whatever its base kind is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    pub hours: Hours,
    pub kind: LaneKind,
}

impl Hours {
    /// The stretches that OSM opening hours name: `Mo-Fr 06:00-10:00,14:30-19:00` is two, both on `Mo-Fr`, in order.
    /// Times are made quarter hours (see `SLOT_MIN`). None for hours that have several rules, say `off`, run past
    /// midnight, overlap, or are not times at all.
    pub fn parse(text: &str) -> Option<Vec<Hours>> {
        let time = |t: &str| {
            let (h, m) = t.split_once(':')?;
            let (h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
            (h < 24 && m < 60 || (h, m) == (24, 0)).then_some(h * 60 + m)
        };
        if text.contains(';') || text.split_whitespace().any(|t| t.eq_ignore_ascii_case("off")) {
            return None;
        }
        let days: Vec<&str> = text.split_whitespace().filter(|t| !t.contains(':')).collect();
        let days = (!days.is_empty()).then(|| days.join(" "));
        let mut spans = Vec::new();
        for range in text.split(|c: char| c.is_whitespace() || c == ',').filter(|r| r.contains(':')) {
            let (from, to) = range.split_once('-')?;
            let (from, to) = (time(from)? / SLOT_MIN * SLOT_MIN, (time(to)? + SLOT_MIN - 1) / SLOT_MIN * SLOT_MIN);
            if to <= from {
                return None;
            }
            spans.push((from, to));
        }
        spans.sort();
        if spans.is_empty() || spans.windows(2).any(|w| w[1].0 < w[0].1) {
            return None;
        }
        Some(spans.into_iter().map(|(from_min, to)| Hours { days: days.clone(), from_min, to_min: to % (24 * 60) }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(text: &str) -> Option<Vec<(i32, i32)>> {
        Hours::parse(text).map(|h| h.into_iter().map(|h| (h.from_min, h.to_min)).collect())
    }

    #[test]
    fn hours_are_read_as_quarter_hours_and_a_day_ends_at_midnight() {
        assert_eq!(spans("07:05-09:50"), Some(vec![(420, 600)]));
        assert_eq!(spans("Mo-Fr 16:00-24:00"), Some(vec![(960, 0)]));
        assert_eq!(spans("Mo-Fr 10:00-12:00,06:00-08:00"), Some(vec![(360, 480), (600, 720)]));
    }

    #[test]
    fn the_days_go_with_each_stretch() {
        let days = |text| Hours::parse(text).unwrap().into_iter().map(|h| h.days).collect::<Vec<_>>();
        assert_eq!(days("Mo-Fr 06:00-10:00,14:30-19:00"), [Some("Mo-Fr".to_string()), Some("Mo-Fr".to_string())]);
        assert_eq!(days("Sa,Su 08:00-12:00"), [Some("Sa,Su".to_string())]);
        assert_eq!(days("06:00-10:00"), [None]);
    }

    #[test]
    fn hours_that_overlap_or_run_past_midnight_or_say_off_are_not_read() {
        for text in
            ["06:00-10:00,09:00-12:00", "22:00-02:00", "Mo-Fr off", "10:00-10:00", "", "Mo-Fr", "sunrise-sunset", "Sa,Su 07:00-09:00; Mo-Fr 06:00-10:00"]
        {
            assert_eq!(Hours::parse(text), None, "{text:?}");
        }
    }
}
