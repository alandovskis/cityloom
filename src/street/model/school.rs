//! What the School street template does besides laying out its pieces (#148): it makes a school zone, a stretch
//! of the street with a lower speed limit and a raised table, from what the editor already has. Nothing here is
//! new geometry: the zone is an ordinary zone (`split_zone`), its speed an ordinary zone speed (`set_speed`), the
//! table an ordinary raised element (`add_calming`, with every one of its checks), each inspectable and editable
//! afterwards. A part that does not fit is skipped, with the reason said, and the others still apply.
//!
//! Every figure is a placeholder, UNVERIFIED: the speed and the length of a school zone are CityLoom's own.

use super::calming::{DEFAULT_TABLE_MM, Kind};
use super::zones::MIN_ZONE_MM;
use super::{Arg, Editor, Said, Segment, snap};

/// The speed limit of a school zone, in kilometres an hour.
pub const SCHOOL_KMH: u16 = 30;

/// How long the school zone is when the zone it is made in is long enough to set it apart, in millimetres.
pub const SCHOOL_ZONE_MM: i32 = 60_000;

/// What applying a template did, part by part, as `Said` sentences: what was applied, and what was skipped and why.
/// Empty for a template that has only its pieces.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TemplateReport {
    pub applied: Vec<Said>,
    pub skipped: Vec<Said>,
}

impl Editor {
    /// Lays `arranged` out in the school zone as one revision labelled `label`, and reports what was done.
    ///
    /// The school zone is a stretch of `SCHOOL_ZONE_MM` in the middle of the selected zone when that zone has room
    /// for it and for `MIN_ZONE_MM` either side; else the selected zone as it is (the whole street when it has one
    /// zone, or when no zone can be made). It gets the pieces, a speed limit of `SCHOOL_KMH` unless it is lower
    /// already, and a raised table in its middle.
    pub(super) fn apply_school(&mut self, arranged: Vec<Segment>, next_uid: u32, label: Said) -> TemplateReport {
        let start = self.cursor;
        let mut report = TemplateReport::default();

        // The stretch.
        match self.structure_frame(false) {
            Ok(frame) => {
                let z = self.zone;
                let (from, to) = self.states[self.cursor].content.span(z, frame.length_mm);
                let a = snap(from + (to - from - SCHOOL_ZONE_MM) / 2);
                let b = a + SCHOOL_ZONE_MM;
                if a - from >= MIN_ZONE_MM && to - b >= MIN_ZONE_MM && self.split_zone(Some(a)).is_ok() && self.split_zone(Some(b)).is_ok() {
                    self.select_zone(z + 1);
                    report.applied.push(Said::new("template-school-applied-zone").with("from", Arg::Length(a)).with("to", Arg::Length(b)));
                } else {
                    let room = Said::new("template-school-room").with("length", Arg::Length(SCHOOL_ZONE_MM)).with("min", Arg::Length(MIN_ZONE_MM));
                    report.skipped.push(Said::new("template-school-skipped-zone").with("reason", Arg::Said(Box::new(room))));
                }
            }
            Err(refusal) => report.skipped.push(Said::new("template-school-skipped-zone").with("reason", Arg::Said(Box::new(refusal.said())))),
        }

        // The pieces, in the school zone.
        self.edit(label.clone(), |segs| {
            *segs = arranged;
            true
        });

        // The speed limit of the school zone.
        let z = self.zone;
        let content = &self.states[self.cursor].content;
        let current = if content.zones.len() > 1 { super::zone_speed::speed_in(content.speed, &content.zones, z) } else { content.speed };
        match current.kmh().filter(|&n| n <= SCHOOL_KMH) {
            Some(n) => report
                .skipped
                .push(Said::new("template-school-skipped-speed").with("kmh", Arg::Num(i64::from(n))).with("limit", Arg::Num(i64::from(SCHOOL_KMH)))),
            None => {
                self.set_speed(Some(SCHOOL_KMH));
                report.applied.push(Said::new("template-school-applied-speed").with("kmh", Arg::Num(i64::from(SCHOOL_KMH))));
            }
        }

        // A raised table in the middle of it, by the table's own rules.
        match self.school_table() {
            Ok(at) => report.applied.push(Said::new("template-school-applied-table").with("at", Arg::Length(at))),
            Err(reason) => report.skipped.push(Said::new("template-school-skipped-table").with("reason", Arg::Said(Box::new(reason)))),
        }

        // One step to undo, whatever it took.
        self.next_uid = self.next_uid.max(next_uid);
        self.selected = None;
        self.squash_since(start, label);
        report
    }

    /// Adds a raised table in the middle of the selected zone, as `add_calming` would, and says where it is.
    fn school_table(&mut self) -> Result<i32, Said> {
        let frame = self.frame.ok_or(super::calming::CalmingRefusal::NoAnchor.said())?;
        let (from, to) = self.states[self.cursor].content.span(self.zone, frame.length_mm);
        if to - from < DEFAULT_TABLE_MM {
            return Err(Said::new("template-school-table-room").with("length", Arg::Length(DEFAULT_TABLE_MM)));
        }
        let at = snap(from + (to - from - DEFAULT_TABLE_MM) / 2);
        self.add_calming(Kind::Table, at, DEFAULT_TABLE_MM).map(|_| at).map_err(|r| r.said())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::model::{Side, Speed, Street, Template, ZoneEnd, ZoneFrame};

    fn editor(length_mm: i32, speed: Speed) -> Editor {
        let mut s = Street::sample(0, Side::Right);
        s.speed = speed;
        Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm, from_end: ZoneEnd::First }))
    }

    fn keys(said: &[Said]) -> Vec<&'static str> {
        said.iter().map(|s| s.key).collect()
    }

    #[test]
    fn a_long_street_gets_a_school_zone_in_its_middle_with_a_lower_speed_and_a_table() {
        let mut e = editor(100_000, Speed::Kmh(50));
        let report = e.apply_template(Template::School).unwrap();
        assert_eq!(keys(&report.applied), ["template-school-applied-zone", "template-school-applied-speed", "template-school-applied-table"]);
        assert!(report.skipped.is_empty(), "{:?}", report.skipped);
        let s = e.snapshot();
        assert_eq!(s.zones.iter().map(|z| z.from_mm).collect::<Vec<_>>(), [0, 20_000, 80_000]);
        assert_eq!(s.zones.iter().map(|z| z.speed).collect::<Vec<_>>(), [Speed::Undecided, Speed::Kmh(30), Speed::Undecided]);
        assert_eq!(s.speed, Speed::Kmh(50), "the street keeps its own");
        assert!(s.zones[0].segments.is_none() && s.zones[2].segments.is_none());
        assert!(s.zones[1].segments.is_some());
        assert_eq!(s.calming.len(), 1);
        assert_eq!((s.calming[0].from_mm, s.calming[0].length_mm), (46_500, DEFAULT_TABLE_MM));
        assert_eq!(e.zone(), 1, "the school zone is the selected one");
    }

    #[test]
    fn it_is_one_revision_that_undo_takes_back_whole() {
        let mut e = editor(100_000, Speed::Kmh(50));
        let before = e.snapshot();
        e.apply_template(Template::School).unwrap();
        let v = e.view();
        assert_eq!(v.revisions.len(), 1);
        assert_eq!(v.revisions[0].label.key, "rev-template");
        // The uid counter is the street's one and is never wound back: what undo restores is the street.
        let same = |e: &Editor| {
            let s = e.snapshot();
            (s.segments, s.zones, s.speed, s.calming, s.length_mm)
        };
        let first = (before.segments, before.zones, before.speed, before.calming, before.length_mm);
        assert!(e.undo());
        assert_eq!(same(&e), first);
        assert!(e.redo());
        assert_eq!(e.snapshot().zones.len(), 3);
        assert!(e.reset());
        assert_eq!(same(&e), first);
    }

    #[test]
    fn what_it_made_is_ordinary_and_editable() {
        let mut e = editor(100_000, Speed::Kmh(50));
        e.apply_template(Template::School).unwrap();
        assert!(e.set_speed(Some(40)), "the zone's speed can be changed");
        assert_eq!(e.snapshot().zones[1].speed, Speed::Kmh(40));
        assert!(e.remove_calming(0).is_ok());
        assert!(e.snapshot().calming.is_empty());
    }

    #[test]
    fn a_street_too_short_to_set_a_zone_apart_makes_the_whole_street_the_school_zone() {
        let mut e = editor(60_000, Speed::Kmh(50));
        let report = e.apply_template(Template::School).unwrap();
        assert_eq!(keys(&report.skipped), ["template-school-skipped-zone"]);
        assert_eq!(keys(&report.applied), ["template-school-applied-speed", "template-school-applied-table"]);
        let s = e.snapshot();
        assert!(s.zones.is_empty());
        assert_eq!(s.speed, Speed::Kmh(30), "one zone: the street's speed");
        assert_eq!(s.calming.len(), 1);
    }

    #[test]
    fn a_street_with_no_anchored_end_skips_the_zone_and_the_table_but_still_applies_the_rest() {
        let mut e = Editor::new(0);
        let report = e.apply_template(Template::School).unwrap();
        assert_eq!(keys(&report.skipped), ["template-school-skipped-zone", "template-school-skipped-table"]);
        assert_eq!(keys(&report.applied), ["template-school-applied-speed"]);
        assert_eq!(e.view().revisions.len(), 1);
        assert_eq!(e.view().speed_kmh, Some(30));
        assert_eq!(e.view().segments.len(), 5);
    }

    #[test]
    fn a_speed_already_at_or_under_the_school_limit_is_left_alone() {
        for kmh in [20, 30] {
            let mut e = editor(100_000, Speed::Kmh(kmh));
            let report = e.apply_template(Template::School).unwrap();
            assert_eq!(keys(&report.skipped), ["template-school-skipped-speed"]);
            let s = e.snapshot();
            assert!(s.zones.iter().all(|z| z.speed.is_undecided()), "{kmh}");
            assert_eq!(s.speed, Speed::Kmh(kmh));
        }
    }

    #[test]
    fn a_street_with_no_speed_is_given_the_school_speed_in_its_zone() {
        let mut e = editor(100_000, Speed::NotSet);
        e.apply_template(Template::School).unwrap();
        assert_eq!(e.snapshot().zones[1].speed, Speed::Kmh(30));
    }

    #[test]
    fn a_zone_too_short_for_a_table_says_so_and_keeps_the_rest() {
        let mut e = editor(6_000, Speed::Kmh(50));
        let report = e.apply_template(Template::School).unwrap();
        assert_eq!(keys(&report.skipped), ["template-school-skipped-zone", "template-school-skipped-table"]);
        assert_eq!(e.snapshot().speed, Speed::Kmh(30));
        assert!(e.snapshot().calming.is_empty());
    }

    #[test]
    fn a_table_the_street_refuses_is_skipped_with_its_own_reason() {
        let mut e = editor(100_000, Speed::Kmh(50));
        e.add_calming(Kind::Table, 40_000, 14_000).unwrap();
        let report = e.apply_template(Template::School).unwrap();
        assert_eq!(keys(&report.skipped), ["template-school-skipped-table"]);
        assert_eq!(report.skipped[0].args.len(), 1);
        assert_eq!(e.snapshot().calming.len(), 1);
    }

    #[test]
    fn the_school_zone_is_made_in_the_selected_zone_only() {
        let mut e = editor(200_000, Speed::Kmh(50));
        e.split_zone(Some(100_000)).unwrap();
        assert_eq!(e.zone(), 1);
        e.apply_template(Template::School).unwrap();
        let s = e.snapshot();
        assert_eq!(s.zones.iter().map(|z| z.from_mm).collect::<Vec<_>>(), [0, 100_000, 120_000, 180_000]);
        assert_eq!(s.zones[2].speed, Speed::Kmh(30));
        assert_eq!((s.zones[0].speed, s.zones[1].speed, s.zones[3].speed), (Speed::Undecided, Speed::Undecided, Speed::Undecided));
    }

    #[test]
    fn the_other_templates_have_no_report() {
        let mut e = editor(100_000, Speed::Kmh(50));
        assert_eq!(e.apply_template(Template::Greenway), Ok(TemplateReport::default()));
        assert!(e.snapshot().zones.is_empty() && e.snapshot().calming.is_empty());
    }
}
