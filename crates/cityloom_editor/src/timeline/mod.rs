//! The design timeline: named milestones of a project's work (see `CLAUDE.md`).

pub mod model;
pub mod store;
pub mod text;
pub mod view;
pub mod vm;
pub mod work;

#[cfg(test)]
mod tests;

use std::rc::Rc;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::shared::i18n::{I18n, Resources};
use crate::shared::ports::Ports;

/// What the timeline says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

/// Draws the timeline into `#timeline-slot` in the words of `i18n`. `ports` are the project's (`project::context`):
/// its storage is the project's, so the milestones are too. The page's entry point (`lib.rs`) puts it together.
pub fn mount_timeline(ports: Ports, i18n: Rc<I18n>, context: vm::Context) -> Rc<vm::TimelineVm> {
    let timeline = vm::TimelineVm::new(ports, i18n, context);
    timeline.load_author();
    let slot_id = "timeline-slot";
    let slot = leptos::prelude::document().get_element_by_id(slot_id).unwrap_or_else(|| panic!("the page has no #{slot_id}"));
    let shown = timeline.clone();
    leptos::mount::mount_to(slot.unchecked_into(), move || view! { <view::TimelineView vm=shown/> }).forget();
    timeline
}
