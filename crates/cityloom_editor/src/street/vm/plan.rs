//! A press on the street's plan, and what it becomes: a click, a drag that picks a stretch, a drag of one end of the
//! pick, of a boundary between runs or of a thing along the street. Pure decisions over what the view measures (the
//! pointer's pixels for the drag threshold, its position along the street from the plan's left end); `StreetVm`'s
//! plan commands, below them, carry them out with the keys (`keys::on_plan`) and say what was picked.

use leptos::prelude::*;

use crate::shared::catalogue::Side;
use crate::shared::said::{Said, say, say_now};
use crate::street::model::{Editor, MIN_ZONE_MM, Pick, PickRefusal, PlanThing, ZoneEnd};
use crate::street::vm::keys::PlanKey;
use crate::street::vm::row::{CurbRow, StopRow};
use crate::street::vm::{StreetVm, text};

/// A press becomes a drag only after it has moved this far.
const DRAG_START_PX: f64 = 4.0;

/// Whether a press has moved far enough to be a drag.
pub fn is_boundary_drag(dx_px: f64) -> bool {
    dx_px.abs() >= DRAG_START_PX
}

/// What a press on the plan landed on. Positions count from the plan's left end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanPress {
    /// The street itself (an empty part of a band), at `at_mm`.
    Street { at_mm: i32 },
    /// A handle of the pick: its left one, or its right one.
    Handle { left: bool },
    /// The marker of the boundary that starts zone `zone` (`PlanBoundary.zone`).
    Boundary { zone: usize },
    /// A thing along the street, pressed at `at_mm`.
    Thing { thing: PlanThing, at_mm: i32 },
}

/// A press under way. Positions count from the plan's left end, but a thing's are anchored, as its row keeps them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlanGesture {
    /// Pressed at `x_px` on the screen and not yet moved past the threshold: a click if it is let go now.
    Pending(PlanPress, f64),
    /// A drag picking from `anchor_mm`, where it started.
    Picking { anchor_mm: i32 },
    /// A drag of one end of the pick; the other end stays at `fixed_mm`.
    Handle { fixed_mm: i32 },
    /// A drag of the boundary that starts zone `zone`, from `at_mm`, begun with `pick` (`None` for the whole street):
    /// the pick stays as it is while the drag lasts, and an end of it that was on the boundary follows it when it ends.
    Boundary { zone: usize, at_mm: i32, pick: Option<(i32, i32)> },
    /// A drag of a thing that started at `from_mm`, pressed at `grab_mm` (both anchored).
    Thing { thing: PlanThing, from_mm: i32, grab_mm: i32 },
    /// A drag that was refused when it would have begun (another drag under way, or nothing may move): nothing until
    /// it is let go.
    Refused,
}

/// The press, before it has moved.
pub fn press(p: PlanPress, x_px: f64) -> PlanGesture {
    PlanGesture::Pending(p, x_px)
}

/// Whether the gesture is a drag with the pointer at `x_px`: a press that has moved past the threshold, or a drag
/// already begun.
pub fn moved(g: &PlanGesture, x_px: f64) -> bool {
    match g {
        PlanGesture::Pending(_, from_px) => is_boundary_drag(x_px - from_px),
        _ => true,
    }
}

/// The end of the pick a handle drag leaves where it is: the right end for the left handle, else the left end. `pick`
/// is the picked stretch, `None` for the whole street, `length_mm` the street's.
pub fn fixed_end(left: bool, pick: Option<(i32, i32)>, length_mm: i32) -> i32 {
    let (from, to) = pick.unwrap_or((0, length_mm));
    if left { to } else { from }
}

/// The pick after the boundary at `from_mm` has moved to `to_mm`: an end of `pick` that was on the boundary follows
/// it; the whole street stays the whole street.
pub fn pick_after_move(pick: Option<(i32, i32)>, from_mm: i32, to_mm: i32) -> Option<(i32, i32)> {
    pick.map(|(a, b)| (if a == from_mm { to_mm } else { a }, if b == from_mm { to_mm } else { b }))
}

/// The picking commands: positions here count from the plan's left end (the street's first end) and are converted with
/// `Editor::anchored_mm` at the editor's edge.
impl StreetVm {
    /// The picked stretch from the plan's left end, `None` for the whole street. Watched.
    pub fn picked(&self) -> Option<(i32, i32)> {
        self.view().plan.pick
    }

    /// The plan's keyboard point, from its left end. Watched.
    pub fn plan_point(&self) -> i32 {
        self.point.get()
    }

    /// The words of the plan's point (its `aria-valuetext`): where it is while it has moved on its own since the stretch
    /// was last picked (the arrows alone, or an extension still too short to pick), else the pick as it is said when
    /// made. Watched.
    pub fn plan_point_text(&self) -> String {
        let units = self.units();
        if self.point_alone.get() {
            return say(&self.i18n, units, &text::point_said(self.point.get()));
        }
        let plan = self.view().plan.clone();
        let what = self.read(|e| e.pick_differs());
        text::picked_said(&plan, what).map(|said| say(&self.i18n, units, &said)).unwrap_or_default()
    }

    /// Picks the stretch between `a_mm` and `b_mm` (from the plan's left end, either order), saying what was picked or
    /// why not.
    pub fn pick_stretch(&self, a_mm: i32, b_mm: i32) -> bool {
        self.extend_from.set(None);
        self.pick_between(a_mm, b_mm)
    }

    /// Picks the run at `at_mm` (from the plan's left end), saying what was picked or why not.
    pub fn pick_run_at(&self, at_mm: i32) {
        self.extend_from.set(None);
        self.pick_with(|e| {
            let at = e.anchored_mm(at_mm);
            e.pick_run_at(at)
        });
    }

    /// Picks the whole street, saying so.
    pub fn pick_whole(&self) {
        self.extend_from.set(None);
        self.pick_with(|e| {
            if e.dragging() {
                return Err(PickRefusal::Dragging);
            }
            e.pick_whole();
            Ok(e.pick())
        });
    }

    /// What a key on the plan's point does (`keys::on_plan`).
    pub fn plan_key(&self, k: PlanKey) {
        let length = self.view_now().plan.length_mm.unwrap_or(0);
        let point = self.point.get_untracked();
        let along = |d: i32| point.saturating_add(d).clamp(0, length);
        let end = |last: bool| if last { length } else { 0 };
        match k {
            PlanKey::Move(d) => self.move_point(along(d)),
            PlanKey::To(last) => self.move_point(end(last)),
            PlanKey::Extend(d) => self.extend_pick(along(d)),
            PlanKey::ExtendTo(last) => self.extend_pick(end(last)),
            PlanKey::PickRun => self.pick_run_at(point),
            PlanKey::PickWhole => self.pick_whole(),
            PlanKey::NudgeBoundary(d) => self.nudge_pick_boundary(d),
        }
    }

    /// A press on the plan: a thing pressed is chosen at once, as on its row. The view keeps the gesture.
    pub fn plan_press(&self, p: PlanPress, x_px: f64) -> PlanGesture {
        if let PlanPress::Thing { thing, .. } = p {
            match thing {
                PlanThing::Curb { side, cell } => self.select_cell::<CurbRow>(side, cell),
                PlanThing::Stop { side, cell } => self.select_cell::<StopRow>(side, cell),
                PlanThing::Calming { cell } => self.select_calming(cell),
                PlanThing::Green { i } => self.select_green(i),
            }
        }
        press(p, x_px)
    }

    /// The pointer moved to `x_px`, `at_mm` from the plan's left end: past the threshold the press becomes a drag, and a
    /// drag of a boundary or a thing follows the pointer (a drag picking a stretch picks when it ends).
    pub fn plan_move(&self, g: &PlanGesture, x_px: f64, at_mm: i32) -> PlanGesture {
        let g = match *g {
            PlanGesture::Pending(p, _) if moved(g, x_px) => self.plan_begin(p),
            g => g,
        };
        self.plan_step(&g, at_mm);
        g
    }

    /// The pointer was let go at `at_mm`: a press that did not move is a click (on the street, it picks the run there),
    /// a drag ends (picking from where it started, or keeping what it moved as one edit).
    pub fn plan_release(&self, g: PlanGesture, at_mm: i32) {
        match g {
            PlanGesture::Pending(PlanPress::Street { at_mm: pressed }, _) => self.pick_run_at(pressed),
            PlanGesture::Pending(..) | PlanGesture::Refused => {}
            PlanGesture::Picking { anchor_mm } => {
                self.pick_stretch(anchor_mm, at_mm);
            }
            PlanGesture::Handle { fixed_mm } => {
                self.pick_stretch(fixed_mm, at_mm);
            }
            PlanGesture::Boundary { zone, at_mm: was, pick } => {
                self.plan_step(&g, at_mm);
                self.end_boundary_drag(true);
                let now = self.boundary_at(zone).unwrap_or(was);
                self.repick(pick_after_move(pick, was, now));
            }
            PlanGesture::Thing { thing, .. } => {
                self.plan_step(&g, at_mm);
                self.end_thing_drag(thing, true);
            }
        }
    }

    /// The gesture is given up (Escape, or the pointer lost): a drag puts back what it moved.
    pub fn plan_cancel(&self, g: PlanGesture) {
        match g {
            PlanGesture::Boundary { pick, .. } => {
                self.end_boundary_drag(false);
                self.repick(pick);
            }
            PlanGesture::Thing { thing, .. } => self.end_thing_drag(thing, false),
            _ => {}
        }
    }

    /// A press that has become a drag: what it drags. A drag that cannot begin says why and does nothing.
    fn plan_begin(&self, p: PlanPress) -> PlanGesture {
        match p {
            PlanPress::Street { at_mm } => PlanGesture::Picking { anchor_mm: at_mm },
            PlanPress::Handle { left } => {
                let plan = &self.view_now().plan;
                PlanGesture::Handle { fixed_mm: fixed_end(left, plan.pick, plan.length_mm.unwrap_or(0)) }
            }
            PlanPress::Boundary { zone } => {
                if let Some(refusal) = self.read(|e| e.structure_blocked()) {
                    self.ports.announcer.say(&text::zone_refused(&self.i18n, self.units_now(), refusal));
                    return PlanGesture::Refused;
                }
                let Some(at_mm) = self.boundary_at(zone) else { return PlanGesture::Refused };
                let pick = self.view_now().plan.pick;
                if !self.begin_boundary_drag() {
                    return PlanGesture::Refused;
                }
                PlanGesture::Boundary { zone, at_mm, pick }
            }
            PlanPress::Thing { thing, at_mm } => {
                let v = self.view_now();
                let from_mm = match thing {
                    PlanThing::Curb { side, cell } => v.curb.sides.iter().find(|s| s.side == side).and_then(|s| s.cells.get(cell)).map(|c| c.from_mm),
                    PlanThing::Stop { side, cell } => v.stops.sides.iter().find(|s| s.side == side).and_then(|s| s.cells.get(cell)).map(|c| c.from_mm),
                    PlanThing::Calming { cell } => v.calming_strip.cells.get(cell).map(|c| c.from_mm),
                    PlanThing::Green { i } => v.green.cells.get(i).map(|c| c.from_mm),
                };
                let Some(from_mm) = from_mm else { return PlanGesture::Refused };
                let begun = match thing {
                    PlanThing::Curb { .. } => self.begin_curb_drag(),
                    PlanThing::Stop { .. } => self.begin_stop_drag(),
                    PlanThing::Calming { .. } => self.begin_calming_drag(),
                    PlanThing::Green { .. } => self.begin_green_drag(),
                };
                if !begun {
                    return PlanGesture::Refused;
                }
                PlanGesture::Thing { thing, from_mm, grab_mm: self.read(|e| e.anchored_mm(at_mm)) }
            }
        }
    }

    /// One step of a drag of a boundary or a thing, the pointer at `at_mm` from the plan's left end: shown, not said.
    fn plan_step(&self, g: &PlanGesture, at_mm: i32) {
        let at = self.read(|e| e.anchored_mm(at_mm));
        match *g {
            PlanGesture::Boundary { zone, .. } => {
                self.quiet(|e| e.move_boundary_of(zone, at).is_ok());
            }
            PlanGesture::Thing { thing, from_mm, grab_mm } => {
                let to = from_mm + at - grab_mm;
                match thing {
                    PlanThing::Curb { .. } => self.drag_curb_to(to),
                    PlanThing::Stop { .. } => self.drag_stop_to(to),
                    PlanThing::Calming { .. } => self.drag_calming_to(to),
                    PlanThing::Green { .. } => self.drag_green_to(to),
                }
            }
            _ => {}
        }
    }

    fn end_thing_drag(&self, thing: PlanThing, keep: bool) {
        match thing {
            PlanThing::Curb { .. } | PlanThing::Stop { .. } => self.end_cell_drag(keep),
            PlanThing::Calming { .. } => self.end_calming_drag(keep),
            PlanThing::Green { .. } => self.end_green_drag(keep),
        }
    }

    /// Where the boundary that starts zone `zone` lies on the plan.
    fn boundary_at(&self, zone: usize) -> Option<i32> {
        self.view_now().plan.boundaries.iter().find(|b| b.zone == zone).map(|b| b.at_mm)
    }

    /// Moves the keyboard point; an extension of the pick starts again from where it lands.
    fn move_point(&self, to_mm: i32) {
        self.extend_from.set(None);
        self.point.set(to_mm);
        self.point_alone.set(true);
    }

    /// Moves the keyboard point to `to_mm`, picking from where the extension started once that is at least
    /// `MIN_ZONE_MM` away (until then nothing is picked or said: the keys are on their way).
    fn extend_pick(&self, to_mm: i32) {
        let from = self.extend_from.get().unwrap_or_else(|| self.point.get_untracked());
        self.extend_from.set(Some(from));
        self.point.set(to_mm);
        if (to_mm - from).abs() < MIN_ZONE_MM || !self.pick_between(from, to_mm) {
            self.point_alone.set(true);
        }
    }

    fn pick_between(&self, a_mm: i32, b_mm: i32) -> bool {
        self.pick_with(|e| {
            let (a, b) = (e.anchored_mm(a_mm), e.anchored_mm(b_mm));
            e.pick_stretch(a, b)
        })
    }

    /// Picks as `f` does and says what was picked, or why not. A stretch picked chooses, on each row along the street,
    /// the stretch at its start (on the side already chosen, else the side traffic keeps), so that what the Add menu
    /// adds there lands at the start of the pick.
    fn pick_with(&self, f: impl FnOnce(&mut Editor) -> Result<Pick, PickRefusal>) -> bool {
        let v = self.view_now();
        let side_of = |sides: &[(Side, bool)]| sides.iter().find(|s| s.1).or_else(|| sides.iter().find(|s| s.0 == v.side)).or(sides.first()).map(|s| s.0);
        let curb = side_of(&v.curb.sides.iter().map(|s| (s.side, s.selected.is_some())).collect::<Vec<_>>());
        let stop = side_of(&v.stops.sides.iter().map(|s| (s.side, s.selected.is_some())).collect::<Vec<_>>());
        let calming = !v.calming_strip.cells.is_empty();
        let picked = self.edit(|e| {
            let picked = f(e);
            if let Ok(Pick::Stretch { from_mm, to_mm }) = picked {
                // The stretch's left end, inside it.
                let at = if e.anchor_end() == ZoneEnd::First { from_mm } else { to_mm - 1 };
                if let Some(side) = curb {
                    e.select_curb(side, at);
                }
                if let Some(side) = stop {
                    e.select_stop(side, at);
                }
                if calming {
                    e.select_calming_at(at);
                }
            }
            Some(picked)
        });
        // None only when the pick reached the street's history on a street shown read-only, which was refused and said.
        let Some(picked) = picked else { return false };
        match picked {
            Ok(_) => {
                self.point_alone.set(false);
                if let Some(said) = self.pick_text() {
                    self.ports.announcer.say(&said);
                }
                true
            }
            Err(refusal) => {
                self.ports.announcer.say(&say_now(&self.i18n, self.units_now(), &refusal.said()));
                false
            }
        }
    }

    /// The pick in words, as said when it is made.
    fn pick_text(&self) -> Option<String> {
        let what = self.read(|e| e.pick_differs());
        text::picked_said(&self.view_now().plan, what).map(|said| say_now(&self.i18n, self.units_now(), &said))
    }

    /// `said`, and after it the pick in words when it is no longer `before` (a move through the history or a reload
    /// clamped it to the street).
    pub(super) fn with_pick_note(&self, said: String, before: Pick) -> String {
        if self.read(|e| e.pick()) == before {
            return said;
        }
        match self.pick_text() {
            Some(note) => format!("{said} {note}"),
            None => said,
        }
    }

    /// Moves the boundary at the pick's left end (else at its right end) by `delta_mm` along the plan, the pick's end
    /// following it; says why when there is none or it cannot move.
    fn nudge_pick_boundary(&self, delta_mm: i32) {
        let plan = self.view_now().plan.clone();
        let (left, right) = plan.pick.unwrap_or((0, plan.length_mm.unwrap_or(0)));
        let boundary = plan.boundaries.iter().find(|b| b.at_mm == left).or_else(|| plan.boundaries.iter().find(|b| b.at_mm == right));
        let Some(&crate::street::model::PlanBoundary { zone, at_mm: was, .. }) = boundary else {
            self.ports.announcer.say(&say_now(&self.i18n, self.units_now(), &Said::new("pick-refused-no-boundary")));
            return;
        };
        let moved = self.zone_edit(|e| {
            let to = e.anchored_mm(was + delta_mm);
            e.move_boundary_of(zone, to)
        });
        if moved {
            let now = self.boundary_at(zone).unwrap_or(was);
            self.repick(pick_after_move(plan.pick, was, now));
        }
    }

    /// Picks `pick` (from the plan's left end; `None` the whole street) again, quietly, keeping the selected piece.
    fn repick(&self, pick: Option<(i32, i32)>) {
        let selected = self.view_now().selected;
        self.edit(|e| {
            match pick {
                Some((a, b)) => {
                    let (a, b) = (e.anchored_mm(a), e.anchored_mm(b));
                    let _ = e.pick_stretch(a, b);
                }
                None => e.pick_whole(),
            }
            e.select(selected);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::Side;

    #[test]
    fn a_press_is_a_drag_only_after_a_few_pixels() {
        assert!(!is_boundary_drag(3.9) && !is_boundary_drag(-3.9));
        assert!(is_boundary_drag(4.0) && is_boundary_drag(-4.0) && is_boundary_drag(40.0));
        let g = press(PlanPress::Street { at_mm: 10_000 }, 100.0);
        assert_eq!(g, PlanGesture::Pending(PlanPress::Street { at_mm: 10_000 }, 100.0));
        assert!(!moved(&g, 103.9) && !moved(&g, 96.1));
        assert!(moved(&g, 104.0) && moved(&g, 96.0));
        assert!(moved(&PlanGesture::Picking { anchor_mm: 0 }, 100.0), "a drag begun stays one");
        let thing = press(PlanPress::Thing { thing: PlanThing::Curb { side: Side::Left, cell: 1 }, at_mm: 5_000 }, 10.0);
        assert!(!moved(&thing, 12.0));
    }

    #[test]
    fn a_handle_drag_keeps_the_other_end_and_a_boundary_drag_takes_the_pick_s_end_along() {
        assert_eq!(fixed_end(true, Some((10_000, 30_000)), 60_000), 30_000);
        assert_eq!(fixed_end(false, Some((10_000, 30_000)), 60_000), 10_000);
        assert_eq!(fixed_end(true, None, 60_000), 60_000, "the whole street's handles are at its ends");
        assert_eq!(fixed_end(false, None, 60_000), 0);
        assert_eq!(pick_after_move(Some((10_000, 30_000)), 10_000, 12_000), Some((12_000, 30_000)));
        assert_eq!(pick_after_move(Some((10_000, 30_000)), 30_000, 28_000), Some((10_000, 28_000)));
        assert_eq!(pick_after_move(Some((10_000, 30_000)), 40_000, 45_000), Some((10_000, 30_000)));
        assert_eq!(pick_after_move(None, 40_000, 45_000), None);
    }
}

#[cfg(test)]
mod pick_tests {
    use std::rc::Rc;

    use super::*;
    use crate::shared::catalogue::{REGIONS, StreetClass};
    use crate::shared::i18n::Locale;
    use crate::shared::ports::{RecordingAnnouncer, test_ports_with_time};
    use crate::street::model::{Calming, Instance, Piece, PlanThing, Street, StreetExt, ZoneEnd, ZoneFrame, zoned};
    use crate::street::model::{CalmingKind, CalmingKindExt, StopKind, Use};
    use crate::street::vm::add_menu::Item;
    use crate::street::vm::level::Level;

    const LEN: i32 = 60_000;

    struct Rig {
        vm: Rc<StreetVm>,
        said: Rc<RecordingAnnouncer>,
    }

    fn rig_of(street: &Street, from_end: ZoneEnd) -> Rig {
        let region = REGIONS.iter().position(|r| r.drive_side == Side::Right).unwrap();
        let (ports, said, _, _) = test_ports_with_time();
        let editor = Editor::from_street(street, street, region).with_frame(Some(ZoneFrame { length_mm: LEN, from_end }));
        Rig { vm: StreetVm::new(ports, crate::i18n_for(Locale::En), editor, None), said }
    }

    fn sample() -> Street {
        Street::sample(0, Side::Right)
    }

    /// The sample street divided at `starts` (anchored at `from_end`), drawn on a road `LEN` long; zone `i` has its last
    /// piece (the right sidewalk, so no taper is needed) widened by `widen[i]` mm.
    fn street(starts: &[i32], widen: &[i32], from_end: ZoneEnd) -> Street {
        let base = sample();
        let mut s = zoned(base.clone(), starts, LEN);
        for (zone, by) in s.zones.iter_mut().zip(widen) {
            if *by != 0 {
                let mut own = base.segments.clone();
                own.last_mut().expect("the sample has pieces").width_mm += by;
                zone.segments = Some(own);
            }
        }
        s.from_end = from_end;
        s.settled()
    }

    /// Three runs: 0 to 20 m, 20 m to 40 m (its right sidewalk 100 mm wider), 40 m to the end, counted from `from_end`.
    fn three_runs(from_end: ZoneEnd) -> Rig {
        rig_of(&street(&[0, 20_000, 40_000], &[0, 100, 0], from_end), from_end)
    }

    /// Each boundary's place on the plan.
    fn boundaries(r: &Rig) -> Vec<i32> {
        r.vm.view_now().plan.boundaries.iter().map(|b| b.at_mm).collect()
    }

    /// The things on the plan, `(thing, from, to)` from its left end.
    fn objects(r: &Rig) -> Vec<(PlanThing, i32, i32)> {
        r.vm.view_now().plan.objects.iter().map(|o| (o.thing, o.from_mm, o.to_mm)).collect()
    }

    /// Presses at `at_mm` (drawn at `at_mm / 100` px), moves to `to_mm` and lets go there.
    fn drag(r: &Rig, p: PlanPress, at_mm: i32, to_mm: i32) {
        let g = r.vm.plan_press(p, f64::from(at_mm) / 100.0);
        let g = r.vm.plan_move(&g, f64::from(to_mm) / 100.0, to_mm);
        r.vm.plan_release(g, to_mm);
    }

    #[test]
    fn alt_arrow_moves_the_boundary_at_the_picks_left_end_and_the_pick_follows_it() {
        for from_end in [ZoneEnd::First, ZoneEnd::Last] {
            let r = three_runs(from_end);
            r.vm.pick_run_at(30_000);
            assert_eq!(r.vm.picked(), Some((20_000, 40_000)), "{from_end:?}");
            r.said.take();
            r.vm.plan_key(PlanKey::NudgeBoundary(-1_000));
            assert_eq!(boundaries(&r), [19_000, 40_000], "{from_end:?}");
            assert_eq!(r.vm.picked(), Some((19_000, 40_000)), "{from_end:?}: the pick's left end follows its boundary");
            assert_eq!(r.said.take().len(), 1, "{from_end:?}: the move is announced");
            assert!(r.vm.view_now().can_undo);
        }
        // A pick whose left end is the street's end moves the boundary at its right end.
        let r = three_runs(ZoneEnd::First);
        r.vm.pick_stretch(0, 20_000);
        r.vm.plan_key(PlanKey::NudgeBoundary(10_000));
        assert_eq!(boundaries(&r), [30_000, 40_000]);
        assert_eq!(r.vm.picked(), Some((0, 30_000)));
        // The whole street has no boundary at either end: the reason is said and nothing moves.
        r.vm.pick_whole();
        r.said.take();
        r.vm.plan_key(PlanKey::NudgeBoundary(1_000));
        assert_eq!(boundaries(&r), [30_000, 40_000]);
        assert_eq!(r.said.take(), ["No stretch boundary at either end of the pick."]);
    }

    /// Right-hand traffic: sidewalk, a bus lane each way against the curbs, sidewalk; so each side has a stops row.
    fn bus_street() -> Street {
        let piece =
            |id: &str, width_mm, direction| Piece { kind: crate::shared::catalogue::kind_index(id).unwrap(), width_mm, direction, variants: Vec::new() };
        Street::imported(
            StreetClass::Local,
            Side::Right,
            &[piece("sidewalk", 3000, None), piece("bus", 3300, Some(1)), piece("bus", 3300, Some(0)), piece("sidewalk", 3000, None)],
        )
    }

    #[test]
    fn picking_a_stretch_chooses_the_free_stretch_at_its_start_so_adding_a_stop_lands_there() {
        let stop_mm = crate::standards::model::stop_min_mm();
        let table_mm = CalmingKind::Table.default_mm();
        for from_end in [ZoneEnd::First, ZoneEnd::Last] {
            let r = rig_of(&bus_street(), from_end);
            assert!(r.vm.pick_stretch(20_000, 40_000));
            assert!(r.vm.add_item(Item::Stop(StopKind::Curb)), "{from_end:?}");
            assert!(r.vm.add_item(Item::Calming(CalmingKind::Table)), "{from_end:?}");
            let things = objects(&r);
            assert!(
                things.iter().any(|&(t, from, to)| matches!(t, PlanThing::Stop { side: Side::Right, .. }) && (from, to) == (20_000, 20_000 + stop_mm)),
                "{from_end:?}: {things:?}"
            );
            assert!(
                things.iter().any(|&(t, from, to)| matches!(t, PlanThing::Calming { .. }) && (from, to) == (20_000, 20_000 + table_mm)),
                "{from_end:?}: {things:?}"
            );
        }
    }

    #[test]
    fn a_change_that_missed_a_stretch_says_where() {
        // The middle run has no parking on its first side.
        let mut s = zoned(sample(), &[0, 20_000, 40_000], LEN);
        let mut middle = s.segments.clone();
        middle.remove(1);
        s.zones[1].segments = Some(middle);
        let r = rig_of(&s, ZoneEnd::First);
        let parking = r.vm.view_now().segments[1].uid;
        r.vm.select(Some(parking));
        r.said.take();
        assert!(r.vm.apply(|e| e.set_width(parking, 2_600)));
        let said = r.said.take();
        assert_eq!(said.len(), 1, "one announcement: {said:?}");
        assert!(said[0].starts_with("Resize parking. "), "{said:?}");
        assert!(said[0].ends_with(" Not changed from 20.0 m to 40.0 m: that stretch has no parking."), "{said:?}");
        // An edit that reached every run says nothing more.
        let sidewalk = r.vm.view_now().segments[0].uid;
        r.vm.select(Some(sidewalk));
        r.said.take();
        assert!(r.vm.apply(|e| e.set_width(sidewalk, 3_000)));
        assert!(!r.said.take()[0].contains("Not changed"));
    }

    #[test]
    fn a_press_then_release_without_moving_picks_the_run_and_announces_it() {
        let r = three_runs(ZoneEnd::First);
        let g = r.vm.plan_press(PlanPress::Street { at_mm: 30_000 }, 300.0);
        let g = r.vm.plan_move(&g, 302.0, 30_200);
        assert!(matches!(g, PlanGesture::Pending(..)), "{g:?}: under the threshold it is still a press");
        r.vm.plan_release(g, 30_200);
        assert_eq!(r.vm.picked(), Some((20_000, 40_000)));
        assert_eq!(r.said.take(), ["From 20.0 m to 40.0 m, 20.0 m long: sidewalk on the right 3.4 m wide."]);
        // A run as long as the street is the whole street.
        let plain = rig_of(&sample(), ZoneEnd::First);
        drag(&plain, PlanPress::Street { at_mm: 30_000 }, 30_000, 30_000);
        assert_eq!(plain.vm.picked(), None);
        assert_eq!(plain.said.take(), ["The whole street, 60.0 m long."]);
    }

    #[test]
    fn a_drag_picks_from_press_to_release_in_left_positions_on_a_street_anchored_at_its_last_end() {
        let r = rig_of(&sample(), ZoneEnd::Last);
        let g = r.vm.plan_press(PlanPress::Street { at_mm: 10_000 }, 100.0);
        let g = r.vm.plan_move(&g, 300.0, 30_000);
        assert_eq!(g, PlanGesture::Picking { anchor_mm: 10_000 });
        assert_eq!(r.vm.picked(), None, "nothing is picked until the drag ends");
        r.vm.plan_release(g, 30_000);
        assert_eq!(r.vm.picked(), Some((10_000, 30_000)));
        assert_eq!(r.vm.read(|e| e.pick()), crate::street::model::Pick::Stretch { from_mm: 30_000, to_mm: 50_000 }, "anchored at the far end");
        assert_eq!(r.said.take(), ["From 10.0 m to 30.0 m, 20.0 m long."]);
        // Dragged leftwards, from 45 m to 25 m.
        drag(&r, PlanPress::Street { at_mm: 45_000 }, 45_000, 25_000);
        assert_eq!(r.vm.picked(), Some((25_000, 45_000)));
    }

    #[test]
    fn dragging_a_handle_moves_that_end_only() {
        let r = rig_of(&sample(), ZoneEnd::First);
        assert!(r.vm.pick_stretch(10_000, 30_000));
        let g = r.vm.plan_press(PlanPress::Handle { left: false }, 300.0);
        let g = r.vm.plan_move(&g, 360.0, 36_000);
        assert_eq!(g, PlanGesture::Handle { fixed_mm: 10_000 });
        r.vm.plan_release(g, 36_000);
        assert_eq!(r.vm.picked(), Some((10_000, 36_000)));
        r.said.take();
        // The left handle, let go within 5 m of the street's end, takes the pick to the end.
        drag(&r, PlanPress::Handle { left: true }, 10_000, 4_000);
        assert_eq!(r.vm.picked(), Some((0, 36_000)));
        assert_eq!(r.said.take(), ["From 0.0 m to 36.0 m, 36.0 m long."]);
        // A handle pressed and let go without moving changes nothing and says nothing.
        let g = r.vm.plan_press(PlanPress::Handle { left: true }, 0.0);
        r.vm.plan_release(g, 0);
        assert_eq!(r.vm.picked(), Some((0, 36_000)));
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn a_too_short_pick_says_why_and_keeps_the_pick() {
        let r = rig_of(&sample(), ZoneEnd::First);
        assert!(r.vm.pick_stretch(20_000, 40_000));
        r.said.take();
        assert!(!r.vm.pick_stretch(10_000, 12_000));
        assert_eq!(r.said.take(), ["A stretch must be at least 5.0 m long."]);
        assert_eq!(r.vm.picked(), Some((20_000, 40_000)));
        drag(&r, PlanPress::Street { at_mm: 10_000 }, 10_000, 13_000);
        assert_eq!(r.said.take(), ["A stretch must be at least 5.0 m long."]);
        assert_eq!(r.vm.picked(), Some((20_000, 40_000)));
    }

    #[test]
    fn shift_arrows_extend_the_pick_from_the_point_and_escape_picks_the_whole_street() {
        let r = rig_of(&sample(), ZoneEnd::First);
        r.vm.plan_key(PlanKey::To(false));
        for _ in 0..10 {
            r.vm.plan_key(PlanKey::Move(1_000));
        }
        assert_eq!(r.vm.plan_point(), 10_000);
        assert!(r.said.take().is_empty(), "moving the point picks nothing");
        for _ in 0..10 {
            r.vm.plan_key(PlanKey::Extend(1_000));
        }
        assert_eq!(r.vm.plan_point(), 20_000);
        assert_eq!(r.vm.picked(), Some((10_000, 20_000)));
        assert_eq!(r.said.take().last().map(String::as_str), Some("From 10.0 m to 20.0 m, 10.0 m long."));
        for _ in 0..3 {
            r.vm.plan_key(PlanKey::Extend(-1_000));
        }
        assert_eq!(r.vm.picked(), Some((10_000, 17_000)), "from where the extension started");
        // Moving the point starts the next extension from where it lands.
        r.vm.plan_key(PlanKey::Move(5_000));
        r.vm.plan_key(PlanKey::Extend(10_000));
        assert_eq!(r.vm.picked(), Some((22_000, 32_000)));
        r.vm.plan_key(PlanKey::ExtendTo(true));
        assert_eq!((r.vm.picked(), r.vm.plan_point()), (Some((22_000, LEN)), LEN));
        r.vm.plan_key(PlanKey::Move(1_000));
        assert_eq!(r.vm.plan_point(), LEN, "the point stays on the street");
        r.said.take();
        r.vm.plan_key(PlanKey::PickWhole);
        assert_eq!(r.vm.picked(), None);
        assert_eq!(r.said.take(), ["The whole street, 60.0 m long."]);
        // Anchored at the last end, the keys still go left to right on the plan.
        let r = rig_of(&sample(), ZoneEnd::Last);
        r.vm.plan_key(PlanKey::Move(10_000));
        r.vm.plan_key(PlanKey::Extend(10_000));
        assert_eq!(r.vm.picked(), Some((10_000, 20_000)));
        assert_eq!(r.vm.read(|e| e.pick()), crate::street::model::Pick::Stretch { from_mm: 40_000, to_mm: 50_000 });
        r.vm.plan_key(PlanKey::PickRun);
        assert_eq!(r.vm.picked(), None, "the run at the point is the whole street");
    }

    #[test]
    fn pressing_a_thing_selects_its_cell_and_opens_its_level() {
        let s = Street {
            from_end: ZoneEnd::First,
            length_mm: Some(LEN),
            curb: vec![Instance { side: Side::Left, from_mm: 10_000, length_mm: 6_000, use_: Use::Parking }],
            calming: vec![Calming { from_mm: 30_000, length_mm: 7_000, kind: CalmingKind::Table, first: None }],
            ..sample()
        };
        let r = rig_of(&s, ZoneEnd::First);
        assert_eq!(r.vm.level(), Level::Street);
        let things = objects(&r);
        let (parking, calming) = (things[0].0, things[1].0);
        let PlanThing::Curb { side: Side::Left, cell } = parking else { panic!("{things:?}") };
        let g = r.vm.plan_press(PlanPress::Thing { thing: parking, at_mm: 12_000 }, 120.0);
        assert_eq!(r.vm.level(), Level::Curb);
        assert_eq!(r.vm.view_now().curb.sides.iter().find(|s| s.side == Side::Left).and_then(|s| s.selected), Some(cell));
        assert_eq!(r.said.take().len(), 1, "the chosen use is announced, as on its row");
        r.vm.plan_release(g, 12_000);
        assert!(r.said.take().is_empty(), "a click chooses, nothing more");
        r.vm.plan_press(PlanPress::Thing { thing: calming, at_mm: 31_000 }, 310.0);
        assert_eq!(r.vm.level(), Level::Calming);
        assert!(r.vm.view_now().calming_strip.selected.is_some());
        // A drag of a thing moves it through its row's drag, one undoable step.
        let revisions = r.vm.view_now().revisions.len();
        drag(&r, PlanPress::Thing { thing: parking, at_mm: 12_000 }, 12_000, 15_000);
        assert_eq!(objects(&r)[0].1..objects(&r)[0].2, 13_000..19_000);
        assert_eq!(r.vm.view_now().revisions.len(), revisions + 1);
        // Anchored at the last end, the use lies at 44 m to 50 m on the plan and moves rightward with the pointer.
        let r = rig_of(&Street { from_end: ZoneEnd::Last, ..s }, ZoneEnd::Last);
        let (parking, from, to) = objects(&r).into_iter().find(|o| matches!(o.0, PlanThing::Curb { .. })).unwrap();
        assert_eq!((from, to), (44_000, 50_000));
        drag(&r, PlanPress::Thing { thing: parking, at_mm: 45_000 }, 45_000, 48_000);
        let (_, from, to) = objects(&r).into_iter().find(|o| matches!(o.0, PlanThing::Curb { .. })).unwrap();
        assert_eq!((from, to), (47_000, 53_000));
        assert_eq!(r.vm.read(|e| e.snapshot().curb[0].from_mm), 7_000, "anchored, from the last end");
    }

    #[test]
    fn the_street_step_of_the_path_picks_the_whole_street() {
        let r = rig_of(&sample(), ZoneEnd::First);
        assert!(r.vm.pick_stretch(20_000, 40_000));
        r.said.take();
        r.vm.show_level(Level::Street);
        assert_eq!(r.vm.picked(), None);
        assert_eq!(r.said.take(), ["The whole street, 60.0 m long."]);
    }

    #[test]
    fn undo_and_redo_say_so_when_they_moved_the_pick() {
        let r = rig_of(&sample(), ZoneEnd::First);
        assert!(r.vm.pick_stretch(10_000, 30_000));
        let uid = r.vm.view_now().segments[0].uid;
        assert!(r.vm.apply(|e| e.set_width(uid, 4_000)));
        r.said.take();
        assert!(r.vm.undo());
        assert!(!r.said.take()[0].contains("From"), "the pick did not move");
        assert!(r.vm.pick_stretch(12_000, 50_000));
        r.said.take();
        // The boundary at 10 m comes back within 5 m of the pick's start, which snaps to it.
        assert!(r.vm.redo());
        assert_eq!(r.vm.picked(), Some((10_000, 50_000)));
        let said = r.said.take();
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].starts_with("Redone."), "{said:?}");
        assert!(said[0].ends_with(" From 10.0 m to 50.0 m, 40.0 m long."), "{said:?}");
    }

    #[test]
    fn an_edit_with_nothing_selected_that_missed_a_stretch_still_says_where() {
        let mut s = zoned(sample(), &[0, 20_000, 40_000], LEN);
        let mut middle = s.segments.clone();
        middle.remove(1);
        s.zones[1].segments = Some(middle);
        let r = rig_of(&s, ZoneEnd::First);
        let parking = r.vm.view_now().segments[1].uid;
        assert_eq!(r.vm.view_now().selected, None);
        assert!(r.vm.apply(|e| e.set_width(parking, 2_600)));
        let said = r.said.take();
        assert!(said[0].ends_with(" Not changed from 20.0 m to 40.0 m: that stretch has no parking."), "{said:?}");
    }

    #[test]
    fn a_boundary_dragged_on_the_plan_moves_once_and_takes_the_pick_s_end_along_when_let_go() {
        let r = three_runs(ZoneEnd::First);
        r.vm.pick_run_at(30_000);
        let uid = r.vm.view_now().segments[0].uid;
        r.vm.select(Some(uid));
        let revisions = r.vm.view_now().revisions.len();
        r.said.take();
        let g = r.vm.plan_press(PlanPress::Boundary { zone: 1 }, 200.0);
        let g = r.vm.plan_move(&g, 230.0, 23_000);
        assert_eq!(g, PlanGesture::Boundary { zone: 1, at_mm: 20_000, pick: Some((20_000, 40_000)) });
        let g = r.vm.plan_move(&g, 250.0, 25_000);
        assert_eq!(boundaries(&r), [25_000, 40_000], "shown as it goes");
        assert_eq!((r.vm.picked(), r.vm.view_now().selected), (Some((20_000, 40_000)), Some(uid)), "the pick and the piece stay while it lasts");
        assert!(r.said.take().is_empty(), "quiet while it lasts");
        r.vm.plan_release(g, 25_000);
        assert_eq!(boundaries(&r), [25_000, 40_000]);
        assert_eq!(r.vm.view_now().revisions.len(), revisions + 1, "one revision");
        assert_eq!(r.vm.picked(), Some((25_000, 40_000)), "the pick's end on the boundary followed it");
        assert_eq!(r.vm.view_now().selected, Some(uid));
        assert_eq!(r.said.take().len(), 1, "the move is announced once");
        // Given up half way, everything is as it was.
        let g = r.vm.plan_press(PlanPress::Boundary { zone: 2 }, 400.0);
        let g = r.vm.plan_move(&g, 450.0, 45_000);
        assert_eq!(boundaries(&r), [25_000, 45_000]);
        r.vm.plan_cancel(g);
        assert_eq!(boundaries(&r), [25_000, 40_000]);
        assert_eq!((r.vm.picked(), r.vm.view_now().selected), (Some((25_000, 40_000)), Some(uid)));
        assert_eq!(r.vm.view_now().revisions.len(), revisions + 1);
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn the_point_speaks_where_it_is_while_it_moves_alone_and_the_pick_once_one_is_made() {
        let r = rig_of(&sample(), ZoneEnd::First);
        let text = || r.vm.plan_point_text();
        assert_eq!(text(), "The whole street, 60.0 m long.", "at first the whole street is picked");
        r.vm.plan_key(PlanKey::Move(10_000));
        assert_eq!(text(), "At 10.0 m");
        // An extension under the shortest stretch picks nothing yet: the point says where it is.
        r.vm.plan_key(PlanKey::Extend(3_000));
        assert_eq!(text(), "At 13.0 m");
        r.vm.plan_key(PlanKey::Extend(2_000));
        assert_eq!(text(), "From 10.0 m to 15.0 m, 5.0 m long.");
        // Shrunk back under the shortest stretch, the pick stays as it was and the point says where it is.
        r.vm.plan_key(PlanKey::Extend(-1_000));
        assert_eq!((r.vm.picked(), text()), (Some((10_000, 15_000)), "At 14.0 m".to_string()));
        // A pick by the pointer says the pick again.
        r.vm.pick_run_at(30_000);
        assert_eq!(text(), "The whole street, 60.0 m long.");
    }

    #[test]
    fn a_keyboard_extension_says_nothing_until_it_is_long_enough_to_pick() {
        let r = rig_of(&sample(), ZoneEnd::First);
        r.vm.plan_key(PlanKey::Move(10_000));
        for _ in 0..4 {
            r.vm.plan_key(PlanKey::Extend(1_000));
        }
        assert_eq!((r.vm.plan_point(), r.vm.picked()), (14_000, None));
        assert!(r.said.take().is_empty());
        r.vm.plan_key(PlanKey::Extend(1_000));
        assert_eq!(r.vm.picked(), Some((10_000, 15_000)));
        assert_eq!(r.said.take(), ["From 10.0 m to 15.0 m, 5.0 m long."]);
    }

    #[test]
    fn with_the_whole_street_picked_a_stop_is_added_at_the_left_end_of_its_free_stretch_whichever_end_is_anchored() {
        let stop_mm = crate::standards::model::stop_min_mm();
        for from_end in [ZoneEnd::First, ZoneEnd::Last] {
            let r = rig_of(&bus_street(), from_end);
            r.vm.select_stop(Side::Right, 0);
            assert!(r.vm.add_stop(StopKind::Curb, stop_mm), "{from_end:?}");
            let stop = objects(&r).into_iter().find(|o| matches!(o.0, PlanThing::Stop { .. })).unwrap();
            assert_eq!((stop.1, stop.2), (0, stop_mm), "{from_end:?}: as a stretch picked from the left end would");
        }
    }
}
