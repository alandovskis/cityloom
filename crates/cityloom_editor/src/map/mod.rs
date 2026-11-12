//! The city map page: the whole city drawn on a MapLibre map to pan,
//! zoom and choose a street or junction from. MapLibre draws the map and takes the
//! pointer (`web/mapcanvas.js`, reached through the `Mapper` port); the view-model,
//! the places it hands the map, and the views around the map live here.

pub mod areas;
pub mod areas_view;
pub mod camera;
pub mod corridor;
pub mod editable;
pub mod home;
pub mod network;
pub mod network_view;
pub mod overlay;
mod report;
mod report_review;
pub mod selection;
pub mod selection_view;
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

    /// Binds the Share menu's Download report item (`#report`, made by the shell's `mount_shell`) to the report's
    /// download. The shell's own listener on the same button closes the menu.
    pub fn bind_report(&self) {
        use wasm_bindgen::JsCast;
        let Some(button) = leptos::prelude::document().get_element_by_id("report") else { return };
        let vm = self.0.clone();
        let cb = Closure::<dyn FnMut()>::new(move || vm.download_report());
        let _ = button.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
        cb.forget();
    }

    /// The About page's city: its roads have been got (`prepare_city`), so the city behind the About card and the
    /// line saying how it stands are drawn. Until then the page shows its words and no city, so they are not kept
    /// waiting for the roads.
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

/// Draws the city map page of the page's project (`project::context`) into the elements it keeps for it, on the map
/// the script made, and hands back what the script needs to reach it. Nothing where the project is not here: the page
/// has then gone to the list.
#[wasm_bindgen]
pub fn mount_map(canvas: crate::shared::platform::MapCanvas) -> Option<MapPage> {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let ports = crate::project::context::open_page(browser_ports())?;
    let ports = crate::shared::ports::Ports { mapper: Rc::new(crate::shared::platform::BrowserMapper::new(canvas)), ..ports };
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
    mount("fit-slot", view! { <view::Status vm=vm.clone()/><view::ReportNotice vm=vm.clone()/> }.into_any());
    mount("checks-lead", view! { <view::ChecksLead vm=vm.clone()/> }.into_any());
    mount("checks", view! { <view::Checks vm=vm.clone()/> }.into_any());
    mount("changes-lead", view! { <view::ChangesLead vm=vm.clone()/> }.into_any());
    mount("changes", view! { <view::Changes vm=vm.clone()/> }.into_any());
    mount("selection-slot", view! { <selection_view::SelectedStreets vm=vm.clone()/> }.into_any());
    mount("areas-slot", view! { <areas_view::Areas vm=vm.clone()/> }.into_any());
    mount("network-lead", view! { <network_view::NetworkLead vm=vm.clone()/> }.into_any());
    mount("network-slot", view! { <network_view::NetworkChecks vm=vm.clone()/> }.into_any());
    mount("source-lead", view! { <source_view::SourceLead vm=vm.clone()/> }.into_any());
    mount("source-slot", view! { <source_view::SourceFindings vm=vm.clone()/> }.into_any());
    mount("title-block", view! { <view::TitleBlock vm=vm.clone()/><view::MapTitle vm=vm.clone()/> }.into_any());
    Some(MapPage(vm, i18n))
}

/// Draws the About page's window title and hands back what the page needs to reach it. The city behind the card and
/// how it stands come later, from `MapPage::mount_city`, once the area's roads are got. Called by the About page's
/// entry point (`lib.rs`, `mount_about`) with the page's one set of words, `i18n`.
pub fn mount_about(i18n: std::rc::Rc<crate::shared::i18n::I18n>) -> MapPage {
    // The About page shows the city of the project opened last (with none, the default area's, only read).
    let root = browser_ports();
    let ports = crate::project::context::peek(root.clone()).unwrap_or(root);
    let vm = vm::MapVm::new(ports, i18n.clone());
    let at =
        |id: &str| -> HtmlElement { leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into() };
    // The window title is the page's own and does not wait for the city; it renders nothing.
    let title = vm.clone();
    leptos::mount::mount_to(at("hero-facts-slot"), move || view! { <home::AboutTitle vm=title.clone()/> }.into_any()).forget();
    MapPage(vm, i18n)
}
