//! A change to several selected streets at once: reviewed first, then made on the streets that can take it. Nothing
//! here decides a rule: each street's own editor does (`Editor::apply_template`, `Editor::make_pedestrian`), so a
//! street the change does not fit says why and is left exactly as it is. Pure over a city and its store, no wording
//! (a reason is `Said` data).

use crate::city::model::City;
use crate::city::store::CityStore;
use crate::shared::catalogue::{PEDESTRIAN_KINDS, kind_index};
use crate::shared::said::{Arg, Said};
use crate::street::model::{Editor, Street, Template};

/// What can be done to several streets: a template of the Transform menu, or making them pedestrian.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Template(Template),
    Pedestrian,
}

impl Op {
    /// The operations, in the order the panel lists them.
    pub const ALL: [Op; 6] = [
        Op::Template(Template::Commercial),
        Op::Template(Template::School),
        Op::Template(Template::Greenway),
        Op::Template(Template::BusCorridor),
        Op::Template(Template::Industrial),
        Op::Pedestrian,
    ];

    /// The id the page uses to tell the operations apart.
    pub fn id(self) -> &'static str {
        match self {
            Op::Template(t) => t.id(),
            Op::Pedestrian => "pedestrian",
        }
    }

    pub fn from_id(id: &str) -> Option<Op> {
        Op::ALL.into_iter().find(|o| o.id() == id)
    }

    /// What it is called, as the street page's Transform menu calls it.
    pub fn name(self) -> Said {
        match self {
            Op::Template(t) => Said::new(t.name_key()),
            Op::Pedestrian => Said::new("make-pedestrian-button"),
        }
    }

    /// Makes the change on one street's editor, or says why it cannot be made.
    fn make(self, editor: &mut Editor) -> Result<(), Said> {
        match self {
            Op::Template(t) => editor.apply_template(t).map(|_| ()).map_err(|refusal| refusal.said(t)),
            Op::Pedestrian => {
                let made = kind_index(PEDESTRIAN_KINDS[0]).is_some_and(|kind| editor.make_pedestrian(kind));
                if made { Ok(()) } else { Err(Said::new("corridor-refused-no-motor")) }
            }
        }
    }
}

/// What the change would do to one street.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// The street can take it.
    Ready,
    /// The street cannot, for this reason, and is left as it is.
    Refused(Said),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub street: u32,
    pub outcome: Outcome,
}

/// The review: every selected street, in the order selected, and whether it can take the change.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub op: Op,
    pub rows: Vec<Row>,
}

impl Plan {
    pub fn ready(&self) -> Vec<u32> {
        self.rows.iter().filter(|r| r.outcome == Outcome::Ready).map(|r| r.street).collect()
    }

    pub fn refused(&self) -> Vec<(u32, Said)> {
        self.rows
            .iter()
            .filter_map(|r| match &r.outcome {
                Outcome::Refused(why) => Some((r.street, why.clone())),
                Outcome::Ready => None,
            })
            .collect()
    }
}

/// The street's editor with the change made on it, or why not: nothing is kept.
fn try_street(city: &City, region: usize, op: Op, street: u32) -> Result<Editor, Said> {
    let mut editor = city.street_editor(street, region).ok_or_else(|| Said::new("corridor-refused-gone"))?;
    op.make(&mut editor)?;
    Ok(editor)
}

/// Tries the change on every street in `streets`, on copies: nothing is changed.
pub fn plan(city: &City, region: usize, op: Op, streets: &[u32]) -> Plan {
    let rows = streets
        .iter()
        .map(|&street| {
            let outcome = match try_street(city, region, op, street) {
                Ok(_) => Outcome::Ready,
                Err(why) => Outcome::Refused(why),
            };
            Row { street, outcome }
        })
        .collect();
    Plan { op, rows }
}

/// What was done.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub op: Op,
    /// The streets changed and kept.
    pub changed: Vec<u32>,
    /// The streets that could not take it, left as they were, each with the reason.
    pub refused: Vec<(u32, Said)>,
    /// The streets changed but not kept (the city would not store them): left as they were.
    pub not_kept: Vec<u32>,
    /// How to take the change back: each street changed, as it was and as it was left.
    pub undo: Undo,
}

/// The streets a change made, as they were before it and as it left them: enough to take it back as one step, so
/// long as none has been edited since.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Undo {
    pub before: Vec<(u32, Street)>,
    pub after: Vec<(u32, Street)>,
}

/// What taking a change back did.
#[derive(Clone, Debug, PartialEq)]
pub enum Undone {
    /// Every street is as it was before the change.
    Restored(Vec<u32>),
    /// These streets were edited since the change; nothing was restored.
    Refused(Vec<u32>),
    /// These streets could not be written back; the others were restored.
    NotKept(Vec<u32>),
}

impl Report {
    /// The report as a sentence a screen reader hears, as `Said` data.
    pub fn said(&self) -> Said {
        let said = |key: &'static str| {
            Said::new(key)
                .with("changed", Arg::Num(self.changed.len() as i64))
                .with("left", Arg::Num(self.refused.len() as i64))
                .with("name", Arg::Said(Box::new(self.op.name())))
        };
        if self.not_kept.is_empty() { said("corridor-done") } else { said("corridor-done-failed").with("failed", Arg::Num(self.not_kept.len() as i64)) }
    }
}

/// Makes the change on every street of `streets` that can take it, and keeps each in `store`; the others are not
/// touched. Reviewed again against the city as stored now, since it may have changed since the review.
pub fn apply(store: &CityStore, op: Op, streets: &[u32]) -> Report {
    let region = store.region();
    let city = store.open();
    let (mut changed, mut refused, mut not_kept, mut undo) = (Vec::new(), Vec::new(), Vec::new(), Undo::default());
    for &street in streets {
        match try_street(&city, region, op, street) {
            Err(why) => refused.push((street, why)),
            Ok(editor) => {
                let after = editor.snapshot();
                let before = city.street_editor(street, region).map(|e| e.snapshot());
                match before {
                    Some(before) if store.write_street(street, after.clone()) => {
                        changed.push(street);
                        undo.before.push((street, before));
                        undo.after.push((street, after));
                    }
                    _ => not_kept.push(street),
                }
            }
        }
    }
    Report { op, changed, refused, not_kept, undo }
}

/// Takes a change back, all of it or none: when any street was edited since (it is not as the change left it), no
/// street is touched. Each street is written back as it was.
pub fn undo(store: &CityStore, undo: &Undo) -> Undone {
    let region = store.region();
    let city = store.open();
    let edited: Vec<u32> = undo
        .after
        .iter()
        .filter(|(street, after)| city.street_editor(*street, region).map(|e| e.snapshot()).as_ref() != Some(after))
        .map(|(street, _)| *street)
        .collect();
    if !edited.is_empty() {
        return Undone::Refused(edited);
    }
    let (mut restored, mut stuck) = (Vec::new(), Vec::new());
    for (street, before) in &undo.before {
        if store.write_street(*street, before.clone()) {
            restored.push(*street);
        } else {
            stuck.push(*street);
        }
    }
    if stuck.is_empty() { Undone::Restored(restored) } else { Undone::NotKept(stuck) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::MemoryStorage;
    use std::rc::Rc;

    fn store() -> CityStore {
        CityStore::sample(Rc::new(MemoryStorage::default()))
    }

    /// A street of the sample city that takes the Greenway, and the freeway, which takes none.
    const FREEWAY: u32 = 23;

    fn takes_greenway(store: &CityStore) -> u32 {
        let city = store.open();
        (1..=22u32)
            .find(|&s| plan(&city, 0, Op::Template(Template::Greenway), &[s]).rows[0].outcome == Outcome::Ready)
            .expect("a street of the sample takes the greenway")
    }

    #[test]
    fn the_review_says_for_each_selected_street_whether_it_can_take_the_change_and_changes_nothing() {
        let store = store();
        let ok = takes_greenway(&store);
        let before = store.open().street_editor(ok, 0).unwrap().snapshot();
        let plan = plan(&store.open(), 0, Op::Template(Template::Greenway), &[FREEWAY, ok, 999]);
        assert_eq!(plan.ready(), vec![ok]);
        assert_eq!(plan.rows.iter().map(|r| r.street).collect::<Vec<_>>(), vec![FREEWAY, ok, 999], "in the order selected");
        let refused = plan.refused();
        assert_eq!(refused[0].0, FREEWAY);
        assert_eq!(refused[0].1.key, "template-refused-freeway");
        assert_eq!(refused[1], (999, Said::new("corridor-refused-gone")));
        assert_eq!(store.open().street_editor(ok, 0).unwrap().snapshot(), before, "a review writes nothing");
    }

    #[test]
    fn applying_changes_the_streets_that_take_it_keeps_them_and_leaves_the_rest_exactly_as_they_were() {
        let store = store();
        let ok = takes_greenway(&store);
        let free_before = store.open().street_editor(FREEWAY, 0).unwrap().snapshot();
        let before = store.open().street_editor(ok, 0).unwrap().snapshot();
        let report = apply(&store, Op::Template(Template::Greenway), &[ok, FREEWAY]);
        assert_eq!(report.changed, vec![ok]);
        assert_eq!(report.refused.len(), 1);
        assert_eq!(report.refused[0].0, FREEWAY);
        assert!(report.not_kept.is_empty());
        let after = store.open().street_editor(ok, 0).unwrap().snapshot();
        assert_ne!(after, before, "the street was changed and kept");
        assert_eq!(store.open().street_editor(FREEWAY, 0).unwrap().snapshot(), free_before, "the refused street is not touched");
    }

    #[test]
    fn making_pedestrian_is_refused_for_a_street_with_nothing_motor_and_the_report_counts_both() {
        let store = store();
        let ok = takes_greenway(&store);
        let first = apply(&store, Op::Pedestrian, &[ok]);
        assert_eq!(first.changed, vec![ok]);
        // now it has no motor piece: a second time is refused, not silently done again
        let again = apply(&store, Op::Pedestrian, &[ok]);
        assert!(again.changed.is_empty());
        assert_eq!(again.refused, vec![(ok, Said::new("corridor-refused-no-motor"))]);
        let mixed = apply(&store, Op::Pedestrian, &[ok, 999]);
        assert_eq!(
            mixed.said(),
            Said::new("corridor-done")
                .with("changed", Arg::Num(0))
                .with("left", Arg::Num(2))
                .with("name", Arg::Said(Box::new(Said::new("make-pedestrian-button"))))
        );
    }

    #[test]
    fn every_operation_has_a_distinct_id_that_reads_back() {
        let ids: Vec<&str> = Op::ALL.iter().map(|o| o.id()).collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), ids.len());
        for op in Op::ALL {
            assert_eq!(Op::from_id(op.id()), Some(op));
        }
        assert_eq!(Op::from_id("nonsense"), None);
    }

    fn two_that_take_greenway(store: &CityStore) -> [u32; 2] {
        let city = store.open();
        let ready: Vec<u32> = (1..=22u32).filter(|&s| plan(&city, 0, Op::Template(Template::Greenway), &[s]).ready() == vec![s]).take(2).collect();
        [ready[0], ready[1]]
    }

    fn stored(store: &CityStore, street: u32) -> crate::street::model::Street {
        store.open().street_editor(street, 0).unwrap().snapshot()
    }

    #[test]
    fn what_was_changed_can_be_undone_as_one_step_and_every_street_comes_back() {
        let store = store();
        let [a, b] = two_that_take_greenway(&store);
        let (before_a, before_b) = (stored(&store, a), stored(&store, b));
        let report = apply(&store, Op::Template(Template::Greenway), &[a, FREEWAY, b]);
        assert_eq!(report.changed, vec![a, b]);
        assert_eq!(report.undo.before.iter().map(|(s, _)| *s).collect::<Vec<_>>(), vec![a, b], "only the streets changed are remembered");
        assert_ne!(stored(&store, a), before_a);
        assert_eq!(undo(&store, &report.undo), Undone::Restored(vec![a, b]));
        assert_eq!((stored(&store, a), stored(&store, b)), (before_a, before_b));
    }

    #[test]
    fn nothing_is_undone_when_a_street_was_edited_since_and_it_says_which() {
        let store = store();
        let [a, b] = two_that_take_greenway(&store);
        let report = apply(&store, Op::Template(Template::Greenway), &[a, b]);
        // the second street is edited on its own page afterwards
        let mut editor = store.open().street_editor(b, 0).unwrap();
        let u = editor.view().segments[0].uid;
        assert!(editor.set_width(u, editor.view().segments[0].width_mm + 100));
        assert!(store.write_street(b, editor.snapshot()));
        let (now_a, now_b) = (stored(&store, a), stored(&store, b));
        assert_eq!(undo(&store, &report.undo), Undone::Refused(vec![b]));
        assert_eq!((stored(&store, a), stored(&store, b)), (now_a, now_b), "not even the street left alone is restored: it is one step");
    }
}
