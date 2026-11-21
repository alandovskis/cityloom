//! What a component of the street page needs to draw from, and change, the
//! shared street.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::catalogue::Side;
use crate::shared::i18n::I18n;
use crate::shared::units::Units;
use crate::street::model::{CalmingKind, Editor, GreenForm, SegView, StopKind, Use, View};
use crate::street::vm::StreetVm;
use crate::street::vm::add_menu::{Entry, Item, Topic};
use crate::street::vm::keys::Action;
use crate::street::vm::row::Row;

/// What a component reads of the street view-model and tells it. The view-model
/// is kept where only this thread can reach it, so the handle is `Copy` and any
/// closure can hold it.
#[derive(Clone, Copy)]
pub struct SheetWatch {
    vm: StoredValue<Rc<StreetVm>, LocalStorage>,
}

impl SheetWatch {
    pub fn new(vm: Rc<StreetVm>) -> SheetWatch {
        SheetWatch { vm: StoredValue::new_local(vm) }
    }

    /// The planning goal chosen.
    pub fn vm_goal(&self) -> crate::shared::goals::GoalChoice {
        self.vm.with_value(|v| v.goal().clone())
    }

    pub fn passes(&self) -> crate::shared::check_fold::PassFold {
        self.vm.with_value(|v| v.passes().clone())
    }

    /// The view, drawing again when the street is edited.
    pub fn view(&self) -> Rc<View> {
        self.vm.with_value(|v| v.view())
    }

    /// The view as it is now, for an event handler: nothing is watched.
    pub fn view_now(&self) -> Rc<View> {
        self.vm.with_value(|v| v.view_now())
    }

    /// How the street stands against each area it is inside, drawing again when the street is edited.
    pub fn area_checks(&self) -> Vec<crate::restriction::model::Finding> {
        self.vm.with_value(|v| v.area_checks())
    }

    /// What the last change did to the street, once the edits have settled, drawing again when it is shown.
    pub fn impact(&self) -> Option<crate::street::vm::impact::Impact> {
        self.vm.with_value(|v| v.impact())
    }

    /// The page's words, for a view: ask with `tr` so it is drawn again on a language switch.
    pub fn i18n(&self) -> Rc<I18n> {
        self.vm.with_value(|v| v.i18n().clone())
    }

    /// `href` to another page of the street's project.
    pub fn link(&self, href: &str) -> String {
        self.vm.with_value(|v| v.link(href))
    }

    /// Whether the street is shown read-only: the page offers no edit.
    pub fn read_only(&self) -> bool {
        self.vm.with_value(|v| v.read_only())
    }

    pub fn units(&self) -> Units {
        self.vm.with_value(|v| v.units())
    }

    pub fn units_now(&self) -> Units {
        self.vm.with_value(|v| v.units_now())
    }

    pub fn now(&self) -> (Rc<View>, Units) {
        (self.view(), self.units())
    }

    /// What `f` makes of one piece of the view, or None when there is no such piece.
    pub fn segment<R>(&self, uid: u32, f: impl FnOnce(&SegView) -> R) -> Option<R> {
        self.view().segments.iter().find(|s| s.uid == uid).map(f)
    }

    /// Edits the street as the page does, announcing what was done.
    pub fn edit(&self, f: impl FnOnce(&mut Editor) -> bool) -> bool {
        self.vm.with_value(|v| v.apply(f))
    }

    /// A change that is not announced, as each step of a drag is.
    pub fn quiet<R: Default>(&self, f: impl FnOnce(&mut Editor) -> R) -> R {
        self.vm.with_value(|v| v.quiet(f))
    }

    /// The topic the Add a piece menu opens on, for an event handler.
    pub fn add_menu_opening(&self) -> Topic {
        self.vm.with_value(|v| v.add_menu_opening())
    }

    /// A topic's items, each with the reason it cannot be added, if any; drawing again when the street is edited.
    pub fn add_entries(&self, topic: Topic) -> Vec<Entry> {
        self.vm.with_value(|v| v.add_entries(topic))
    }

    /// The items whose name holds `query`, drawing again when the street is edited or the language switched.
    pub fn add_search(&self, query: &str) -> Vec<Entry> {
        self.vm.with_value(|v| v.add_search(query))
    }

    /// Adds the item, or says why it cannot be added.
    pub fn add_item(&self, item: Item) -> bool {
        self.vm.with_value(|v| v.add_item(item))
    }

    /// Says how many items the search matches.
    pub fn say_matches(&self, count: usize) {
        self.vm.with_value(|v| v.say_matches(count));
    }

    pub fn select(&self, uid: Option<u32>) {
        self.vm.with_value(|v| v.select(uid));
    }

    pub fn finish_gesture(&self, commit: bool) {
        self.vm.with_value(|v| v.end_gesture(commit));
    }

    pub fn select_cell<R: Row>(&self, side: Side, cell: usize) {
        self.vm.with_value(|v| v.select_cell::<R>(side, cell));
    }

    pub fn select_cell_relative<R: Row>(&self, side: Side, delta: i32) {
        self.vm.with_value(|v| v.select_cell_relative::<R>(side, delta));
    }

    pub fn select_cell_edge<R: Row>(&self, side: Side, last: bool) {
        self.vm.with_value(|v| v.select_cell_edge::<R>(side, last));
    }

    pub fn nudge_cell<R: Row>(&self, delta_mm: i32) -> bool {
        self.vm.with_value(|v| v.nudge_cell::<R>(delta_mm))
    }

    pub fn begin_cell_drag<R: Row>(&self) -> bool {
        self.vm.with_value(|v| v.begin_cell_drag::<R>())
    }

    pub fn drag_cell_to<R: Row>(&self, from_mm: i32) {
        self.vm.with_value(|v| v.drag_cell_to::<R>(from_mm));
    }

    pub fn end_cell_drag(&self, keep: bool) {
        self.vm.with_value(|v| v.end_cell_drag(keep));
    }

    pub fn add_curb(&self, use_: Use, length_mm: i32) -> bool {
        self.vm.with_value(|v| v.add_curb(use_, length_mm))
    }

    pub fn set_curb_use(&self, use_: Use) -> bool {
        self.vm.with_value(|v| v.set_curb_use(use_))
    }

    pub fn resize_curb(&self, length_mm: i32) -> bool {
        self.vm.with_value(|v| v.resize_curb(length_mm))
    }

    pub fn remove_curb(&self) -> bool {
        self.vm.with_value(|v| v.remove_curb())
    }

    pub fn select_calming(&self, cell: usize) {
        self.vm.with_value(|v| v.select_calming(cell));
    }

    pub fn select_calming_relative(&self, delta: i32) {
        self.vm.with_value(|v| v.select_calming_relative(delta));
    }

    pub fn select_calming_edge(&self, last: bool) {
        self.vm.with_value(|v| v.select_calming_edge(last));
    }

    pub fn calming_kind_shown(&self) -> CalmingKind {
        self.vm.with_value(|v| v.calming_kind_shown())
    }

    pub fn pick_calming_kind(&self, kind: CalmingKind) {
        self.vm.with_value(|v| v.pick_calming_kind(kind));
    }

    pub fn calming_first_shown(&self) -> crate::shared::catalogue::Side {
        self.vm.with_value(|v| v.calming_first_shown())
    }

    pub fn pick_calming_first(&self, side: crate::shared::catalogue::Side) {
        self.vm.with_value(|v| v.pick_calming_first(side));
    }

    pub fn add_calming(&self, kind: CalmingKind, length_mm: i32) -> bool {
        self.vm.with_value(|v| v.add_calming(kind, length_mm))
    }

    pub fn set_calming_length(&self, length_mm: i32) -> bool {
        self.vm.with_value(|v| v.set_calming_length(length_mm))
    }

    pub fn remove_calming(&self) -> bool {
        self.vm.with_value(|v| v.remove_calming())
    }

    pub fn nudge_calming(&self, delta_mm: i32) -> bool {
        self.vm.with_value(|v| v.nudge_calming(delta_mm))
    }

    pub fn begin_calming_drag(&self) -> bool {
        self.vm.with_value(|v| v.begin_calming_drag())
    }

    pub fn drag_calming_to(&self, from_mm: i32) {
        self.vm.with_value(|v| v.drag_calming_to(from_mm));
    }

    pub fn end_calming_drag(&self, keep: bool) {
        self.vm.with_value(|v| v.end_calming_drag(keep));
    }

    pub fn stop_kind_shown(&self) -> StopKind {
        self.vm.with_value(|v| v.stop_kind_shown())
    }

    pub fn stop_shelter_shown(&self) -> bool {
        self.vm.with_value(|v| v.stop_shelter_shown())
    }

    pub fn stop_name_shown(&self) -> String {
        self.vm.with_value(|v| v.stop_name_shown())
    }

    pub fn pick_stop_kind(&self, kind: StopKind) {
        self.vm.with_value(|v| v.pick_stop_kind(kind));
    }

    pub fn default_stop_kind(&self) -> StopKind {
        self.vm.with_value(|v| v.default_stop_kind())
    }

    pub fn add_stop(&self, kind: StopKind, length_mm: i32) -> bool {
        self.vm.with_value(|v| v.add_stop(kind, length_mm))
    }

    pub fn set_stop_kind(&self, kind: StopKind) -> bool {
        self.vm.with_value(|v| v.set_stop_kind(kind))
    }

    pub fn set_stop_shelter(&self, on: bool) -> bool {
        self.vm.with_value(|v| v.set_stop_shelter(on))
    }

    pub fn name_stop(&self, name: &str) -> bool {
        self.vm.with_value(|v| v.name_stop(name))
    }

    pub fn resize_stop(&self, length_mm: i32) -> bool {
        self.vm.with_value(|v| v.resize_stop(length_mm))
    }

    pub fn remove_stop(&self) -> bool {
        self.vm.with_value(|v| v.remove_stop())
    }

    pub fn select_green(&self, i: usize) {
        self.vm.with_value(|v| v.select_green(i));
    }

    pub fn select_green_relative(&self, delta: i32) {
        self.vm.with_value(|v| v.select_green_relative(delta));
    }

    pub fn select_green_edge(&self, last: bool) {
        self.vm.with_value(|v| v.select_green_edge(last));
    }

    pub fn nudge_green(&self, delta_mm: i32) -> bool {
        self.vm.with_value(|v| v.nudge_green(delta_mm))
    }

    pub fn begin_green_drag(&self) -> bool {
        self.vm.with_value(|v| v.begin_green_drag())
    }

    pub fn drag_green_to(&self, from_mm: i32) {
        self.vm.with_value(|v| v.drag_green_to(from_mm));
    }

    pub fn end_green_drag(&self, keep: bool) {
        self.vm.with_value(|v| v.end_green_drag(keep));
    }

    pub fn add_green_clear(&self, form: GreenForm) -> bool {
        self.vm.with_value(|v| v.add_green_clear(form))
    }

    pub fn add_green(&self, form: GreenForm) -> bool {
        self.vm.with_value(|v| v.add_green(form))
    }

    pub fn change_green(&self, form: &GreenForm) -> bool {
        self.vm.with_value(|v| v.change_green(form))
    }

    pub fn remove_green(&self) -> bool {
        self.vm.with_value(|v| v.remove_green())
    }

    /// The pieces an element `length_mm` long starting at `at_mm` on the plan can stand on, drawing again when the
    /// street is edited.
    pub fn hosts_at(&self, at_mm: i32, length_mm: i32) -> Vec<(usize, crate::shared::said::Said)> {
        let _ = self.view();
        self.vm.with_value(|v| v.hosts_at(at_mm, length_mm))
    }

    /// Where element `c` of the Green row starts on the plan, from its left end.
    pub fn green_left_start(&self, c: &crate::street::model::GreenCell) -> i32 {
        self.vm.with_value(|v| v.green_left_start(c))
    }

    pub fn say_bad_number(&self) {
        self.vm.with_value(|v| v.say_bad_number());
    }

    pub fn speed_options(&self) -> Vec<crate::street::vm::SpeedOption> {
        self.vm.with_value(|v| v.speed_options())
    }

    /// The level of the details panel shown, drawing again when another is.
    pub fn level(&self) -> super::level::Level {
        self.vm.with_value(|v| v.level())
    }

    /// The path to the level shown, worded.
    pub fn path(&self) -> Vec<super::Crumb> {
        self.vm.with_value(|v| v.path())
    }

    pub fn show_level(&self, level: super::level::Level) {
        self.vm.with_value(|v| v.show_level(level))
    }

    pub fn set_speed(&self, kmh: Option<u16>) -> bool {
        self.vm.with_value(|v| v.set_speed(kmh))
    }

    pub fn follow_street_speed(&self) -> bool {
        self.vm.with_value(|v| v.follow_street_speed())
    }

    pub fn undo(&self) -> bool {
        self.vm.with_value(|v| v.undo())
    }

    pub fn redo(&self) -> bool {
        self.vm.with_value(|v| v.redo())
    }

    pub fn reset(&self) -> bool {
        self.vm.with_value(|v| v.reset())
    }

    /// What the Transform dialog offers, drawing again when the street is edited.
    pub fn transform_choices(&self) -> crate::street::vm::transform::TransformChoices {
        self.vm.with_value(|v| v.transform_choices())
    }

    /// What a choice of the Transform dialog would do, tried on a copy of the street.
    pub fn preview(&self, choice: crate::street::vm::transform::Choice) -> crate::street::vm::transform::Preview {
        self.vm.with_value(|v| v.preview(choice))
    }

    /// Applies a choice of the Transform dialog, as one undoable step.
    pub fn apply_choice(&self, choice: crate::street::vm::transform::Choice) -> bool {
        self.vm.with_value(|v| v.apply_choice(choice))
    }

    /// What a key on the section asks for, as far as it changes the street.
    /// Going to the width field, and letting go of a drag, are for the page.
    pub fn apply(&self, action: Action) {
        let v = self.view_now();
        let selected = v.selected;
        match action {
            Action::Select(dir) => self.vm.with_value(|v| v.select_relative(dir)),
            Action::Move(dir) => {
                let Some(uid) = selected else { return };
                let Some(i) = v.segments.iter().position(|s| s.uid == uid) else { return };
                let j = i as i32 + dir;
                if j >= 0 && (j as usize) < v.segments.len() {
                    self.edit(|e| e.move_to(uid, j as usize));
                }
            }
            Action::Nudge(mm) => {
                if let Some(uid) = selected {
                    self.edit(|e| e.nudge_width(uid, mm));
                }
            }
            Action::Remove => {
                if let Some(uid) = selected {
                    self.edit(|e| e.remove(uid));
                }
            }
            Action::EditWidth => {}
            Action::Escape => self.select(None),
        }
    }
}
