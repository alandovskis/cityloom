//! What the School street template does besides laying out its pieces (#148): it makes a school zone, a stretch
//! of the street with a lower speed limit and a raised table, from what the editor already has. Nothing here is
//! new geometry: the stretch is an ordinary pick (`pick_stretch`) laid out as any edit over the pick is
//! (`edit_uniform`, which splits at its ends), its speed an ordinary speed of the pick (`set_speed`), the table an
//! ordinary raised element (`add_calming`, with every one of its checks), each inspectable and editable afterwards.
//! A part that does not fit is skipped, with the reason said, and the others still apply.
//!
//! Every figure is a placeholder, UNVERIFIED: the speed and the length of a school zone are CityLoom's own.

use super::CalmingKind as Kind;
use super::calming::DEFAULT_TABLE_MM;
use super::zones::MIN_ZONE_MM;
use super::{Arg, Editor, Pick, Said, Segment, Speed, snap};
use cityloom_model::street::speed_in;

/// The speed limit of a school zone, in kilometres an hour.
pub const SCHOOL_KMH: u16 = 30;

/// How long the school zone is when the pick it is made in is long enough to set it apart, in millimetres.
pub const SCHOOL_ZONE_MM: i32 = 60_000;

/// What applying a template did, part by part, as `Said` sentences: what was applied, and what was skipped and why.
/// Empty for a template that has only its pieces.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TemplateReport {
    pub applied: Vec<Said>,
    pub skipped: Vec<Said>,
}

impl Editor {
    /// Lays `arranged` out in the school stretch as one revision labelled `label`, and reports what was done.
    ///
    /// The school stretch is the middle `SCHOOL_ZONE_MM` of the pick (the whole street when it is picked whole) when
    /// the pick has room for it and for `MIN_ZONE_MM` either side, and becomes the pick; else it is the pick itself.
    /// It is picked as any stretch is (`pick_stretch`), so an end that a boundary between runs lies under `MIN_ZONE_MM`
    /// from snaps to it, leaving no sliver: the stretch can then be from 50.2 m to 69.8 m long. It gets the pieces,
    /// with one set of uids, across any boundaries inside it, a speed limit of `SCHOOL_KMH` in each part of it above
    /// that or with none (a part at or under it keeps its own), and a raised table in its middle. The rest of the pick
    /// is left as it was.
    pub(super) fn apply_school(&mut self, arranged: Vec<Segment>, next_uid: u32, label: Said) -> TemplateReport {
        let start = self.cursor;
        let mut report = TemplateReport::default();

        // The stretch.
        match self.structure_frame(false) {
            Ok(frame) => {
                let (from, to) = self.pick_span(frame.length_mm);
                let a = snap(from + (to - from - SCHOOL_ZONE_MM) / 2);
                if to - from >= SCHOOL_ZONE_MM + 2 * MIN_ZONE_MM
                    && let Ok(Pick::Stretch { from_mm, to_mm }) = self.pick_stretch(a, a + SCHOOL_ZONE_MM)
                {
                    let (a, b) = (self.left_mm(from_mm), self.left_mm(to_mm));
                    report.applied.push(Said::new("template-school-applied-zone").with("from", Arg::Length(a.min(b))).with("to", Arg::Length(a.max(b))));
                } else {
                    let room = Said::new("template-school-room").with("length", Arg::Length(SCHOOL_ZONE_MM)).with("min", Arg::Length(MIN_ZONE_MM));
                    report.skipped.push(Said::new("template-school-skipped-zone").with("reason", Arg::Said(Box::new(room))));
                }
            }
            Err(refusal) => report.skipped.push(Said::new("template-school-skipped-zone").with("reason", Arg::Said(Box::new(refusal.said())))),
        }

        // The pieces, in the school stretch.
        self.edit_uniform(label.clone(), arranged);

        // The speed limit of the school stretch, part by part: a part at or under the school limit keeps its own.
        let content = &self.states[self.cursor].content;
        let above = |kmh: Option<u16>| kmh.is_none_or(|n| n > SCHOOL_KMH);
        let speeds: Vec<Option<u16>> = self.zones_in_pick().map(|z| speed_in(content.speed, &content.zones, z).kmh()).collect();
        if speeds.iter().all(|&k| above(k)) {
            self.set_speed(Some(SCHOOL_KMH));
            report.applied.push(Said::new("template-school-applied-speed").with("kmh", Arg::Num(i64::from(SCHOOL_KMH))));
        } else if speeds.iter().any(|&k| above(k)) {
            self.lower_to_school_speed(label.clone());
            report.applied.push(Said::new("template-school-applied-speed").with("kmh", Arg::Num(i64::from(SCHOOL_KMH))));
        } else {
            let highest = speeds.into_iter().flatten().max().unwrap_or(SCHOOL_KMH);
            report
                .skipped
                .push(Said::new("template-school-skipped-speed").with("kmh", Arg::Num(i64::from(highest))).with("limit", Arg::Num(i64::from(SCHOOL_KMH))));
        }

        // A raised table in the middle of it, by the table's own rules.
        match self.school_table() {
            Ok(at) => {
                // Where the table starts on the plan, from its left end, as every position the page shows.
                let at = self.left_mm(at).min(self.left_mm(at + DEFAULT_TABLE_MM));
                report.applied.push(Said::new("template-school-applied-table").with("at", Arg::Length(at)))
            }
            Err(reason) => report.skipped.push(Said::new("template-school-skipped-table").with("reason", Arg::Said(Box::new(reason)))),
        }

        // One step to undo, whatever it took.
        self.next_uid = self.next_uid.max(next_uid);
        self.selected = None;
        self.squash_since(start, label);
        report
    }

    /// Gives `SCHOOL_KMH` to each zone in the pick whose speed (its own, else the street's) is above it or not set,
    /// as one edit (`over_pick`); the zones at or under it keep theirs. Only for a pick of several zones.
    fn lower_to_school_speed(&mut self, label: Said) -> bool {
        self.over_pick(label, |c, range, _, _| {
            for z in range {
                if speed_in(c.speed, &c.zones, z).kmh().is_none_or(|n| n > SCHOOL_KMH) {
                    c.zones[z].speed = Speed::Kmh(SCHOOL_KMH);
                }
            }
            Vec::new()
        })
    }

    /// Adds a raised table in the middle of the pick (the school stretch), as `add_calming` would, and says where it is.
    fn school_table(&mut self) -> Result<i32, Said> {
        let frame = self.frame.ok_or(super::calming::CalmingRefusal::NoAnchor.said())?;
        let (from, to) = self.pick_span(frame.length_mm);
        if to - from < DEFAULT_TABLE_MM {
            return Err(Said::new("template-school-table-room").with("length", Arg::Length(DEFAULT_TABLE_MM)));
        }
        let at = snap(from + (to - from - DEFAULT_TABLE_MM) / 2);
        self.add_calming(Kind::Table, at, DEFAULT_TABLE_MM).map(|_| at).map_err(|r| r.said())
    }

    /// The pick's span in anchored millimetres along a street `length_mm` long: all of it when picked whole.
    fn pick_span(&self, length_mm: i32) -> (i32, i32) {
        match self.pick() {
            Pick::Whole => (0, length_mm),
            Pick::Stretch { from_mm, to_mm } => (from_mm, to_mm),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::model::StreetExt;
    use crate::street::model::{Side, Speed, Street, Template, ZoneEnd, ZoneFrame};

    fn editor(length_mm: i32, speed: Speed) -> Editor {
        let mut s = Street::sample(0, Side::Right);
        s.speed = speed;
        Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm, from_end: ZoneEnd::First }))
    }

    fn keys(said: &[Said]) -> Vec<&'static str> {
        said.iter().map(|s| s.key).collect()
    }

    fn starts(e: &Editor) -> Vec<i32> {
        e.snapshot().zones.iter().map(|z| z.from_mm).collect()
    }

    fn stretch(from_mm: i32, to_mm: i32) -> Pick {
        Pick::Stretch { from_mm, to_mm }
    }

    /// A 200 m street at 50 km/h in two runs: the base to 70 m, then a section of its own (a wider first piece).
    fn two_runs() -> Editor {
        let mut s = crate::street::model::zoned(Street::sample(0, Side::Right), &[0, 70_000], 200_000);
        s.speed = Speed::Kmh(50);
        let mut own = s.segments.clone();
        own[0].width_mm += 100;
        s.zones[1].segments = Some(own);
        Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 200_000, from_end: ZoneEnd::First }))
    }

    #[test]
    fn the_school_template_on_a_long_stretch_makes_its_60_m_stretch_in_the_middle_and_leaves_the_rest() {
        let mut e = two_runs();
        let was = e.snapshot();
        // A 100 m pick across the boundary at 70 m: the school stretch is 40 m to 100 m, across it.
        e.pick_stretch(20_000, 120_000).unwrap();
        let report = e.apply_template(Template::School).unwrap();
        assert_eq!(keys(&report.applied), ["template-school-applied-zone", "template-school-applied-speed", "template-school-applied-table"]);
        assert_eq!((report.applied[0].args[0].1.clone(), report.applied[0].args[1].1.clone()), (Arg::Length(40_000), Arg::Length(100_000)));
        assert_eq!(starts(&e), [0, 40_000, 100_000]);
        let s = e.snapshot();
        let sections = s.sections();
        assert_eq!((sections[0], sections[2]), (was.sections()[0], was.sections()[1]), "the rest of the pick is as it was");
        assert!(!cityloom_model::street::same_pieces(sections[1], was.sections()[0]), "the school stretch has the template's pieces");
        assert_eq!(s.zones.iter().map(|z| z.speed).collect::<Vec<_>>(), [Speed::Undecided, Speed::Kmh(30), Speed::Undecided]);
        assert_eq!(s.speed, Speed::Kmh(50));
        assert_eq!(s.calming.len(), 1);
        assert_eq!((s.calming[0].from_mm, s.calming[0].length_mm), (66_500, DEFAULT_TABLE_MM), "the table is in the stretch's middle");
        assert_eq!((e.pick(), e.zone()), (stretch(40_000, 100_000), 1), "the school stretch is picked");
    }

    #[test]
    fn the_school_template_on_a_short_stretch_makes_the_stretch_itself_the_school_zone() {
        let mut e = editor(200_000, Speed::Kmh(50));
        // 60 m: no 5 m to spare on each side of a 60 m school stretch.
        e.pick_stretch(50_000, 110_000).unwrap();
        let report = e.apply_template(Template::School).unwrap();
        assert_eq!(keys(&report.skipped), ["template-school-skipped-zone"]);
        assert_eq!(keys(&report.applied), ["template-school-applied-speed", "template-school-applied-table"]);
        assert_eq!(starts(&e), [0, 50_000, 110_000]);
        let s = e.snapshot();
        assert!(s.zones[0].segments.is_none() && s.zones[2].segments.is_none());
        assert!(s.zones[1].segments.is_some());
        assert_eq!(s.zones.iter().map(|z| z.speed).collect::<Vec<_>>(), [Speed::Undecided, Speed::Kmh(30), Speed::Undecided]);
        assert_eq!(s.calming[0].from_mm, 76_500);
        assert_eq!(e.pick(), stretch(50_000, 110_000));
    }

    #[test]
    fn a_whole_street_sidewalk_change_after_the_school_template_reaches_the_school_stretch() {
        let mut e = editor(100_000, Speed::Kmh(50));
        e.apply_template(Template::School).unwrap();
        assert_eq!(starts(&e), [0, 20_000, 80_000]);
        e.pick_whole();
        let sidewalk = crate::shared::catalogue::kind_index("sidewalk").unwrap();
        let first_sidewalk = |section: &[Segment]| section.iter().find(|g| g.kind == sidewalk).map(|g| (g.uid, g.width_mm));
        let (uid, _) = first_sidewalk(e.current()).unwrap();
        let school_uid = first_sidewalk(e.snapshot().sections()[1]).unwrap().0;
        assert_ne!(uid, school_uid, "the school stretch has fresh uids");
        assert!(e.set_width(uid, 3_000));
        let s = e.snapshot();
        assert_eq!(s.sections().iter().map(|g| first_sidewalk(g).unwrap().1).collect::<Vec<_>>(), [3_000, 3_000, 3_000]);
        assert!(e.missed().is_empty());
    }

    #[test]
    fn the_school_template_lowers_only_the_parts_of_its_stretch_above_the_school_limit() {
        // Two runs on a 200 m street at 50 km/h: 20 km/h to 70 m, the street's 50 km/h after it.
        let mut s = crate::street::model::zoned(Street::sample(0, Side::Right), &[0, 70_000], 200_000);
        s.speed = Speed::Kmh(50);
        s.zones[0].speed = Speed::Kmh(20);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 200_000, from_end: ZoneEnd::First }));
        let before = e.snapshot();
        // The school stretch, 40 m to 100 m, lies over both.
        e.pick_stretch(20_000, 120_000).unwrap();
        let report = e.apply_template(Template::School).unwrap();
        assert!(keys(&report.applied).contains(&"template-school-applied-speed"), "{:?}", report);
        assert_eq!(starts(&e), [0, 40_000, 70_000, 100_000]);
        let after = e.snapshot();
        let speeds: Vec<Speed> = (0..after.zones.len()).map(|z| after.zone_speed(z)).collect();
        assert_eq!(speeds, [Speed::Kmh(20), Speed::Kmh(20), Speed::Kmh(30), Speed::Kmh(50)], "the 20 km/h part is never raised");
        assert_eq!(e.view().revisions.len(), 1);
        assert!(e.undo());
        let s = e.snapshot();
        assert_eq!((s.segments, s.zones, s.speed, s.calming), (before.segments, before.zones, before.speed, before.calming));
    }

    #[test]
    fn the_school_template_is_still_one_undo() {
        let mut e = two_runs();
        let before = e.snapshot();
        e.pick_stretch(20_000, 120_000).unwrap();
        e.apply_template(Template::School).unwrap();
        let v = e.view();
        assert_eq!(v.revisions.len(), 1);
        assert_eq!(v.revisions[0].label.key, "rev-template");
        assert!(e.undo());
        let s = e.snapshot();
        assert_eq!((s.segments, s.zones, s.speed, s.calming), (before.segments, before.zones, before.speed, before.calming));
        assert_eq!(e.pick(), stretch(40_000, 100_000), "the pick is kept, clamped to the street");
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
    fn on_a_street_anchored_at_its_last_end_the_report_counts_from_the_plan_s_left_end() {
        // 120 m, anchored at its last end: the pick from 10 m to 100 m from the anchor is from 20 m to 110 m on the plan.
        let s = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 120_000, from_end: ZoneEnd::Last }));
        e.pick_stretch(10_000, 100_000).unwrap();
        let report = e.apply_template(Template::School).unwrap();
        assert_eq!(keys(&report.applied), ["template-school-applied-zone", "template-school-applied-speed", "template-school-applied-table"]);
        // The school stretch is 25 m to 85 m from the anchor, so 35 m to 95 m on the plan; its table, 7 m long, starts
        // 51.5 m from the anchor and 61.5 m from the left (120 - 51.5 - 7).
        assert_eq!(e.pick(), stretch(25_000, 85_000));
        assert_eq!(report.applied[0].args, vec![("from", Arg::Length(35_000)), ("to", Arg::Length(95_000))]);
        assert_eq!(e.snapshot().calming[0].from_mm, 51_500);
        assert_eq!(report.applied[2].args, vec![("at", Arg::Length(61_500))]);
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
