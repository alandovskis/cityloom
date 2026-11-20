//! What the street and junction pages show of a place's scenarios (named alternative designs kept beside its
//! baseline, `city::scenario`) and what the person does with them. The page opens on the baseline; which design is
//! shown lasts as long as the page (it is not kept). Each view-model keeps a `ScenarioState` and is a
//! `ScenarioHost`, which `scenarios_view` draws. No wording here: a refusal is a `Said` the view words.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::I18n;
use crate::shared::said::Said;

/// One scenario of the place, as the picker lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioItem {
    pub id: u32,
    pub name: String,
}

/// The scenarios of the place a page edits, which of them is shown, and what the last action was refused for.
#[derive(Clone)]
pub struct ScenarioState {
    /// The scenario shown and edited; None: the baseline.
    active: ArcRwSignal<Option<u32>>,
    items: ArcRwSignal<Vec<ScenarioItem>>,
    /// Why the last action on a name was refused, until the next action.
    refusal: ArcRwSignal<Option<Said>>,
    /// A delete of the scenario shown waits for the person to confirm it.
    asking_delete: ArcRwSignal<bool>,
}

impl Default for ScenarioState {
    fn default() -> Self {
        ScenarioState {
            active: ArcRwSignal::new(None),
            items: ArcRwSignal::new(Vec::new()),
            refusal: ArcRwSignal::new(None),
            asking_delete: ArcRwSignal::new(false),
        }
    }
}

impl ScenarioState {
    /// The scenario shown, for a view (watched).
    pub fn active(&self) -> Option<u32> {
        self.active.get()
    }

    /// The same, for a command.
    pub fn active_now(&self) -> Option<u32> {
        self.active.get_untracked()
    }

    pub fn items(&self) -> Vec<ScenarioItem> {
        self.items.get()
    }

    /// The name of scenario `id`, for a command.
    pub fn name_now(&self, id: u32) -> Option<String> {
        self.items.with_untracked(|items| items.iter().find(|i| i.id == id).map(|i| i.name.clone()))
    }

    pub fn refusal(&self) -> Option<Said> {
        self.refusal.get()
    }

    pub fn asking_delete(&self) -> bool {
        self.asking_delete.get()
    }

    pub fn asking_delete_now(&self) -> bool {
        self.asking_delete.get_untracked()
    }

    /// Shows scenario `id` (None: the baseline). What an earlier action asked or was refused for is over.
    pub fn show(&self, id: Option<u32>) {
        self.active.set(id);
        self.settle();
    }

    /// The place's scenarios as kept now, by id and name.
    pub fn list(&self, items: Vec<(u32, String)>) {
        let items: Vec<ScenarioItem> = items.into_iter().map(|(id, name)| ScenarioItem { id, name }).collect();
        if self.items.with_untracked(|now| *now != items) {
            self.items.set(items);
        }
    }

    pub fn refuse(&self, why: Said) {
        self.asking_delete.set(false);
        self.refusal.set(Some(why));
    }

    pub fn ask_delete(&self) {
        self.refusal.set(None);
        self.asking_delete.set(true);
    }

    /// An action was done: nothing is refused or waits any more.
    pub fn settle(&self) {
        if self.refusal.with_untracked(Option::is_some) {
            self.refusal.set(None);
        }
        if self.asking_delete.get_untracked() {
            self.asking_delete.set(false);
        }
    }
}

/// A page whose place has scenarios the person manages: the street's or the junction's view-model.
pub trait ScenarioHost {
    fn i18n(&self) -> Rc<I18n>;
    fn scenario_state(&self) -> &ScenarioState;
    /// Whether the place can keep scenarios (it is a place of the city with a key of its own).
    fn scenarios_offered(&self) -> bool;
    /// Whether the page shows its project read-only (#222): the designs can be looked at, and every action on them
    /// is refused as any edit is.
    fn read_only(&self) -> bool {
        false
    }
    /// Shows scenario `id`, or the baseline, what is shown now being kept first. Whether it is shown.
    fn switch_scenario(&self, id: Option<u32>) -> bool;
    /// A new scenario named `name`, made from the baseline and shown. Whether it was made.
    fn create_scenario(&self, name: &str) -> bool;
    /// A copy of the scenario shown, named `name`, and shown. Whether it was made.
    fn duplicate_scenario(&self, name: &str) -> bool;
    /// Names the scenario shown `name`. Whether it was renamed.
    fn rename_scenario(&self, name: &str) -> bool;
    /// Deletes the scenario shown, once the person has confirmed it (the first press asks), and shows the
    /// baseline. Whether it was deleted.
    fn delete_scenario(&self) -> bool;
}
