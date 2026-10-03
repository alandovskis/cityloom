//! The page, drawn from the model by Leptos. Each component reads the model's
//! view and calls the model's edits; the page holds no rules of its own.

pub mod bind;
pub mod core;
pub mod inspector;
pub mod keys;
pub mod live;
pub mod map;
pub mod map_svg;
pub mod notes;
pub mod page;
pub mod plan;
pub mod platform;
pub mod plan_svg;
pub mod shared;
pub mod sheet;
pub mod sheet_watch;
pub mod store;
pub mod street;
pub mod street_inspector;
pub mod street_keys;
pub mod street_notes;
pub mod street_page;
pub mod street_svg;
pub mod street_text;
pub mod turns;
pub mod watch;

#[cfg(test)]
mod tests;

use leptos::prelude::*;
use leptos::web_sys::HtmlElement;
use wasm_bindgen::prelude::*;

/// Draws the junction page into the elements it keeps for them: the plan, its
/// heading and status, undo and redo, the key, the streets and samples to start
/// from, and the notes beside it.
#[wasm_bindgen]
pub fn mount_page(plan: &crate::Plan) {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let at = |id: &str| -> HtmlElement {
        leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into()
    };
    let shared = plan.0.clone();
    let mount = |id: &str, view: AnyView| {
        leptos::mount::mount_to(at(id), move || view).forget();
    };
    mount("scroll", view! { <plan::PlanDrawing shared=shared.clone()/> }.into_any());
    mount("street", view! { <page::Header shared=shared.clone()/> }.into_any());
    mount("title-block", view! { <page::TitleBlock shared=shared.clone()/> }.into_any());
    mount("fit-slot", view! { <page::Fit shared=shared.clone()/> }.into_any());
    mount("history", view! { <page::History shared=shared.clone()/> }.into_any());
    mount("plan-key", view! { <page::Key shared=shared.clone()/> }.into_any());
    mount("palette", view! { <page::Palette shared=shared.clone()/> }.into_any());
    mount("samples", view! { <page::Samples shared=shared.clone()/> }.into_any());
    mount("turns", view! { <turns::Turns shared=shared.clone()/> }.into_any());
    mount("inspector", view! { <inspector::Inspector shared=shared.clone()/> }.into_any());
    mount("across", view! { <notes::Across shared=shared.clone()/> }.into_any());
    mount("conflicts", view! { <notes::Conflicts shared=shared.clone()/> }.into_any());
    mount("conflict-note", view! { <notes::ConflictNote shared=shared.clone()/> }.into_any());
    mount("checks", view! { <notes::Checks shared=shared.clone()/> }.into_any());
    mount("measures", view! { <notes::Measures shared=shared.clone()/> }.into_any());
    mount("revs", view! { <notes::Revisions shared=shared/> }.into_any());
    // Streets that belong to a city are not added, and there is no street page to go to.
    if plan.0.view().linked {
        for selector in [".lower", ".surface[href=\"index.html\"]"] {
            if let Ok(Some(el)) = leptos::prelude::document().query_selector(selector) {
                el.unchecked_into::<HtmlElement>().set_hidden(true);
            }
        }
    }
}

/// Says `text` to a screen reader.
#[wasm_bindgen]
pub fn say(text: &str) {
    live::say(text);
}

/// Draws the street page into the elements it keeps for them: the section, its
/// heading and status, undo and redo, the menu to add a piece, the time of day,
/// the details of the selected piece, and the notes beside it. `ends`, for a
/// street of a city, is the JSON of the places it runs between.
#[wasm_bindgen]
pub fn mount_street_page(sheet: &crate::Sheet, ends: Option<String>) {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let at = |id: &str| -> HtmlElement {
        leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into()
    };
    let shared = sheet.0.clone();
    let ends: Option<Vec<street_page::StreetEnd>> = ends.and_then(|json| serde_json::from_str(&json).ok());
    let mount = |id: &str, view: AnyView| {
        leptos::mount::mount_to(at(id), move || view).forget();
    };
    mount("scroll", view! { <street::StreetDrawing shared=shared.clone()/> }.into_any());
    mount("street", view! { <street_page::StreetHeader shared=shared.clone() ends=ends.clone()/> }.into_any());
    mount("title-block", view! { <street_page::TitleBlock shared=shared.clone()/> }.into_any());
    mount("fit-slot", view! { <street_page::Fit shared=shared.clone()/> }.into_any());
    mount("fit-checks-slot", view! { <street_notes::FitChecks shared=shared.clone()/> }.into_any());
    mount("checks-n-slot", view! { <street_notes::ChecksBadge shared=shared.clone()/> }.into_any());
    mount("history", view! { <street_page::History shared=shared.clone()/> }.into_any());
    mount("add", view! { <street_page::AddMenu shared=shared.clone()/> }.into_any());
    mount("clock-slot", view! { <street_page::Clock shared=shared.clone()/> }.into_any());
    mount("time-note", view! { <street_page::TimeNote shared=shared.clone()/> }.into_any());
    mount("welcome-slot", view! { <street_page::Welcome shared=shared.clone()/> }.into_any());
    mount("inspector", view! { <street_inspector::StreetInspector shared=shared.clone()/> }.into_any());
    mount("space", view! { <street_notes::Space shared=shared.clone()/> }.into_any());
    mount("cap", view! { <street_notes::Capacity shared=shared.clone()/> }.into_any());
    mount("checks", view! { <street_notes::Checks shared=shared.clone()/> }.into_any());
    mount("revs", view! { <street_notes::Revisions shared=shared.clone()/> }.into_any());
    mount("measures", view! { <street_notes::Measures shared=shared/> }.into_any());
    // A street of a city is reached from the map, and its junctions are pages of their own.
    if ends.is_some() {
        if let Ok(Some(el)) = leptos::prelude::document().query_selector(".surface[href=\"intersection.html\"]") {
            el.unchecked_into::<HtmlElement>().set_hidden(true);
        }
    }
}

/// The map page as the script sees it: what the pages around it ask of it.
#[wasm_bindgen]
pub struct MapPage(std::rc::Rc<crate::vm::map::MapVm>);

#[wasm_bindgen]
impl MapPage {
    /// Sets the units lengths are shown in, `"m"` or `"ft"`.
    pub fn set_units(&self, units: &str) {
        self.0.set_units(crate::units::Units::parse(units));
    }

    /// Sets the region, by index into `materials().regions`.
    pub fn set_region(&self, region: usize) -> bool {
        self.0.set_region(region);
        true
    }

    /// The id of the region the city is shown in.
    pub fn region_id(&self) -> String {
        crate::catalogue::REGIONS[self.0.region()].id.to_string()
    }
}

/// Draws the city map page into the elements it keeps for it, and hands back
/// what the script needs to reach it.
#[wasm_bindgen]
pub fn mount_map() -> MapPage {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let vm = crate::vm::map::MapVm::new(platform::browser_ports());
    let at = |id: &str| -> HtmlElement {
        leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into()
    };
    let mount = |id: &str, view: AnyView| {
        leptos::mount::mount_to(at(id), move || view).forget();
    };
    mount("street", view! { <map::MapHeader vm=vm.clone()/> }.into_any());
    mount("reset-slot", view! { <map::ResetButton vm=vm.clone()/> }.into_any());
    mount("map-tools-slot", view! { <map::MapTools vm=vm.clone()/> }.into_any());
    mount("map-slot", view! { <map::MapView vm=vm.clone()/> }.into_any());
    mount("plan-key", view! { <map::Legend vm=vm.clone()/> }.into_any());
    mount("fit-slot", view! { <map::Status vm=vm.clone()/> }.into_any());
    mount("inspector", view! { <map::Places vm=vm.clone()/> }.into_any());
    mount("checks-lead", view! { <map::ChecksLead vm=vm.clone()/> }.into_any());
    mount("checks", view! { <map::Checks vm=vm.clone()/> }.into_any());
    mount("changes-lead", view! { <map::ChangesLead vm=vm.clone()/> }.into_any());
    mount("changes", view! { <map::Changes vm=vm.clone()/> }.into_any());
    mount("title-block", view! { <map::TitleBlock vm=vm.clone()/> }.into_any());
    MapPage(vm)
}
