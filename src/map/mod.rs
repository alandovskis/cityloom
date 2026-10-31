//! The city map page: the whole city drawn over an OpenStreetMap basemap to pan,
//! zoom and choose a street or junction from. MapLibre draws the map and takes the
//! pointer (`web/basemap.js`, reached through the `Mapper` port); the view-model,
//! the places it hands the map, and the views around the map live here.

pub mod areas;
pub mod areas_view;
pub mod camera;
pub mod editable;
pub mod home;
pub mod network;
pub mod network_view;
pub mod overlay;
pub mod source_quality;
pub mod source_view;
pub mod style;
pub mod svg;
#[cfg(test)]
mod tests;
pub mod view;
pub mod vm;

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::HtmlElement;
use wasm_bindgen::prelude::*;

use crate::shared::i18n::Resources;
use crate::shared::platform::browser_ports;

/// What the map page says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

/// The map page as the script sees it: what the pages around it ask of it.
#[wasm_bindgen]
pub struct MapPage(Rc<vm::MapVm>, Rc<crate::shared::i18n::I18n>);

#[wasm_bindgen]
impl MapPage {
    /// Binds the shell every page shares (settings menu, sidebars, notes tabs)
    /// to this page.
    pub fn mount_shell(&self) {
        crate::shell::mount(self.1.clone(), self.0.clone(), "details-areas", true);
    }

    /// The home page's city: its roads have been got (`prepare_city`), so the city behind the hero card and the
    /// line saying how it stands are drawn. Until then the page shows its search and no city, so the search
    /// is not kept waiting for the roads.
    pub fn mount_city(&self) {
        self.0.reopen();
        let vm = self.0.clone();
        let at =
            |id: &str| -> HtmlElement { leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into() };
        leptos::mount::mount_to(at("hero-map-slot"), {
            let vm = vm.clone();
            move || view! { <home::HeroMap vm=vm.clone()/> }.into_any()
        })
        .forget();
        leptos::mount::mount_to(at("hero-facts-slot"), move || view! { <home::HeroFacts vm=vm.clone()/> }.into_any()).forget();
    }
}

/// Draws the city map page into the elements it keeps for it, over the basemap the script made, and hands
/// back what the script needs to reach it.
#[wasm_bindgen]
pub fn mount_map(basemap: crate::shared::platform::Basemap) -> MapPage {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let ports = crate::shared::ports::Ports { mapper: Rc::new(crate::shared::platform::BrowserMapper::new(basemap)), ..browser_ports() };
    // One instance for the page: the shell switches it and the map's words follow.
    let i18n = crate::i18n_browser(&ports);
    let vm = vm::MapVm::new(ports, i18n.clone());
    vm.attach();
    let at =
        |id: &str| -> HtmlElement { leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into() };
    let mount = |id: &str, view: AnyView| {
        leptos::mount::mount_to(at(id), move || view).forget();
    };
    mount("search-slot", view! { <view::SearchBox vm=vm.clone()/> }.into_any());
    mount("map-tools-slot", view! { <view::MapTools vm=vm.clone()/> }.into_any());
    mount("map-slot", view! { <view::MapView vm=vm.clone()/> }.into_any());
    mount("plan-key", view! { <view::Legend vm=vm.clone()/> }.into_any());
    mount("fit-slot", view! { <view::Status vm=vm.clone()/> }.into_any());
    mount("checks-lead", view! { <view::ChecksLead vm=vm.clone()/> }.into_any());
    mount("checks", view! { <view::Checks vm=vm.clone()/> }.into_any());
    mount("changes-lead", view! { <view::ChangesLead vm=vm.clone()/> }.into_any());
    mount("changes", view! { <view::Changes vm=vm.clone()/> }.into_any());
    mount("areas-slot", view! { <areas_view::Areas vm=vm.clone()/> }.into_any());
    mount("network-lead", view! { <network_view::NetworkLead vm=vm.clone()/> }.into_any());
    mount("network-slot", view! { <network_view::NetworkChecks vm=vm.clone()/> }.into_any());
    mount("source-lead", view! { <source_view::SourceLead vm=vm.clone()/> }.into_any());
    mount("source-slot", view! { <source_view::SourceFindings vm=vm.clone()/> }.into_any());
    mount("title-block", view! { <view::TitleBlock vm=vm.clone()/><view::MapTitle vm=vm.clone()/> }.into_any());
    MapPage(vm, i18n)
}

/// Draws the home page's search box into the element it keeps for it, and hands back what the script needs to
/// reach the page. The city behind the hero card and how it stands come later, from `MapPage::mount_city`, once
/// the area's roads are got.
#[wasm_bindgen]
pub fn mount_home() -> MapPage {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let ports = browser_ports();
    // One instance for the page: the shell switches it, and the hero and the search follow.
    let i18n = crate::i18n_browser(&ports);
    let vm = vm::MapVm::new(ports, i18n.clone());
    let at =
        |id: &str| -> HtmlElement { leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into() };
    let mount = |id: &str, view: AnyView| {
        leptos::mount::mount_to(at(id), move || view).forget();
    };
    // The window title is the page's own and does not wait for the city; it renders nothing.
    mount(
        "search-slot",
        view! { <crate::place::view::AreaSearch vm=crate::place::vm::AreaVm::new(browser_ports(), i18n.clone())/><home::HomeTitle vm=vm.clone()/> }.into_any(),
    );
    MapPage(vm, i18n)
}
