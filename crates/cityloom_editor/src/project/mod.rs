//! The person's projects: named containers, each tied to an area. The home page is their list.

pub mod context;
pub mod migrate;
pub mod model;
pub mod store;
pub mod text;
pub mod view;
pub mod vm;

#[cfg(test)]
mod tests;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::shared::i18n::Resources;
use crate::shared::platform::browser_ports;

pub use migrate::migrate_legacy;
pub use model::{Project, Refused};
pub use store::{DAMAGED_KEY, LAST_KEY, PROJECTS_KEY, ProjectStore, last_changed};

/// What the list of projects says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

/// The home page as the script sees it: the list of projects, and below it the About section with the city drawn
/// behind it (the map's own home views).
#[wasm_bindgen]
pub struct ProjectsPage {
    about: crate::map::MapPage,
}

#[wasm_bindgen]
impl ProjectsPage {
    /// Binds the shell every page shares (settings menu, sign-in menu, language) to this page.
    pub fn mount_shell(&self) {
        self.about.mount_shell();
    }

    /// The city behind the About section, once its roads are got (`prepare_city`).
    pub fn mount_city(&self) {
        self.about.mount_city();
    }
}

/// Draws the list of projects into `#projects-slot`, after moving the work kept from before projects into the first
/// project, then the About section's search; and hands back what the script needs to reach the page. The home page
/// works on the root storage: it is in no project.
#[wasm_bindgen]
pub fn mount_projects() -> ProjectsPage {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let ports = browser_ports();
    // One instance for the page: the shell switches it, and the list and the About section follow.
    let i18n = crate::i18n_browser(&ports);
    // First: the move of the old work decides which project the About section's city is.
    let projects = vm::ProjectsVm::new(ports, i18n.clone());
    let slot_id = "projects-slot";
    let slot = leptos::prelude::document().get_element_by_id(slot_id).unwrap_or_else(|| panic!("the page has no #{slot_id}"));
    let list = projects.clone();
    leptos::mount::mount_to(slot.unchecked_into(), move || view! { <view::ProjectsView vm=list/> }).forget();
    // Another tab that changes the list changes this one.
    let other = projects;
    let on_storage = Closure::<dyn FnMut(leptos::web_sys::StorageEvent)>::new(move |e: leptos::web_sys::StorageEvent| {
        if e.key().is_none_or(|k| k == PROJECTS_KEY || k == crate::sync::model::SYNC_KEY) {
            other.refresh();
        }
    });
    let _ = leptos::prelude::window().add_event_listener_with_callback("storage", on_storage.as_ref().unchecked_ref());
    on_storage.forget();
    ProjectsPage { about: crate::map::mount_home(i18n) }
}
