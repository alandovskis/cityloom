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
pub mod text;
#[cfg(test)]
mod tests;
pub mod turns;
pub mod vm;
pub mod watch;

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::HtmlElement;
use wasm_bindgen::prelude::*;

use crate::shared::catalogue::SAMPLES;
use crate::shared::platform::browser_ports;
use crate::vm::binding::{CityBinding, Place};
use crate::vm::city_store::CityStore;

/// One junction being edited. The page drives it through the components the
/// module mounts; the script only starts it up and hands it what the shell
/// chooses (units, region).
#[wasm_bindgen]
pub struct Plan(Rc<vm::JunctionVm>);

#[wasm_bindgen]
impl Plan {
    #[wasm_bindgen(constructor)]
    pub fn new(sample: usize) -> Plan {
        Plan(vm::JunctionVm::new(browser_ports(), model::Junction::new(sample), None))
    }

    /// Binds the shell every page shares (settings menu, sidebars, notes tabs)
    /// to this page.
    pub fn mount_shell(&self) {
        crate::ui::shell::mount(self.0.clone(), "details");
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
/// heading and status, undo and redo, the key, the streets and samples to start
/// from, and the notes beside it.
#[wasm_bindgen]
pub fn mount_page(plan: &Plan) {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let at = |id: &str| -> HtmlElement {
        leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into()
    };
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
    mount("palette", view! { <page::Palette vm=vm.clone()/> }.into_any());
    mount("samples", view! { <page::Samples vm=vm.clone()/> }.into_any());
    mount("turns", view! { <turns::Turns vm=vm.clone()/> }.into_any());
    mount("inspector", view! { <inspector::Inspector vm=vm.clone()/> }.into_any());
    mount("across", view! { <notes::Across vm=vm.clone()/> }.into_any());
    mount("conflicts", view! { <notes::Conflicts vm=vm.clone()/> }.into_any());
    mount("conflict-note", view! { <notes::ConflictNote vm=vm.clone()/> }.into_any());
    mount("checks", view! { <notes::Checks vm=vm.clone()/> }.into_any());
    mount("measures", view! { <notes::Measures vm=vm.clone()/> }.into_any());
    mount("revs", view! { <notes::Revisions vm=vm/> }.into_any());
    // Streets that belong to a city are not added, and there is no street page to go to.
    if plan.0.view_now().linked {
        for selector in [".lower", ".surface[href=\"index.html\"]"] {
            if let Ok(Some(el)) = leptos::prelude::document().query_selector(selector) {
                el.unchecked_into::<HtmlElement>().set_hidden(true);
            }
        }
    }
}

/// Controls, sample junctions, the streets an arm may take and the limits, as JSON.
#[wasm_bindgen]
pub fn junction_catalogue() -> String {
    use model::*;
    let samples: Vec<_> = JUNCTION_SAMPLES.iter().map(|s| serde_json::json!({ "name": s.name, "arms": s.arms.len() })).collect();
    let streets: Vec<_> = SAMPLES.iter().map(|s| serde_json::json!({ "name": s.name, "row_mm": s.row_mm, "pieces": s.segments, "freeway": s.freeway })).collect();
    crate::json(&serde_json::json!({
        "controls": CONTROLS,
        "approaches": APPROACHES,
        "stops": STOPS,
        "rules": RULES,
        "samples": samples,
        "streets": streets,
        "limits": {
            "min_arms": MIN_ARMS, "max_arms": MAX_ARMS,
            "bearing_step": BEARING_STEP, "min_separation": MIN_SEPARATION,
            "corner": [MIN_CORNER_MM, MAX_CORNER_MM, RING_STEP_MM],
            "setback": [MIN_SETBACK_MM, MAX_SETBACK_MM, RING_STEP_MM],
            "crossing": [MIN_CROSSING_MM, MAX_CROSSING_MM, RING_STEP_MM],
            "offset_step": OFFSET_STEP_MM,
            "ring_step": RING_STEP_MM, "max_ring": MAX_RING_MM,
            "island_mm": ISLAND_MM,
            "approach": [APPROACH_MIN_MM, APPROACH_MAX_MM, APPROACH_STEP_MM, TRANSIT_LANE_MM],
            "cycle": [CYCLE_MIN_MM, CYCLE_MAX_MM, RING_STEP_MM, CYCLE_DEFAULT_MM],
        },
    }))
}

/// The name of a junction of the city kept in this browser; empty when there is none.
#[wasm_bindgen]
pub fn junction_name(node: u32) -> String {
    CityStore::new(browser_ports().storage).open().junction_name(node).unwrap_or_default()
}

/// The junction editor on one junction of the city kept in this browser, which
/// writes what is made back to the city. Nothing when there is no such junction
/// or it cannot be drawn with the streets as they now are.
#[wasm_bindgen]
pub fn open_junction(node: u32) -> Option<Plan> {
    let ports = browser_ports();
    let store = CityStore::new(ports.storage.clone());
    let junction = store.open().junction_editor(node, store.region())?;
    let place = Place::Junction(node);
    Some(Plan(vm::JunctionVm::new(ports, junction, Some(CityBinding { store, place }))))
}
