//! What an edit did to the street, as data: the width each use gained or lost, and the design checks that
//! began to pass and the ones that began to fail (#64).
//!
//! Pure: no DOM, no signals, no I/O and no words. `Input::of` reads the street's presentation-ready view (the
//! width table's shares and the checks' results, which are what the Notes tab shows) before and after an edit,
//! `diff` compares the two, and the result says everything as `Said` data that the page puts into words in its
//! language and units.
//!
//! One edit is one change: a grouped operation (Transform, an Atlas measure, Make pedestrian, Start over) is a
//! single step of the history, so it is a single `Impact`. Undo and Redo are the same comparison made across the
//! step they moved over, so the summary of an undo is the reverse of the summary of the edit.

use crate::restriction::model::Finding;
use crate::shared::catalogue::{Mode, StreetClass, mode_key};
use crate::shared::said::{Arg, Said};
use crate::street::model::View;
use crate::street::text::check_label;

/// One check as the Checks tab lists it: whether it passes, and what it is called.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckState {
    /// Names the check across edits: a design check's id, or the position of the area it is judged against.
    pub key: String,
    pub ok: bool,
    pub label: Said,
}

/// The street as the summary reads it, taken once before an edit and once after.
#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    pub class: StreetClass,
    /// What the street is called, for the sentence about its class.
    pub name: Said,
    /// The width each use has (the width table's design column), in millimetres.
    pub uses: Vec<(Mode, i32)>,
    pub checks: Vec<CheckState>,
    /// The label of the step the history stands on: the edit just made, or the one an Undo is about to take back.
    pub last: Option<Said>,
}

impl Input {
    /// The street's view, with how it stands against each area it is inside (the rows the Checks tab adds).
    pub fn of(view: &View, areas: &[Finding]) -> Input {
        let design = view.checks.iter().map(|c| CheckState { key: c.id.to_string(), ok: c.ok, label: check_label(c) });
        let areas = areas.iter().enumerate().map(|(i, f)| CheckState { key: format!("area-{i}"), ok: f.ok(), label: f.label() });
        Input {
            class: view.class,
            name: view.name.clone(),
            uses: view.outcomes.share.iter().map(|s| (s.mode, s.mm)).collect(),
            checks: design.chain(areas).collect(),
            last: view.revisions.last().map(|r| r.label.clone()),
        }
    }
}

/// What kind of change is summarized: it names the headline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Edit,
    Undo,
    Redo,
    StartOver,
}

/// A use whose width changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UseChange {
    pub mode: Mode,
    pub before_mm: i32,
    pub after_mm: i32,
}

impl UseChange {
    /// `Walking: 3.0 m -> 3.9 m (0.9 m more)`, as a message.
    pub fn said(&self) -> Said {
        let key = if self.after_mm > self.before_mm { "impact-use-more" } else { "impact-use-less" };
        Said::new(key)
            .with("use", Arg::Msg(mode_key(self.mode)))
            .with("before", Arg::Length(self.before_mm))
            .with("after", Arg::Length(self.after_mm))
            .with("amount", Arg::Length((self.after_mm - self.before_mm).abs()))
    }
}

/// Whether the checks as a whole got better or worse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Improved,
    Regressed,
    Mixed,
    Unchanged,
}

/// What a change did to the street.
#[derive(Clone, Debug, PartialEq)]
pub struct Impact {
    pub cause: Cause,
    /// What the change was: the edit's label, `Undone: ...`, `Redone: ...` or that the street was started over.
    pub headline: Said,
    /// The uses whose width changed, in the width table's order.
    pub uses: Vec<UseChange>,
    /// The street became pedestrian, or carries traffic again.
    pub class: Option<Said>,
    pub now_passing: Vec<Said>,
    pub now_failing: Vec<Said>,
    pub verdict: Verdict,
}

impl Impact {
    /// The change touched neither the space nor the checks.
    pub fn is_empty(&self) -> bool {
        self.uses.is_empty() && self.class.is_none() && self.now_passing.is_empty() && self.now_failing.is_empty()
    }

    /// The one line on the checks as a whole; none when no check changed.
    pub fn verdict_said(&self) -> Said {
        let counts = |s: Said| s.with("passing", Arg::Num(self.now_passing.len() as i64)).with("failing", Arg::Num(self.now_failing.len() as i64));
        counts(Said::new(match self.verdict {
            Verdict::Improved => "impact-verdict-improved",
            Verdict::Regressed => "impact-verdict-regressed",
            Verdict::Mixed => "impact-verdict-mixed",
            Verdict::Unchanged => "impact-verdict-unchanged",
        }))
    }
}

/// What `cause` did to the street, from how it stood before and after.
///
/// A check that passes now and did not before, or that is gone and was failing, is newly passing; one that fails
/// now and did not before, or is new and fails, is newly failing. A check that appears already passing, or goes
/// away already passing, is neither. A use is listed only when its width changed.
pub fn diff(cause: Cause, before: &Input, after: &Input) -> Impact {
    let uses = after
        .uses
        .iter()
        .filter_map(|&(mode, after_mm)| {
            let before_mm = before.uses.iter().find(|(m, _)| *m == mode).map_or(0, |(_, mm)| *mm);
            (before_mm != after_mm).then_some(UseChange { mode, before_mm, after_mm })
        })
        .collect();
    let mut now_passing = Vec::new();
    let mut now_failing = Vec::new();
    for now in &after.checks {
        match before.checks.iter().find(|c| c.key == now.key) {
            Some(was) if was.ok && !now.ok => now_failing.push(now.label.clone()),
            Some(was) if !was.ok && now.ok => now_passing.push(now.label.clone()),
            None if !now.ok => now_failing.push(now.label.clone()),
            _ => {}
        }
    }
    // A failing check that is no longer asked for has stopped failing.
    for was in before.checks.iter().filter(|c| !c.ok && !after.checks.iter().any(|n| n.key == c.key)) {
        now_passing.push(was.label.clone());
    }
    let verdict = match (now_passing.is_empty(), now_failing.is_empty()) {
        (false, true) => Verdict::Improved,
        (true, false) => Verdict::Regressed,
        (false, false) => Verdict::Mixed,
        (true, true) => Verdict::Unchanged,
    };
    let class = match (before.class == StreetClass::Pedestrian, after.class == StreetClass::Pedestrian) {
        (false, true) => Some("street-now-pedestrian"),
        (true, false) => Some("street-traffic-again"),
        _ => None,
    }
    .map(|key| Said::new(key).with("street", Arg::Said(Box::new(after.name.clone()))));
    Impact { cause, headline: headline(cause, before, after), uses, class, now_passing, now_failing, verdict }
}

fn headline(cause: Cause, before: &Input, after: &Input) -> Said {
    // The step an Undo took back is the one the history stood on before it; an edit or a Redo, the one it stands on now.
    let step = |from: &Option<Said>, key: &'static str| match from {
        Some(label) => Said::new(key).with("what", Arg::Said(Box::new(label.clone()))),
        None => Said::new("impact-changed"),
    };
    match cause {
        Cause::Edit => after.last.clone().unwrap_or_else(|| Said::new("impact-changed")),
        Cause::Undo => step(&before.last, "impact-undone"),
        Cause::Redo => step(&after.last, "impact-redone"),
        Cause::StartOver => Said::new("impact-started-over"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::{Side, kind_index};
    use crate::shared::testing::{en, fr};
    use crate::street::model::{Editor, Street, ZoneEnd, ZoneFrame, zoned};

    fn input(e: &Editor) -> Input {
        Input::of(&e.view(), &[])
    }

    fn kind(id: &str) -> usize {
        kind_index(id).unwrap()
    }

    fn first(e: &Editor) -> u32 {
        e.view().segments[0].uid
    }

    fn change(e: &mut Editor, f: impl FnOnce(&mut Editor) -> bool) -> Impact {
        let before = input(e);
        assert!(f(e), "the edit was refused");
        diff(Cause::Edit, &before, &input(e))
    }

    fn said(list: &[Said]) -> Vec<String> {
        list.iter().map(en).collect()
    }

    fn walking(i: &Impact) -> Option<(i32, i32)> {
        i.uses.iter().find(|u| u.mode == Mode::Foot).map(|u| (u.before_mm, u.after_mm))
    }

    #[test]
    fn narrowing_a_sidewalk_lists_only_the_use_that_changed_with_before_after_and_the_difference() {
        let mut e = Editor::new(0);
        let uid = first(&e);
        let i = change(&mut e, |e| e.set_width(uid, 3_000));
        assert_eq!(i.uses, [UseChange { mode: Mode::Foot, before_mm: 6_600, after_mm: 6_300 }]);
        assert_eq!(en(&i.uses[0].said()), "Walking: 6.6 m \u{2192} 6.3 m (0.3 m less)");
        assert!(!i.is_empty());
        assert_eq!(i.cause, Cause::Edit);
        let more = change(&mut e, |e| e.set_width(uid, 3_500));
        assert_eq!(en(&more.uses[0].said()), "Walking: 6.3 m \u{2192} 6.8 m (0.5 m more)");
    }

    #[test]
    fn a_length_follows_the_units_and_the_language_of_the_page() {
        let mut e = Editor::new(0);
        let uid = first(&e);
        let i = change(&mut e, |e| e.set_width(uid, 3_000));
        assert_eq!(fr(&i.uses[0].said()), "Marche\u{a0}: 6,6\u{a0}m \u{2192} 6,3\u{a0}m (0,3\u{a0}m de moins)");
        let feet = crate::shared::said::say_now(&crate::i18n_for(crate::shared::i18n::Locale::En), crate::shared::units::Units::Feet, &i.uses[0].said());
        assert!(feet.contains("ft"), "{feet}");
    }

    #[test]
    fn a_use_that_did_not_change_is_not_listed_and_each_one_that_did_is_in_the_width_tables_order() {
        let mut e = Editor::new(0);
        let (uid, tree) = (first(&e), kind("planting"));
        let i = change(&mut e, |e| e.set_width(uid, 3_000));
        assert!(i.uses.iter().all(|u| u.mode == Mode::Foot));
        let before = input(&e);
        e.add(tree, 0);
        let after = input(&e);
        let i = diff(Cause::Edit, &before, &after);
        let modes: Vec<Mode> = i.uses.iter().map(|u| u.mode).collect();
        let order: Vec<Mode> = Mode::ALL.iter().chain([&Mode::Neutral]).copied().filter(|m| modes.contains(m)).collect();
        assert_eq!(modes, order);
        assert!(modes.contains(&Mode::Green));
    }

    #[test]
    fn a_check_that_begins_to_fail_is_listed_as_newly_failing_and_not_as_newly_passing() {
        let mut e = Editor::new(0);
        let uid = first(&e);
        let i = change(&mut e, |e| e.set_width(uid, 3_800));
        assert_eq!(said(&i.now_failing), ["Fits the street width"]);
        assert!(i.now_passing.is_empty());
        assert_eq!(i.verdict, Verdict::Regressed);
    }

    #[test]
    fn a_check_that_begins_to_pass_is_listed_as_newly_passing_and_not_as_newly_failing() {
        let mut e = Editor::new(0);
        let uid = first(&e);
        e.set_width(uid, 3_800);
        let i = change(&mut e, |e| e.set_width(uid, 3_300));
        assert_eq!(said(&i.now_passing), ["Fits the street width"]);
        assert!(i.now_failing.is_empty());
        assert_eq!(i.verdict, Verdict::Improved);
    }

    #[test]
    fn an_edit_that_fixes_one_check_and_breaks_another_is_mixed_and_lists_both_apart() {
        // Too wide, and with a sidewalk missing on one side: widening nothing, removing the sidewalk fixes the width
        // and breaks the sidewalks.
        let mut e = Editor::new(0);
        let uid = first(&e);
        e.set_width(uid, 3_800);
        let i = change(&mut e, |e| e.remove(uid));
        assert_eq!(said(&i.now_passing), ["Fits the street width"]);
        assert_eq!(said(&i.now_failing), ["Sidewalk on both sides"]);
        assert_eq!(i.verdict, Verdict::Mixed);
    }

    #[test]
    fn a_check_that_keeps_its_result_is_not_called_out() {
        let mut e = Editor::new(0);
        let uid = first(&e);
        let i = change(&mut e, |e| e.set_width(uid, 3_000));
        assert!(i.now_passing.is_empty() && i.now_failing.is_empty());
        assert_eq!(i.verdict, Verdict::Unchanged);
    }

    #[test]
    fn an_edit_that_changes_neither_space_nor_checks_is_empty() {
        let mut e = Editor::new(0);
        let i = change(&mut e, |e| e.set_speed(Some(30)));
        assert!(i.is_empty(), "{i:?}");
        assert_eq!(i.verdict, Verdict::Unchanged);
        // The same street twice, whatever the cause.
        let now = input(&e);
        assert!(diff(Cause::Edit, &now, &now).is_empty());
    }

    #[test]
    fn undo_gives_the_reverse_of_the_edit_and_redo_gives_the_edit_again() {
        let mut e = Editor::new(0);
        let uid = first(&e);
        let edit = change(&mut e, |e| e.set_width(uid, 3_800));
        let wide = input(&e);
        assert!(e.undo());
        let narrow = input(&e);
        let undo = diff(Cause::Undo, &wide, &narrow);
        assert_eq!(walking(&edit), Some((6_600, 7_100)));
        assert_eq!(walking(&undo), Some((7_100, 6_600)));
        assert_eq!(said(&edit.now_failing), said(&undo.now_passing));
        assert!(undo.now_failing.is_empty());
        assert_eq!(undo.verdict, Verdict::Improved);
        assert!(e.redo());
        let redo = diff(Cause::Redo, &narrow, &input(&e));
        assert_eq!(redo.uses, edit.uses);
        assert_eq!(said(&redo.now_failing), said(&edit.now_failing));
        // The headline names the step that was taken back or done again.
        let step = wide.last.clone().unwrap();
        assert_eq!(en(&undo.headline), format!("Undone: {}", en(&step)));
        assert_eq!(en(&redo.headline), format!("Redone: {}", en(&step)));
        assert_eq!(en(&edit.headline), en(&step));
        assert_eq!(fr(&undo.headline), format!("Annulé\u{a0}: {}", fr(&step)));
        assert_eq!(fr(&redo.headline), format!("Rétabli\u{a0}: {}", fr(&step)));
    }

    #[test]
    fn starting_over_is_a_change_of_its_own_with_its_own_headline() {
        let mut e = Editor::new(0);
        let uid = first(&e);
        e.set_width(uid, 3_800);
        let before = input(&e);
        assert!(e.reset());
        let i = diff(Cause::StartOver, &before, &input(&e));
        assert_eq!(en(&i.headline), "Started over");
        assert_eq!(walking(&i), Some((7_100, 6_600)));
        assert_eq!(said(&i.now_passing), ["Fits the street width"]);
    }

    #[test]
    fn a_grouped_operation_is_one_change_covering_every_piece_it_touched() {
        let mut e = Editor::new(0);
        let sidewalk = kind("sidewalk");
        let i = change(&mut e, |e| e.make_pedestrian(sidewalk));
        assert_eq!(en(&i.headline), "Made pedestrian");
        let motor = i.uses.iter().find(|u| u.mode == Mode::Vehicle).unwrap();
        assert_eq!(motor.after_mm, 0);
        assert!(motor.before_mm > 0);
        // Everything the vehicles gave up, walking took.
        assert_eq!(walking(&i).map(|(b, a)| a - b), Some(motor.before_mm));
        // One step to take back: undoing it is the one reverse change.
        assert_eq!(e.view().revisions.len(), 1);
    }

    #[test]
    fn making_the_street_pedestrian_says_so_and_so_does_giving_it_traffic_again() {
        let mut e = Editor::new(0);
        let sidewalk = kind("sidewalk");
        let i = change(&mut e, |e| e.make_pedestrian(sidewalk));
        assert!(en(i.class.as_ref().unwrap()).ends_with("is now a pedestrian street."));
        assert!(!i.is_empty());
        let before = input(&e);
        assert!(e.undo());
        let back = diff(Cause::Undo, &before, &input(&e));
        assert!(en(back.class.as_ref().unwrap()).ends_with("carries traffic again."));
        // A change that leaves the class alone says nothing of it.
        let mut e = Editor::new(0);
        let uid = first(&e);
        assert!(change(&mut e, |e| e.set_width(uid, 3_000)).class.is_none());
    }

    #[test]
    fn a_street_with_zones_is_compared_by_the_zone_selected() {
        let length = 40_000;
        let street = zoned(Street::sample(0, Side::Right), &[0, 20_000], length);
        let mut e = Editor::from_street(&street, &street, 0).with_frame(Some(ZoneFrame { length_mm: length, from_end: ZoneEnd::First }));
        let uid = first(&e);
        // An edit goes to the selected zone, which has its own section, and so does its summary.
        assert!(e.select_zone(1));
        let i = change(&mut e, |e| e.set_width(uid, 3_000));
        assert_eq!(walking(&i), Some((6_600, 6_300)));
        // The other zone's section is untouched.
        e.select_zone(0);
        assert_eq!(e.view().outcomes.share.iter().find(|s| s.mode == Mode::Foot).map(|s| s.mm), Some(6_600));
    }

    #[test]
    fn a_check_that_is_asked_for_no_more_has_stopped_failing_and_a_new_one_that_fails_has_started() {
        let check = |key: &str, ok| CheckState { key: key.into(), ok, label: Said::new(if ok { "check-passes" } else { "check-fails" }) };
        let base = Input { class: StreetClass::Local, name: Said::new("street-today"), uses: Vec::new(), checks: Vec::new(), last: None };
        let was = Input { checks: vec![check("a", false), check("b", true)], ..base.clone() };
        let now = Input { checks: vec![check("c", false), check("d", true)], ..base };
        let i = diff(Cause::Edit, &was, &now);
        assert_eq!(i.now_passing.len(), 1, "a is gone and was failing");
        assert_eq!(i.now_failing.len(), 1, "c is new and fails");
        assert_eq!(en(&i.headline), "The street changed");
    }
}
