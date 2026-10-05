//! The street page: a street's cross-section edited as a row of pieces. Its
//! model, view-model, words and views live together.

pub mod inspector;
pub mod keys;
pub mod measures;
pub mod model;
pub mod notes;
pub mod page;
pub mod svg;
#[cfg(test)]
mod tests;
pub mod text;
pub mod view;
pub mod vm;
pub mod watch;

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::HtmlElement;
use wasm_bindgen::prelude::*;

use crate::city::binding::{CityBinding, Place};
use crate::city::store::CityStore;
use crate::shared::platform::browser_ports;

/// One street being edited. The page drives it through the components the
/// module mounts; the script only starts it up and hands it what the shell
/// chooses (units, region).
#[wasm_bindgen]
pub struct Sheet(Rc<vm::StreetVm>);

#[wasm_bindgen]
impl Sheet {
    /// Binds the shell every page shares (settings menu, sidebars, notes tabs)
    /// to this page.
    pub fn mount_shell(&self) {
        crate::shell::mount(self.0.clone(), "piece details", true);
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

/// Draws the street page into the elements it keeps for them: the section, its
/// heading and status, undo and redo, the menu to add a piece, the time of day,
/// the details of the selected piece, and the notes beside it. `ends`, for a
/// street of a city, is the JSON of the places it runs between.
#[wasm_bindgen]
pub fn mount_street_page(sheet: &Sheet, ends: Option<String>) {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let at =
        |id: &str| -> HtmlElement { leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into() };
    let vm = sheet.0.clone();
    let ends: Option<Vec<page::StreetEnd>> = ends.and_then(|json| serde_json::from_str(&json).ok());
    let mount = |id: &str, view: AnyView| {
        leptos::mount::mount_to(at(id), move || view).forget();
    };
    mount("scroll", view! { <view::StreetDrawing vm=vm.clone()/> }.into_any());
    mount("street", view! { <page::StreetHeader vm=vm.clone() ends=ends.clone()/> }.into_any());
    mount("title-block", view! { <page::TitleBlock vm=vm.clone()/> }.into_any());
    mount("fit-slot", view! { <page::Fit vm=vm.clone()/> }.into_any());
    mount("fit-checks-slot", view! { <notes::FitChecks vm=vm.clone()/> }.into_any());
    mount("checks-n-slot", view! { <notes::ChecksBadge vm=vm.clone()/> }.into_any());
    mount("history", view! { <page::History vm=vm.clone()/> }.into_any());
    mount("add", view! { <page::AddMenu vm=vm.clone()/> }.into_any());
    mount("clock-slot", view! { <page::Clock vm=vm.clone()/> }.into_any());
    mount("time-note", view! { <page::TimeNote vm=vm.clone()/> }.into_any());
    mount("welcome-slot", view! { <page::Welcome vm=vm.clone()/> }.into_any());
    mount("inspector", view! { <inspector::StreetInspector vm=vm.clone()/> }.into_any());
    mount("space", view! { <notes::Space vm=vm.clone()/> }.into_any());
    mount("cap", view! { <notes::Capacity vm=vm.clone()/> }.into_any());
    mount("checks", view! { <notes::Checks vm=vm.clone()/> }.into_any());
    mount("revs", view! { <notes::Revisions vm=vm.clone()/> }.into_any());
    mount("measures", view! { <notes::Measures vm=vm/> }.into_any());
}

/// The two ends of a street of the city kept in this browser as JSON: each a
/// junction or where it leaves the map.
#[wasm_bindgen]
pub fn street_ends(edge: u32) -> String {
    crate::json(&CityStore::current(browser_ports().storage).open().street_ends(edge))
}

/// The street editor on one street of the city kept in this browser, which
/// writes what is made back to the city. Nothing when there is no such street.
#[wasm_bindgen]
pub fn open_street(edge: u32) -> Option<Sheet> {
    let ports = browser_ports();
    let store = CityStore::current(ports.storage.clone());
    let street = store.open().street_editor(edge, store.region())?;
    let place = Place::Street(edge);
    Some(Sheet(vm::StreetVm::new(ports, street, Some(CityBinding { store, place }))))
}
