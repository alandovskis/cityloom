//! The Transform dialog's choices, and what each would do to the street, tried on a copy before it is applied.

use std::rc::Rc;

use crate::shared::catalogue::{KINDS, PEDESTRIAN_KINDS, kind_index};
use crate::shared::said::Said;
use crate::street::model::{Template, TemplateReport, TemplateView, View};
use crate::street::vm::StreetVm;
use crate::street::vm::impact::{Cause, Impact, Input, diff};

/// One whole-street change the dialog offers: a kind the motor pieces become, an Atlas measure, or a template.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Choice {
    /// The catalogue index of a kind of `PEDESTRIAN_KINDS`.
    Pedestrian(usize),
    /// An Atlas code (`measures::DEFS`).
    Measure(&'static str),
    Template(Template),
}

impl Choice {
    /// A key naming the choice on the page: `ped-<kind id>`, `measure-<code lowercased>` or `template-<template id>`.
    pub fn key(self) -> String {
        match self {
            Choice::Pedestrian(kind) => format!("ped-{}", KINDS[kind].id),
            Choice::Measure(code) => format!("measure-{}", code.to_lowercase()),
            Choice::Template(t) => format!("template-{}", t.id()),
        }
    }
}

/// The dialog's groups of choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Pedestrian,
    Transit,
    Templates,
}

/// What the Transform dialog offers (`StreetVm::transform_choices`): the same choices the Transform menu offered.
#[derive(Clone, Debug, PartialEq)]
pub struct TransformChoices {
    /// Catalogue indices of the kinds the motor pieces can become; none when no piece is a motor piece.
    pub pedestrian: Vec<usize>,
    /// One entry per Atlas family with a choice, in order of first appearance: the code's letter, lowercased
    /// ('a'..='f'), and its codes in the Notes table's order.
    pub transit: Vec<(char, Vec<&'static str>)>,
    /// Every street template, with the reason one cannot be applied to the selected zone.
    pub templates: Vec<TemplateView>,
}

impl TransformChoices {
    /// The groups with something in them, in the order Pedestrian, Transit, Templates.
    pub fn groups(&self) -> Vec<Group> {
        [Group::Pedestrian, Group::Transit, Group::Templates].into_iter().filter(|g| !self.in_group(*g).is_empty()).collect()
    }

    /// The choices of a group, the transit families one after the other.
    pub fn in_group(&self, group: Group) -> Vec<Choice> {
        match group {
            Group::Pedestrian => self.pedestrian.iter().map(|k| Choice::Pedestrian(*k)).collect(),
            Group::Transit => self.transit.iter().flat_map(|(_, codes)| codes.iter().map(|c| Choice::Measure(c))).collect(),
            Group::Templates => self.templates.iter().map(|t| Choice::Template(t.template)).collect(),
        }
    }

    /// Nothing to offer.
    pub fn is_empty(&self) -> bool {
        self.pedestrian.is_empty() && self.transit.is_empty() && self.templates.is_empty()
    }
}

/// What a choice would do to the street: the street as it is now, and the street after or why the choice is refused.
/// Neither view has a selected piece.
#[derive(Clone)]
pub struct Preview {
    pub before: Rc<View>,
    pub outcome: Outcome,
}

#[derive(Clone)]
pub enum Outcome {
    /// The street after the change, what the change did (as the page's summary says it after an edit) and, for a
    /// template, its report (empty but for School).
    Changed { after: Rc<View>, impact: Impact, report: Option<TemplateReport> },
    /// The edit refuses the choice, and why.
    Refused(Said),
}

impl Preview {
    /// The choice changes the street, so it can be applied.
    pub fn can_apply(&self) -> bool {
        matches!(self.outcome, Outcome::Changed { .. })
    }
}

/// The Atlas families of `codes`, in order of first appearance, each with its codes in the order given.
fn families(codes: impl IntoIterator<Item = &'static str>) -> Vec<(char, Vec<&'static str>)> {
    let mut families: Vec<(char, Vec<&'static str>)> = Vec::new();
    for code in codes {
        let family = code.chars().next().map_or('?', |c| c.to_ascii_lowercase());
        match families.iter_mut().find(|(f, _)| *f == family) {
            Some((_, codes)) => codes.push(code),
            None => families.push((family, vec![code])),
        }
    }
    families
}

impl StreetVm {
    /// What the Transform dialog offers, drawn again when the street is edited: the kinds the motor pieces can become
    /// (while the street has a motor piece), the transit measures the street is not already and can be arranged as
    /// (the Notes table's Arrange rows, by Atlas family), and every template.
    pub fn transform_choices(&self) -> TransformChoices {
        let v = self.view();
        TransformChoices {
            pedestrian: if v.has_motor { PEDESTRIAN_KINDS.iter().filter_map(|id| kind_index(id)).collect() } else { Vec::new() },
            transit: families(v.measures.iter().filter(|m| m.can_apply && !m.present).map(|m| m.code)),
            templates: v.templates.clone(),
        }
    }

    /// What `choice` would do, found by running the very edit Apply runs on a copy of the editor (which keeps the
    /// selected zone, the region, the time shown, the zone frame and the ends). The street is not touched, nothing is
    /// said, and nothing is watched.
    pub fn preview(&self, choice: Choice) -> Preview {
        let mut e = self.core.read(|e| e.clone());
        e.select(None);
        let before = Rc::new(e.view());
        let before_input = Input::of(&before, &self.findings_for(&e.snapshot()));
        let done: Result<Option<TemplateReport>, Said> = match choice {
            Choice::Pedestrian(kind) => e.make_pedestrian(kind).then_some(None).ok_or_else(|| Said::new("transform-refused")),
            Choice::Measure(code) => e.apply_measure(code).then_some(None).ok_or_else(|| Said::new("measure-refused")),
            Choice::Template(t) => e.apply_template(t).map(Some).map_err(|refusal| refusal.said(t)),
        };
        let outcome = match done {
            Ok(report) => {
                let after = Rc::new(e.view());
                let impact = diff(Cause::Edit, &before_input, &Input::of(&after, &self.findings_for(&e.snapshot())));
                Outcome::Changed { after, impact, report }
            }
            Err(said) => Outcome::Refused(said),
        };
        Preview { before, outcome }
    }

    /// Applies `choice` as the page does: one undoable step, announced; a refusal changes nothing.
    pub fn apply_choice(&self, choice: Choice) -> bool {
        match choice {
            Choice::Pedestrian(kind) => self.apply(|e| e.make_pedestrian(kind)),
            Choice::Measure(code) => self.apply_measure(code),
            Choice::Template(t) => self.apply_template(t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::{REGIONS, Side, StreetClass, kind_index};
    use crate::shared::i18n::Locale;
    use crate::shared::ports::{ManualScheduler, RecordingAnnouncer, test_ports_with_time};
    use crate::street::model::{Editor, Piece, Street, StreetExt, TemplateRefusal, ZoneEnd, ZoneFrame};
    use crate::street::vm::IMPACT_DELAY_MS;

    const LENGTH: i32 = 60_000;

    /// Right-hand traffic: sidewalk, parking, two bus lanes, parking and a sidewalk.
    const PARKED: [(&str, i32, Option<usize>); 6] =
        [("sidewalk", 3000, None), ("parking", 2400, None), ("bus", 3300, Some(1)), ("bus", 3300, Some(0)), ("parking", 2400, None), ("sidewalk", 3000, None)];
    /// Too narrow for the Commercial main street.
    const NARROW: [(&str, i32, Option<usize>); 3] = [("sidewalk", 1800, None), ("travel", 3000, Some(1)), ("sidewalk", 1800, None)];

    struct Rig {
        vm: Rc<StreetVm>,
        said: Rc<RecordingAnnouncer>,
        time: Rc<ManualScheduler>,
    }

    fn rig(sample: usize) -> Rig {
        let (ports, said, _, time) = test_ports_with_time();
        Rig { vm: StreetVm::new(ports, crate::i18n_for(Locale::En), Editor::new(sample), None), said, time }
    }

    fn street(pieces: &[(&str, i32, Option<usize>)]) -> Street {
        let pieces: Vec<Piece> =
            pieces.iter().map(|&(id, width_mm, direction)| Piece { kind: kind_index(id).unwrap(), width_mm, direction, variants: Vec::new() }).collect();
        Street::imported(StreetClass::Local, Side::Right, &pieces)
    }

    fn rig_of(street: &Street, frame_mm: Option<i32>, locale: Locale) -> Rig {
        let region = REGIONS.iter().position(|r| r.drive_side == Side::Right).unwrap();
        let (ports, said, _, time) = test_ports_with_time();
        let editor = Editor::from_street(street, street, region).with_frame(frame_mm.map(|length_mm| ZoneFrame { length_mm, from_end: ZoneEnd::First }));
        Rig { vm: StreetVm::new(ports, crate::i18n_for(locale), editor, None), said, time }
    }

    fn sidewalk() -> usize {
        kind_index("sidewalk").unwrap()
    }

    fn json(v: &View) -> serde_json::Value {
        serde_json::to_value(v).unwrap()
    }

    /// Previews `choice`, applies it, and holds the preview's After and impact equal to the street's after Apply.
    fn assert_preview_matches_apply(r: &Rig, choice: Choice) {
        let preview = r.vm.preview(choice);
        let Outcome::Changed { after, impact, .. } = preview.outcome else { panic!("{choice:?} previews no change") };
        assert!(r.vm.apply_choice(choice), "{choice:?}");
        assert_eq!(json(&after), json(&r.vm.view_now()), "{choice:?}");
        r.time.advance(IMPACT_DELAY_MS + 1);
        let real = r.vm.impact_now().unwrap_or_else(|| panic!("{choice:?} shows no impact"));
        assert_eq!(Impact { headline: real.headline.clone(), ..impact }, real, "{choice:?}");
    }

    #[test]
    fn the_choices_group_the_measures_by_atlas_family() {
        let r = rig(0);
        let choices = r.vm.transform_choices();
        let offered: Vec<&str> = r.vm.view_now().measures.iter().filter(|m| m.can_apply && !m.present).map(|m| m.code).collect();
        assert!(!offered.is_empty(), "the sample is offered a measure");
        let flat: Vec<&str> = choices.transit.iter().flat_map(|(_, codes)| codes.iter().copied()).collect();
        assert_eq!(flat, offered);
        for (family, codes) in &choices.transit {
            assert!(codes.iter().all(|c| c.chars().next().unwrap().to_ascii_lowercase() == *family), "{family} {codes:?}");
        }
        let mut families: Vec<char> = choices.transit.iter().map(|(f, _)| *f).collect();
        families.dedup();
        families.sort();
        families.dedup();
        assert_eq!(families.len(), choices.transit.len(), "no family twice");
        assert_eq!(choices.in_group(Group::Transit), offered.iter().map(|c| Choice::Measure(c)).collect::<Vec<_>>());
    }

    #[test]
    fn the_groups_leave_out_an_empty_one() {
        let r = rig(0);
        assert_eq!(r.vm.transform_choices().groups()[0], Group::Pedestrian);
        assert!(r.vm.apply(|e| e.make_pedestrian(sidewalk())));
        let choices = r.vm.transform_choices();
        assert!(!choices.groups().contains(&Group::Pedestrian), "{:?}", choices.groups());
        assert_eq!(choices.in_group(Group::Templates).len(), 5);
        assert!(choices.in_group(Group::Pedestrian).is_empty());
        assert!(!choices.is_empty());
    }

    #[test]
    fn a_choice_has_a_key_for_the_page() {
        assert_eq!(Choice::Pedestrian(sidewalk()).key(), "ped-sidewalk");
        assert_eq!(Choice::Measure("B1").key(), "measure-b1");
        assert_eq!(Choice::Template(Template::BusCorridor).key(), "template-bus-corridor");
    }

    #[test]
    fn preview_matches_apply() {
        let choices = rig(0).vm.transform_choices();
        let groups = choices.groups();
        let all: Vec<Choice> = groups.iter().flat_map(|g| choices.in_group(*g)).collect();
        let mut tried = 0;
        for c in all {
            let r = rig(0);
            if !r.vm.preview(c).can_apply() {
                continue;
            }
            assert_preview_matches_apply(&r, c);
            tried += 1;
        }
        assert!(tried > 3, "most choices apply to the sample: {tried}");
    }

    #[test]
    fn the_preview_leaves_the_street_alone() {
        let r = rig(0);
        let choices = r.vm.transform_choices();
        let revisions = |r: &Rig| serde_json::to_value(&r.vm.view_now().revisions).unwrap();
        let (street, history) = (r.vm.read(|e| e.snapshot()), revisions(&r));
        for g in choices.groups() {
            for c in choices.in_group(g) {
                r.vm.preview(c);
                assert_eq!(r.vm.read(|e| e.snapshot()), street, "{c:?}");
                assert_eq!(revisions(&r), history, "{c:?}");
            }
        }
        assert!(r.said.take().is_empty());
        r.time.advance(IMPACT_DELAY_MS + 1);
        assert_eq!(r.vm.impact_now(), None);
    }

    #[test]
    fn preview_matches_apply_at_the_time_shown() {
        let r = rig(0);
        let parking = r.vm.view_now().segments[1].uid;
        assert!(r.vm.edit(|e| e.add_variant(parking))); // a transit lane 07:00-10:00
        assert!(r.vm.edit(|e| e.set_time(8 * 60)));
        assert_eq!(r.vm.view_now().segments[1].active_variant, Some(0));
        assert_preview_matches_apply(&r, Choice::Pedestrian(sidewalk()));
        assert_eq!(r.vm.view_now().time_min, 8 * 60);
    }

    #[test]
    fn preview_of_school_carries_its_report_and_matches_apply() {
        let r = rig_of(&street(&PARKED), Some(LENGTH), Locale::En);
        let preview = r.vm.preview(Choice::Template(Template::School));
        let Outcome::Changed { report: Some(report), .. } = &preview.outcome else { panic!("School previews a change with its report") };
        assert!(!report.applied.is_empty(), "{report:?}");
        assert_preview_matches_apply(&r, Choice::Template(Template::School));
    }

    #[test]
    fn a_template_that_does_not_fit_previews_its_reason() {
        let r = rig_of(&street(&NARROW), Some(LENGTH), Locale::En);
        let v = r.vm.view_now();
        assert!(v.row_mm < Template::Commercial.min_mm(), "{}", v.row_mm);
        let refusal = v.templates.iter().find(|t| t.template == Template::Commercial).unwrap().refusal.unwrap();
        assert!(matches!(refusal, TemplateRefusal::TooNarrow { .. }), "{refusal:?}");
        let preview = r.vm.preview(Choice::Template(Template::Commercial));
        assert!(!preview.can_apply());
        let Outcome::Refused(said) = preview.outcome else { panic!("Commercial does not fit") };
        assert_eq!(said, refusal.said(Template::Commercial));
        let before = r.vm.read(|e| e.snapshot());
        assert!(!r.vm.apply_choice(Choice::Template(Template::Commercial)));
        assert_eq!(r.vm.read(|e| e.snapshot()), before);
    }

    #[test]
    fn the_preview_draws_no_selected_piece() {
        let r = rig(0);
        let uid = r.vm.view_now().segments[0].uid;
        r.vm.edit(|e| {
            e.select(Some(uid));
            true
        });
        assert_eq!(r.vm.view_now().selected, Some(uid));
        let preview = r.vm.preview(Choice::Pedestrian(sidewalk()));
        assert_eq!(preview.before.selected, None);
        let Outcome::Changed { after, .. } = preview.outcome else { panic!("the sample can be made pedestrian") };
        assert_eq!(after.selected, None);
        assert_eq!(r.vm.view_now().selected, Some(uid), "the page's selection stays");
    }

    #[test]
    fn applying_a_choice_is_one_step_that_undo_reverses() {
        let r = rig(0);
        let s0 = r.vm.read(|e| e.snapshot());
        let code = r.vm.transform_choices().transit[0].1[0];
        assert!(r.vm.apply_choice(Choice::Measure(code)));
        assert_ne!(r.vm.read(|e| e.snapshot()), s0);
        assert!(!r.said.take().is_empty(), "announced as today");
        assert!(r.vm.undo());
        // the uid counter is not taken back, so a later piece never reuses an undone one's uid
        let s1 = r.vm.read(|e| e.snapshot());
        assert_eq!(Street { next_uid: s0.next_uid, ..s1 }, s0);
        assert!(!r.vm.view_now().can_undo, "one step");
    }
}
