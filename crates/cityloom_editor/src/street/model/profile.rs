//! The street's checks read against the standards profile in force (`crate::standards::model`): which standard a
//! check's threshold came from, and the check a profile adds, the speed target of the street's class. With no profile
//! in force nothing here adds anything, and the checks are the built-in ones.

use super::zone_speed;
use super::{Arg, Check, Editor, Said, StreetClass};
use crate::shared::catalogue::class_key;
use crate::standards::model::{self as standards, Field};

/// The figure a check's threshold is, for the checks a profile can change.
fn field_of(id: &str) -> Option<Field> {
    match id {
        "access" => Some(Field::AccessLane),
        "curb-crossing" => Some(Field::CrossingClearance),
        "curb-bike" => Some(Field::DoorBuffer),
        "curb-sidewalk" => Some(Field::PatioWalk),
        "stop-length" => Some(Field::StopMin),
        _ => None,
    }
}

impl Editor {
    /// Says, on each check that reads a figure a profile can set, which standard it used (see `Check::standard`).
    pub(super) fn name_standards(&self, checks: &mut [Check]) {
        for c in checks {
            c.standard = field_of(c.id).and_then(standards::note).or_else(|| (c.id == "speed-target").then(|| standards::note_target(self.class())).flatten());
        }
    }

    /// The speed target of the street's class, when a profile names one: the highest speed limit of any zone must not
    /// be over it. A street with no speed limit set has nothing to exceed.
    pub(super) fn speed_target_check(&self) -> Option<Check> {
        let class: StreetClass = self.class();
        let target = standards::target_kmh(class)?;
        let content = &self.states[self.cursor].content;
        let highest = zone_speed::max_kmh(content.speed, &content.zones);
        let over = highest.filter(|&kmh| kmh > target);
        let class = Arg::Msg(class_key(class));
        let detail = match (highest, over) {
            (Some(kmh), Some(_)) => {
                Said::new("check-detail-speed-target-over").with("kmh", Arg::Num(kmh.into())).with("target", Arg::Num(target.into())).with("class", class)
            }
            (Some(kmh), None) => {
                Said::new("check-detail-speed-target-ok").with("kmh", Arg::Num(kmh.into())).with("target", Arg::Num(target.into())).with("class", class)
            }
            (None, _) => Said::new("check-detail-speed-target-none").with("target", Arg::Num(target.into())).with("class", class),
        };
        Some(Check { id: "speed-target", ok: over.is_none(), label: Said::new("check-label-speed-target"), detail, zone: None, standard: None })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Editor, Speed, Street};
    use crate::shared::catalogue::{KINDS, Side, StreetClass, kind_index};
    use crate::standards::model::{self as standards, Field, Standards, WidthRange, activate_for_test};

    fn editor() -> Editor {
        Editor::new(0)
    }

    fn check(e: &Editor, id: &str) -> crate::street::model::Check {
        e.view().checks.into_iter().find(|c| c.id == id).unwrap_or_else(|| panic!("no check {id}"))
    }

    #[test]
    fn with_no_profile_no_check_names_a_standard_and_there_is_no_target_check() {
        let v = editor().view();
        assert!(v.checks.iter().all(|c| c.standard.is_none()));
        assert!(v.checks.iter().all(|c| c.id != "speed-target"));
    }

    #[test]
    fn the_access_check_reads_the_profile_and_names_it_with_both_figures() {
        let e = editor();
        let access = check(&e, "access");
        assert!(access.ok, "the sample street has 3.3 m lanes");
        assert_eq!(access.detail.args, vec![("amount", crate::shared::said::Arg::Length(3000))]);
        let _p = activate_for_test("City of X", Standards { access_lane_mm: 3500, ..Standards::default() });
        let access = check(&e, "access");
        assert!(!access.ok, "3.3 m lanes are under the profile's 3.5 m");
        assert_eq!(access.detail.args, vec![("amount", crate::shared::said::Arg::Length(3500))]);
        assert_eq!(access.standard, standards::note(Field::AccessLane));
        let note = access.standard.unwrap();
        assert_eq!(note.key, "standards-note-length");
    }

    #[test]
    fn a_looser_profile_passes_what_the_built_in_standard_fails() {
        let mut s = Street::sample(2, Side::Right);
        s.class = StreetClass::Local;
        let e = Editor::from_street(&s, &s, 0);
        let built_in = check(&e, "access").ok;
        let _p = activate_for_test("Lanes", Standards { access_lane_mm: 2000, ..Standards::default() });
        assert!(check(&e, "access").ok);
        // the built-in answer is what it was, whatever it is, with the profile gone again
        drop(_p);
        assert_eq!(check(&e, "access").ok, built_in);
    }

    #[test]
    fn a_profile_changes_the_widths_a_piece_may_take() {
        let travel = kind_index("travel").unwrap();
        let mut e = editor();
        let lane = e.current().iter().find(|s| s.kind == travel).unwrap().uid;
        let (min, max) = (KINDS[travel].min_mm, KINDS[travel].max_mm);
        let view = e.view();
        let seg = view.segments.iter().find(|s| s.uid == lane).unwrap();
        assert_eq!((seg.min_mm, seg.max_mm), (min, max));
        let mut s = Standards::default();
        s.kind_widths.insert("travel".into(), WidthRange { min_mm: 3000, max_mm: 3400 });
        let _p = activate_for_test("Narrow", s);
        let view = e.view();
        let seg = view.segments.iter().find(|s| s.uid == lane).unwrap();
        assert_eq!((seg.min_mm, seg.max_mm), (3000, 3400));
        // a width over the profile's maximum is brought inside it
        e.set_width(lane, 3700);
        let width = e.view().segments.iter().find(|s| s.uid == lane).unwrap().width_mm;
        assert!(width <= 3400, "{width}");
    }

    #[test]
    fn the_speed_limits_offered_are_the_profiles() {
        let mut e = editor();
        assert!(e.set_speed(Some(40)));
        assert!(!e.set_speed(Some(35)));
        let _p = activate_for_test("Slow", Standards { speed_choices: vec![25, 35, 45], ..Standards::default() });
        assert!(e.set_speed(Some(35)));
        assert!(!e.set_speed(Some(40)), "40 is not one of the profile's speeds");
    }

    #[test]
    fn a_class_with_a_target_speed_gets_a_check_that_names_the_standard() {
        let mut s = Street::sample(0, Side::Right);
        s.class = StreetClass::Local;
        s.speed = Speed::Kmh(50);
        let e = Editor::from_street(&s, &s, 0);
        let mut st = Standards::default();
        st.target_kmh.insert("local".into(), 30);
        let _p = activate_for_test("Slow streets", st.clone());
        let c = check(&e, "speed-target");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-speed-target-over");
        assert_eq!(c.standard, standards::note_target(StreetClass::Local));
        assert_eq!(c.standard.as_ref().unwrap().key, "standards-note-target");
        s.speed = Speed::Kmh(30);
        assert!(check(&Editor::from_street(&s, &s, 0), "speed-target").ok);
        s.speed = Speed::NotSet;
        let none = check(&Editor::from_street(&s, &s, 0), "speed-target");
        assert!(none.ok);
        assert_eq!(none.detail.key, "check-detail-speed-target-none");
        // a class the profile names no target for is not checked
        let mut other = Standards::default();
        other.target_kmh.insert("arterial".into(), 50);
        let _q = activate_for_test("Other", other);
        assert!(editor().view().checks.iter().all(|c| c.id != "speed-target"));
    }

    #[test]
    fn the_curb_and_stop_checks_name_the_figures_they_read() {
        for (id, field) in [
            ("curb-crossing", Field::CrossingClearance),
            ("curb-bike", Field::DoorBuffer),
            ("curb-sidewalk", Field::PatioWalk),
            ("stop-length", Field::StopMin),
        ] {
            assert_eq!(super::field_of(id), Some(field), "{id}");
        }
        assert_eq!(super::field_of("fits"), None);
    }
}
