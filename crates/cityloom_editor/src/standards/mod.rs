//! Project-specific design standards: a profile of local thresholds that the street page's checks read in place of
//! the built-in ones. The model (figures, validation, what is active), the store (the browser's profiles) and the
//! Profiles section live here; the street page reads only `model` and `store`.

pub mod model;
pub mod store;
#[cfg(test)]
mod tests;
pub mod text;
pub mod view;
pub mod vm;

use crate::shared::i18n::Resources;

/// What the standards profiles say, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::I18n;
use crate::shared::ports::Ports;
use crate::shared::units::Units;

/// Draws the Profiles section into `slot` (the street page's `#standards-slot`). `units` is read tracked, so that the
/// lengths follow the page's units; `on_change` is called when the profile in force, or what it holds, may have changed
/// (the page's checks are then drawn again).
pub fn mount_profiles(slot: leptos::web_sys::HtmlElement, ports: Ports, i18n: Rc<I18n>, units: Rc<dyn Fn() -> Units>, on_change: Rc<dyn Fn()>) {
    let vm = vm::StandardsVm::new(ports, i18n, units, on_change);
    leptos::mount::mount_to(slot, move || view! { <view::Profiles vm=vm.clone()/> }).forget();
}
