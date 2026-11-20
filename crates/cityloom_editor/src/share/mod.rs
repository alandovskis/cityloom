//! Sharing a design by a link (#221, see `CLAUDE.md`): the share format, the page that makes a project's links and the
//! page that opens one.

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

/// What sharing says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

/// The element the page is drawn into (`web/share.html`).
const SLOT: &str = "sharing-slot";

fn slot() -> leptos::web_sys::HtmlElement {
    leptos::prelude::document().get_element_by_id(SLOT).unwrap_or_else(|| panic!("the page has no #{SLOT}")).unchecked_into()
}

/// Draws the sharing of the project the page is opened in into `#sharing-slot`. `ports` are the project's
/// (`project::context`). The page's entry point (`lib.rs`) puts it together.
pub fn mount_share(ports: Ports, i18n: Rc<I18n>, context: vm::Context) -> Rc<vm::ShareVm> {
    let share = vm::ShareVm::new(ports, i18n, context);
    share.load_account();
    let shown = share.clone();
    leptos::mount::mount_to(slot(), move || view! { <view::ShareView vm=shown/> }).forget();
    share
}

/// Draws the opening of the link the page's address carries into `#sharing-slot`, and starts it. `ports` are the root's:
/// the page is in no project until the copy is kept.
pub fn mount_open(ports: Ports, i18n: Rc<I18n>) -> Rc<vm::OpenVm> {
    let open = vm::OpenVm::new(ports, i18n);
    let shown = open.clone();
    leptos::mount::mount_to(slot(), move || view! { <view::OpenView vm=shown/> }).forget();
    open.start();
    open
}
