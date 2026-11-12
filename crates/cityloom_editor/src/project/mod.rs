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

use std::rc::Rc;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::shared::i18n::{I18n, Resources};
use crate::shared::ports::Ports;

pub use migrate::migrate_legacy;
pub use model::{Project, Refused};
pub use store::{DAMAGED_KEY, LAST_KEY, PROJECTS_KEY, ProjectStore, last_changed};

/// What the list of projects says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

/// Draws the list of projects into `#projects-slot` in the words of `i18n`, after moving the work kept from before
/// projects into the first project, and keeps it current when another tab changes the list. `ports` are the page's
/// root ports: the home page is in no project. The home page's entry point (`lib.rs`) calls this before anything of
/// the page reads a project, since the move decides which one that is.
pub fn mount_list(ports: Ports, i18n: Rc<I18n>) -> Rc<vm::ProjectsVm> {
    let projects = vm::ProjectsVm::new(ports, i18n);
    let slot_id = "projects-slot";
    let slot = leptos::prelude::document().get_element_by_id(slot_id).unwrap_or_else(|| panic!("the page has no #{slot_id}"));
    let list = projects.clone();
    leptos::mount::mount_to(slot.unchecked_into(), move || view! { <view::ProjectsView vm=list/> }).forget();
    let other = projects.clone();
    let on_storage = Closure::<dyn FnMut(leptos::web_sys::StorageEvent)>::new(move |e: leptos::web_sys::StorageEvent| {
        if e.key().is_none_or(|k| k == PROJECTS_KEY || k == crate::sync::model::SYNC_KEY) {
            other.refresh();
        }
    });
    let _ = leptos::prelude::window().add_event_listener_with_callback("storage", on_storage.as_ref().unchecked_ref());
    on_storage.forget();
    projects
}
