//! The city map page: the whole city drawn over an OpenStreetMap basemap to pan,
//! zoom and choose a street or junction from. MapLibre draws the map and takes the
//! pointer (`web/basemap.js`, reached through the `Mapper` port); the view-model,
//! the places it hands the map, and the views around the map live here.

pub mod camera;
pub mod home;
pub mod overlay;
pub mod projection;
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
        crate::shell::mount(self.1.clone(), self.0.clone(), "details-places", false);
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
    mount("street", view! { <view::MapHeader vm=vm.clone()/> }.into_any());
    mount("reset-slot", view! { <view::ResetButton vm=vm.clone()/> }.into_any());
    mount("map-tools-slot", view! { <view::MapTools vm=vm.clone()/> }.into_any());
    mount("map-slot", view! { <view::MapView vm=vm.clone()/> }.into_any());
    mount("plan-key", view! { <view::Legend vm=vm.clone()/> }.into_any());
    mount("fit-slot", view! { <view::Status vm=vm.clone()/> }.into_any());
    mount("inspector", view! { <view::Places vm=vm.clone()/> }.into_any());
    mount("checks-lead", view! { <view::ChecksLead vm=vm.clone()/> }.into_any());
    mount("checks", view! { <view::Checks vm=vm.clone()/> }.into_any());
    mount("changes-lead", view! { <view::ChangesLead vm=vm.clone()/> }.into_any());
    mount("changes", view! { <view::Changes vm=vm.clone()/> }.into_any());
    mount("title-block", view! { <view::TitleBlock vm=vm.clone()/> }.into_any());
    MapPage(vm, i18n)
}

/// Draws the home page into the elements it keeps for it: the city behind the
/// hero card, the search box, and how the city stands. Hands back what the script
/// needs to reach it.
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
    mount("hero-map-slot", view! { <home::HeroMap vm=vm.clone()/> }.into_any());
    mount("search-slot", view! { <crate::place::view::AreaSearch vm=crate::place::vm::AreaVm::new(browser_ports(), i18n.clone())/> }.into_any());
    mount("hero-facts-slot", view! { <home::HeroFacts vm=vm.clone()/><home::HomeTitle vm=vm.clone()/> }.into_any());
    MapPage(vm, i18n)
}
