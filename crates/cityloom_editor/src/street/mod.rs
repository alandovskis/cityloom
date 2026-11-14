//! The street page: a street's cross-section edited as a row of pieces. Its
//! model, view-model, words and views live together.

pub mod model;
pub mod view;
pub mod vm;

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::HtmlElement;
use wasm_bindgen::prelude::*;

use crate::city::binding::{CityBinding, Place};
use crate::city::store::CityStore;
use crate::shared::i18n::Resources;
use crate::shared::platform::browser_ports;
use crate::shared::ports::Ports;
use crate::shared::project_id::link;

/// What the street page says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

/// One street being edited. The page drives it through the components the
/// module mounts; the script only starts it up and hands it what the shell
/// chooses (units, region).
#[wasm_bindgen]
pub struct Sheet(Rc<vm::StreetVm>, CityStore);

impl Sheet {
    /// The view-model behind the page (tests).
    #[cfg(test)]
    pub fn vm(&self) -> Rc<vm::StreetVm> {
        self.0.clone()
    }

    /// The view-model behind the page, for the composition root (`lib.rs`), which hands what it reads to the sections
    /// of other slices.
    pub(crate) fn view_model(&self) -> Rc<vm::StreetVm> {
        self.0.clone()
    }
}

#[wasm_bindgen]
impl Sheet {
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

/// Draws the street page into the elements it keeps for them: the section, its
/// heading and status, undo and redo, the menu to add a piece, the time of day,
/// the details of the selected piece, and the notes beside it. `edge`, for a
/// street of the city kept in this browser, is the street: its header names the
/// places it runs between.
#[wasm_bindgen]
pub fn mount_street_page(sheet: &Sheet, edge: Option<u32>) {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let at =
        |id: &str| -> HtmlElement { leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into() };
    let vm = sheet.0.clone();
    let city = edge.map(|edge| (edge, sheet.1.open()));
    let ends = city.as_ref().map(|(edge, city)| city.street_ends(*edge));
    let neighbours = city.as_ref().map(|(edge, city)| city.street_neighbours(*edge));
    let mount = |id: &str, view: AnyView| {
        leptos::mount::mount_to(at(id), move || view).forget();
    };
    mount("scroll", view! { <view::StreetDrawing vm=vm.clone()/> }.into_any());
    mount("h-drawing", view! { <view::page::DrawingHeading vm=vm.clone()/> }.into_any());
    mount("street", view! { <view::page::StreetHeader vm=vm.clone() ends=ends.clone() neighbours=neighbours/> }.into_any());
    mount("pieces-slot", view! { <view::piece_list::PieceList vm=vm.clone()/> }.into_any());
    mount("zones-slot", view! { <view::strip::ZoneStrip vm=vm.clone() ends=ends.clone()/> }.into_any());
    mount("curb-slot", view! { <view::curb_strip::CurbStrips vm=vm.clone()/> }.into_any());
    mount("panel-path-slot", view! { <view::panel_path::PanelPath vm=vm.clone()/> }.into_any());
    mount("zone-entry-slot", view! { <view::panel_path::ZoneEntry vm=vm.clone()/> }.into_any());
    mount("calming-slot", view! { <view::calming_strip::CalmingStrip vm=vm.clone()/> }.into_any());
    mount("green-slot", view! { <view::green_strip::GreenStrip vm=vm.clone()/> }.into_any());
    mount("speed-slot", view! { <view::speed_view::SpeedLimit vm=vm.clone()/> }.into_any());
    mount("zone-tools-slot", view! { <view::zone_tools::ZoneTools vm=vm.clone()/> }.into_any());
    mount("curb-tools-slot", view! { <view::curb_tools::CurbTools vm=vm.clone()/> }.into_any());
    mount("stop-tools-slot", view! { <view::stop_tools::StopTools vm=vm.clone()/> }.into_any());
    mount("calming-tools-slot", view! { <view::calming_tools::CalmingTools vm=vm.clone()/> }.into_any());
    mount("green-tools-slot", view! { <view::green_tools::GreenTools vm=vm.clone()/> }.into_any());
    mount("title-block", view! { <view::page::TitleBlock vm=vm.clone()/> }.into_any());
    mount("fit-slot", view! { <view::page::Fit vm=vm.clone()/> }.into_any());
    mount("fit-checks-slot", view! { <view::notes::FitChecks vm=vm.clone()/> }.into_any());
    mount("checks-n-slot", view! { <view::notes::ChecksBadge vm=vm.clone()/> }.into_any());
    mount("history", view! { <view::page::History vm=vm.clone()/> }.into_any());
    mount("add", view! { <view::page::AddMenu vm=vm.clone()/> }.into_any());
    mount("transform-slot", view! { <view::page::Transform vm=vm.clone()/> }.into_any());
    mount("clock-slot", view! { <view::page::Clock vm=vm.clone()/> }.into_any());
    mount("time-note", view! { <view::page::TimeNote vm=vm.clone()/> }.into_any());
    mount("welcome-slot", view! { <view::page::Welcome vm=vm.clone()/> }.into_any());
    mount("inspector", view! { <view::inspector::StreetInspector vm=vm.clone()/> }.into_any());
    mount("impact", view! { <view::notes::LastChange vm=vm.clone()/> }.into_any());
    mount("space", view! { <view::notes::Space vm=vm.clone()/> }.into_any());
    mount("cap", view! { <view::notes::Capacity vm=vm.clone()/> }.into_any());
    mount("goal-slot", view! { <view::notes::GoalGuide vm=vm.clone()/> }.into_any());
    mount("checks", view! { <view::notes::Checks vm=vm.clone()/> }.into_any());
    mount("atlas-source", view! { <view::notes::AtlasSource vm=vm.clone()/> }.into_any());
    mount("revs", view! { <view::notes::Revisions vm=vm.clone()/> }.into_any());
    mount("provenance", view! { <view::page::Provenance vm=vm/> }.into_any());
}

/// The street editor on one street of the city of the page's project (`project::context`), which writes what is
/// made back to the city. Nothing when there is no such street (the page has then gone to its project's map), or
/// when the project is not here (the page has gone to the list).
#[wasm_bindgen]
pub fn open_street(edge: u32) -> Option<Sheet> {
    open_street_in(browser_ports(), edge)
}

/// `open_street` from the page's own ports (root storage).
pub fn open_street_in(ports: Ports, edge: u32) -> Option<Sheet> {
    let ports = crate::project::context::open_page(ports)?;
    // The standards the browser chose are in force before the street is read: the checks and the widths a piece may
    // take follow them (none chosen: the built-in standards).
    crate::standards::store::activate(ports.storage.clone());
    let store = CityStore::current(ports.storage.clone());
    // No street is numbered 0: the page's script gives 0 for an address with no street.
    let Some(street) = (edge != 0).then(|| store.open().street_editor(edge, store.region())).flatten() else {
        ports.navigator.replace(&link("map.html", ports.storage.project().as_deref()));
        return None;
    };
    let place = Place::Street(edge);
    // One instance for the page: the shell switches it and the street's words follow.
    let i18n = crate::i18n_browser(&ports);
    let vm = vm::StreetVm::new(ports, i18n, street, Some(CityBinding { store: store.clone(), place }));
    vm.watch_sync();
    Some(Sheet(vm, store))
}
