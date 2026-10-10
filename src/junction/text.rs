//! What the page says in words after the junction changes: the status line, and
//! what a screen reader is told about an edit or a selection.
//!
//! A view words the status with `arms_text` and `fit_text`, which ask with the tracked `tr` and
//! `say`, so it is drawn again when the language is switched; a command uses the `_now` forms.

use crate::junction::model::control_key;
use crate::junction::read_model::JView;
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::{Arg, Said, say, say_now};
use crate::shared::units::Units;

/// How a message is asked for: tracked for a view, or as it is now for a command.
#[derive(Clone, Copy)]
struct Words<'a> {
    i18n: &'a I18n,
    watched: bool,
}

impl Words<'_> {
    fn tr(self, key: &str, args: &Args) -> String {
        if self.watched { self.i18n.tr(key, args) } else { self.i18n.tr_now(key, args) }
    }

    fn say(self, said: &Said) -> String {
        // The words of the status line speak of no length.
        if self.watched { say(self.i18n, Units::Metres, said) } else { say_now(self.i18n, Units::Metres, said) }
    }

    fn arms(self, v: &JView) -> String {
        self.tr("jn-arms", &Args::new().num("n", v.arms.len() as i64))
    }

    fn fit(self, v: &JView) -> String {
        let bad: Vec<&Said> = v.checks.iter().filter(|c| !c.ok).map(|c| &c.label).collect();
        match bad.len() {
            0 => {
                self.tr("jn-fit-ok", &Args::new().str("arms", self.arms(v)).str("control", self.tr(control_key(v.control_index), &Args::new()).to_lowercase()))
            }
            n => {
                let list = bad
                    .into_iter()
                    .cloned()
                    .reduce(|head, tail| Said::new("jn-list-comma").with("head", Arg::Said(Box::new(head))).with("tail", Arg::Said(Box::new(tail))))
                    .map(|l| self.say(&l).to_lowercase())
                    .unwrap_or_default();
                self.tr("jn-fit-bad", &Args::new().num("n", n as i64).str("checks", list))
            }
        }
    }
}

/// "5 streets", for a view: drawn again when the language changes.
pub fn arms_text(v: &JView, i18n: &I18n) -> String {
    Words { i18n, watched: true }.arms(v)
}

/// The status line, for a view: what needs attention, or that every check passes.
pub fn fit_text(v: &JView, i18n: &I18n) -> String {
    Words { i18n, watched: true }.fit(v)
}

/// The status line, for a command: nothing is watched.
pub fn fit_text_now(v: &JView, i18n: &I18n) -> String {
    Words { i18n, watched: false }.fit(v)
}

/// What the last edit was, and where the junction stands.
pub fn edit_text(v: &JView, i18n: &I18n, units: Units) -> Option<String> {
    v.revisions.last().map(|r| i18n.tr_now("jn-edit", &Args::new().str("edit", say_now(i18n, units, &r.label)).str("fit", fit_text_now(v, i18n))))
}

/// A sample junction was loaded.
#[cfg(test)]
pub fn sample_text(v: &JView, i18n: &I18n) -> String {
    format!("{}. {}", say_now(i18n, Units::Metres, &v.name), fit_text_now(v, i18n))
}

/// What is selected, for a screen reader; nothing when nothing is.
pub fn selection_text(v: &JView, i18n: &I18n, units: Units) -> Option<String> {
    let s = &v.selected;
    let kind = s.kind?;
    let arm = v.arms.iter().find(|a| a.uid == s.uid);
    let label = arm.map_or_else(String::new, |a| say_now(i18n, units, &a.label));
    let locale = i18n.locale_now();
    let args = Args::new().str("arm", label);
    Some(match kind {
        "bus" => i18n.tr_now("jn-sel-bus", &Args::new()),
        "cycle" => i18n.tr_now("jn-sel-cycle", &Args::new().str("width", units.length_in(v.ring.as_ref().and_then(|r| r.cycle_mm).unwrap_or(0), locale))),
        "lane" => i18n.tr_now("jn-sel-lane", &args.num("n", s.lane as i64 + 1).num("count", arm.map_or(0, |a| a.lanes.len()) as i64)),
        "arm" => i18n.tr_now("jn-sel-arm", &args.num("degrees", arm.map_or(0, |a| a.bearing).into())),
        "corner" => {
            let radius = v.corners.iter().find(|c| c.uid == s.uid).map_or(0, |c| c.radius_mm);
            i18n.tr_now("jn-sel-corner", &args.str("radius", units.length_in(radius, locale)))
        }
        _ => {
            let continuous = arm.and_then(|a| a.crossing.as_ref()).is_some_and(|c| c.continuous);
            i18n.tr_now(if continuous { "jn-sel-crossing-continuous" } else { "jn-sel-crossing" }, &args)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::*;
    use crate::shared::i18n::Locale;

    fn arm_at(j: &Junction, bearing: i32) -> u32 {
        j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid
    }

    fn en() -> std::rc::Rc<I18n> {
        crate::i18n_for(Locale::En)
    }

    fn fr() -> std::rc::Rc<I18n> {
        crate::i18n_for(Locale::FrCa)
    }

    #[test]
    fn a_junction_that_works_says_how_many_streets_and_how_it_is_run() {
        let j = Junction::new(0);
        assert_eq!(fit_text(&j.view(), &en()), "4 streets, traffic signal. Every check passes.");
        assert_eq!(arms_text(&j.view(), &en()), "4 streets");
        assert_eq!(fit_text(&j.view(), &fr()), "4 rues, feux de circulation. Toutes les vérifications réussissent.");
    }

    #[test]
    fn a_junction_that_fails_one_check_names_it() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        j.set_island(n, false); // the avenue is then too far to cross in one go
        assert_eq!(fit_text(&j.view(), &en()), "One check needs attention: crossing distance.");
        assert_eq!(fit_text(&j.view(), &fr()), "Une vérification est à corriger\u{a0}: distance de traversée.");
    }

    #[test]
    fn a_junction_that_fails_several_counts_and_names_them() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        j.set_corner(n, 11_000);
        assert_eq!(fit_text(&j.view(), &en()), "2 checks need attention: slow turns at crossings, sidewalk survives the corner.");
        assert_eq!(fit_text(&j.view(), &fr()), "2 vérifications sont à corriger\u{a0}: virages lents aux passages pour piétons, le trottoir survit au coin.");
    }

    #[test]
    fn the_counts_said_are_singular_at_zero_and_one_in_french_and_at_one_in_english() {
        let count = |i: &I18n, key: &str, name: &'static str, n: i64| i.tr_now(key, &Args::new().num(name, n).str("checks", "x").str("arm", "A"));
        let (en, fr) = (en(), fr());
        let cases: [(&str, &'static str, [&str; 3], [&str; 3]); 4] = [
            ("jn-arms", "n", ["0 streets", "1 street", "2 streets"], ["0 rue", "1 rue", "2 rues"]),
            // The status line says "jn-fit-ok" when no check fails, so it never asks for this one with 0;
            // the 0 case only pins how each language's plural rule reads it.
            (
                "jn-fit-bad",
                "n",
                ["0 checks need attention: x.", "One check needs attention: x.", "2 checks need attention: x."],
                ["Une vérification est à corriger\u{a0}: x.", "Une vérification est à corriger\u{a0}: x.", "2 vérifications sont à corriger\u{a0}: x."],
            ),
            (
                "jn-check-signal-bad",
                "n",
                ["0 streets is too many for one signal", "1 street is too many for one signal", "2 streets is too many for one signal"],
                ["0 rue, c’est trop pour un seul jeu de feux", "1 rue, c’est trop pour un seul jeu de feux", "2 rues, c’est trop pour un seul jeu de feux"],
            ),
            ("jn-sel-arm", "degrees", ["A, 0 degrees", "A, 1 degree", "A, 2 degrees"], ["A, 0 degré", "A, 1 degré", "A, 2 degrés"]),
        ];
        for (key, name, english, french) in cases {
            for n in 0..3 {
                assert_eq!(count(&en, key, name, n as i64), english[n], "{key} {n} in English");
                assert_eq!(count(&fr, key, name, n as i64), french[n], "{key} {n} in French");
            }
        }
        assert_eq!(count(&en, "jn-arms", "n", 3), "3 streets");
        assert_eq!(count(&fr, "jn-arms", "n", 3), "3 rues");
    }

    #[test]
    fn an_edit_is_announced_by_its_label_and_where_the_junction_stands() {
        let mut j = Junction::new(0);
        assert_eq!(edit_text(&j.view(), &en(), Units::Metres), None);
        let n = arm_at(&j, 0);
        j.set_corner(n, 7_000);
        assert_eq!(
            edit_text(&j.view(), &en(), Units::Metres).unwrap(),
            "Corner after Sample Avenue 2 (north): 7.0 m radius. 4 streets, traffic signal. Every check passes."
        );
        assert_eq!(
            edit_text(&j.view(), &fr(), Units::Metres).unwrap(),
            "Coin après Sample Avenue 2 (nord)\u{a0}: rayon de 7,0\u{a0}m. 4 rues, feux de circulation. Toutes les vérifications réussissent."
        );
    }

    #[test]
    fn a_sample_is_announced_by_its_name() {
        assert_eq!(sample_text(&Junction::new(1).view(), &en()), "Street and lane. 3 streets, side streets stop. Every check passes.");
    }

    #[test]
    fn nothing_selected_says_nothing() {
        assert_eq!(selection_text(&Junction::new(0).view(), &en(), Units::Metres), None);
    }

    #[test]
    fn each_kind_of_selection_says_what_it_is() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        let say = |j: &Junction, u: Units| selection_text(&j.view(), &en(), u).unwrap();
        j.select(Target::Arm(n));
        assert_eq!(say(&j, Units::Metres), "Sample Avenue 2 (north), 0 degrees");
        j.select(Target::Corner(n));
        assert_eq!(say(&j, Units::Metres), "Corner after Sample Avenue 2 (north), 6.0 m radius");
        assert_eq!(say(&j, Units::Feet), "Corner after Sample Avenue 2 (north), 19.7 ft radius");
        assert_eq!(selection_text(&j.view(), &fr(), Units::Metres).unwrap(), "Coin après Sample Avenue 2 (nord), rayon de 6,0\u{a0}m");
        j.select(Target::Crossing(n));
        assert_eq!(say(&j, Units::Metres), "Crossing on Sample Avenue 2 (north)");
        j.select(Target::Lane(n, 1));
        assert_eq!(say(&j, Units::Metres), "Lane 2 of 2, Sample Avenue 2 (north)");
    }

    #[test]
    fn a_continuous_crossing_is_selected_as_a_continuous_sidewalk_in_both_languages() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        assert!(j.set_continuous(n, true));
        j.select(Target::Crossing(n));
        assert_eq!(selection_text(&j.view(), &en(), Units::Metres).unwrap(), "Continuous sidewalk across Sample Avenue 2 (north)");
        assert_eq!(selection_text(&j.view(), &fr(), Units::Metres).unwrap(), "Trottoir continu à travers Sample Avenue 2 (nord)");
        assert!(j.set_continuous(n, false));
        assert_eq!(selection_text(&j.view(), &en(), Units::Metres).unwrap(), "Crossing on Sample Avenue 2 (north)");
    }

    #[test]
    fn the_bus_lane_and_cycle_track_say_what_they_are() {
        let mut j = Junction::new(0);
        j.set_control(ROUNDABOUT);
        let (n, s) = (arm_at(&j, 0), arm_at(&j, 180));
        j.set_bus(Some((n, s)));
        j.set_cycle(Some(2_000));
        j.select(Target::Bus);
        assert_eq!(selection_text(&j.view(), &en(), Units::Metres).unwrap(), "Bus lane across the middle");
        j.select(Target::Cycle);
        assert_eq!(selection_text(&j.view(), &en(), Units::Metres).unwrap(), "Cycle track, 2.0 m wide");
        assert_eq!(selection_text(&j.view(), &fr(), Units::Metres).unwrap(), "Piste cyclable, 2,0\u{a0}m de large");
    }
}
