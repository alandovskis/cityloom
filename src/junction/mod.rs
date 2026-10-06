//! The junction page: a junction of streets edited on a plan. Its model, view-model,
//! words and views live together.

pub mod frame;
pub mod geometry;
pub mod inspector;
pub mod keys;
pub mod model;
pub mod notes;
pub mod page;
pub mod plan;
pub mod plan_svg;
pub mod read_model;
#[cfg(test)]
mod tests;
pub mod text;
pub mod turns;
pub mod vm;
pub mod watch;

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::HtmlElement;
use wasm_bindgen::prelude::*;

use crate::city::binding::{CityBinding, Place};
use crate::city::store::CityStore;
use crate::shared::platform::browser_ports;

/// One junction being edited. The page drives it through the components the
/// module mounts; the script only starts it up and hands it what the shell
/// chooses (units, region).
#[wasm_bindgen]
pub struct Plan(Rc<vm::JunctionVm>);

#[wasm_bindgen]
impl Plan {
    /// Binds the shell every page shares (settings menu, sidebars, notes tabs)
    /// to this page.
    pub fn mount_shell(&self) {
        crate::shell::mount(self.0.clone(), "details", true);
    }

    /// The page is being left: what is waiting to be kept in the city is kept now.
    pub fn flush(&self) {
        self.0.flush();
    }

    /// The whole drawable state as JSON.
    pub fn view(&self) -> String {
        crate::json(&*self.0.view_now())
    }
}

/// Draws the junction page into the elements it keeps for them: the plan, its
/// heading and status, undo and redo, the key, and the notes beside it.
#[wasm_bindgen]
pub fn mount_page(plan: &Plan) {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let at =
        |id: &str| -> HtmlElement { leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into() };
    let vm = plan.0.clone();
    let mount = |id: &str, view: AnyView| {
        leptos::mount::mount_to(at(id), move || view).forget();
    };
    mount("scroll", view! { <plan::PlanDrawing vm=vm.clone()/> }.into_any());
    mount("street", view! { <page::Header vm=vm.clone()/> }.into_any());
    mount("title-block", view! { <page::TitleBlock vm=vm.clone()/> }.into_any());
    mount("fit-slot", view! { <page::Fit vm=vm.clone()/> }.into_any());
    mount("history", view! { <page::History vm=vm.clone()/> }.into_any());
    mount("plan-key", view! { <page::Key vm=vm.clone()/> }.into_any());
    mount("turns", view! { <turns::Turns vm=vm.clone()/> }.into_any());
    mount("inspector", view! { <inspector::Inspector vm=vm.clone()/> }.into_any());
    mount("across", view! { <notes::Across vm=vm.clone()/> }.into_any());
    mount("conflicts", view! { <notes::Conflicts vm=vm.clone()/> }.into_any());
    mount("conflict-note", view! { <notes::ConflictNote vm=vm.clone()/> }.into_any());
    mount("checks", view! { <notes::Checks vm=vm.clone()/> }.into_any());
    mount("measures", view! { <notes::Measures vm=vm.clone()/> }.into_any());
    mount("revs", view! { <notes::Revisions vm=vm/> }.into_any());
}

/// The name of a junction of the city kept in this browser; empty when there is none.
#[wasm_bindgen]
pub fn junction_name(node: u32) -> String {
    CityStore::current(browser_ports().storage).open().junction_name(node).unwrap_or_default()
}

/// The junction editor on one junction of the city kept in this browser, which
/// writes what is made back to the city. Nothing when there is no such junction
/// or it cannot be drawn with the streets as they now are.
#[wasm_bindgen]
pub fn open_junction(node: u32) -> Option<Plan> {
    let ports = browser_ports();
    let store = CityStore::current(ports.storage.clone());
    let junction = store.open().junction_editor(node, store.region())?;
    let place = Place::Junction(node);
    Some(Plan(vm::JunctionVm::new(ports, junction, Some(CityBinding { store, place }))))
}
