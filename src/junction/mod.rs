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
use crate::shared::i18n::Resources;
use crate::shared::platform::browser_ports;

/// What the junction says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

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
        crate::shell::mount(self.0.i18n().clone(), self.0.clone(), "details-details", true);
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

/// The words of the junction page, and of the page of a junction that cannot be drawn: the stored language, or
/// the browser's, which the document is then told it is in.
fn page_i18n(ports: &crate::shared::ports::Ports) -> Rc<crate::shared::i18n::I18n> {
    crate::i18n_browser(ports)
}

/// Whether the city kept in this browser has junction `node`; the page goes back to the map when it has not.
#[wasm_bindgen]
pub fn junction_exists(node: u32) -> bool {
    CityStore::current(browser_ports().storage).open().junction_name(node).is_some()
}

/// The page of junction `node` when the streets that meet there can no longer be drawn as a junction: it says
/// so under the junction's name, and keeps the shell every page shares (the settings menu and the sidebars).
#[wasm_bindgen]
pub fn mount_bare_shell(node: u32) {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let ports = browser_ports();
    let i18n = page_i18n(&ports);
    let name = CityStore::current(ports.storage.clone()).open().junction_name(node);
    let doc = leptos::prelude::document();
    let find = |selector: &str| doc.query_selector(selector).ok().flatten();
    // Nothing here can be drawn or changed: the plan, its tools and its status make way for the panel.
    if let Some(tools) = find(".tools") {
        let _ = tools.set_attribute("hidden", "");
    }
    for selector in [".plan-drawing", ".statusbar"] {
        if let Some(el) = find(selector) {
            el.remove();
        }
    }
    if let (Some(at), Some(name)) = (find("#street"), name) {
        let i18n = i18n.clone();
        leptos::mount::mount_to(at.unchecked_into(), move || view! { <page::StuckHeader i18n=i18n name=name/> }).forget();
    }
    if let Some(at) = find(".stage-main") {
        let i18n = i18n.clone();
        leptos::mount::mount_to(at.unchecked_into(), move || view! { <page::Stuck i18n=i18n/> }).forget();
    }
    crate::shell::mount(i18n, Rc::new(crate::shell::Bare::default()), "details-details", true);
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
    let i18n = page_i18n(&ports);
    Some(Plan(vm::JunctionVm::new(ports, i18n, junction, Some(CityBinding { store, place }))))
}
